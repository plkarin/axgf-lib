// SPDX-License-Identifier: Apache-2.0
//! # dedup — safe deduplication of persons and families
//!
//! Two passes, in order:
//!
//! 1. **Person merge.** Group persons by
//!    `(normalized_display_name, birth_year, death_year)` where all
//!    three components are known. Groups with two or more members are
//!    candidates. A candidate group is merged **only** when it is
//!    unambiguous. It is *ambiguous* — and the whole group is skipped
//!    with a [`crate::boundary::envelope::DiagnosticCode::ManualReviewRequired`]
//!    diagnostic — when any two persons in the group have an
//!    ancestor/descendant relationship (father/son homonym) or share
//!    a direct parent (same-name siblings / cousins). When the group
//!    is merged, references to the removed persons are rewritten to
//!    the keeper across every entity in the bundle.
//!
//! 2. **Family merge.** Group families by their sorted set of
//!    `union.persons[*].person_id`. Groups with two or more families
//!    are candidates. A candidate group is merged only when it is
//!    unambiguous: matching `union.type` and start-date years within
//!    ±1. Otherwise a `MANUAL_REVIEW_REQUIRED` diagnostic is emitted
//!    and the group is left alone. When merged: the lowest-UUID
//!    family survives, children and documents are unioned (deduped
//!    by their `_id`), and references to the removed families are
//!    rewritten.
//!
//! Doing person merge first is deliberate: after person merge, two
//! families that shared "the same couple" under different person IDs
//! will now share identical spouse sets and be caught by pass 2 as a
//! byproduct.
//!
//! **Never merged automatically:** father/son homonyms, same-name
//! siblings/cousins, any group whose members disagree on core dates
//! or union type. These get a `MANUAL_REVIEW_REQUIRED` diagnostic;
//! the caller decides.
//!
//! The `data` payload has the shape:
//! `{"bundle": <flat>, "merged_persons": u, "merged_families": u, "manual_review": u}`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::FlatBundle;
use crate::boundary::lifecycle::{
    check_manifest_version, compute_stats, now_iso8601_utc, parse_flat,
};

/// See [`crate::deduplicate`].
pub fn deduplicate(flat_json: &str) -> Envelope {
    let mut bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }

    let mut diags: Vec<Diagnostic> = Vec::new();
    let merged_persons = merge_persons(&mut bundle, &mut diags);
    let merged_families = merge_families(&mut bundle, &mut diags);
    refresh_manifest(&mut bundle);
    let manual_review = diags
        .iter()
        .filter(|d| d.code == DiagnosticCode::ManualReviewRequired)
        .count();

    let flat = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok_with(
        json!({
            "bundle":          flat,
            "merged_persons":  merged_persons,
            "merged_families": merged_families,
            "manual_review":   manual_review,
        }),
        diags,
    )
}

// -------------------------------------------------------------------------
// Person merge
// -------------------------------------------------------------------------

fn merge_persons(b: &mut FlatBundle, diags: &mut Vec<Diagnostic>) -> usize {
    // Bucket persons by (display, birth_y, death_y) triples that are
    // fully populated. Persons with any missing component are
    // unambiguous singletons — never merged.
    let mut buckets: BTreeMap<(String, i32, i32), Vec<String>> = BTreeMap::new();
    for (id, p) in &b.persons {
        let Some(display) = p
            .get("identity")
            .and_then(|i| i.get("name"))
            .and_then(|n| n.get("display"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let display = normalize_name(display);
        let Some(by) = person_year(p, "birth") else {
            continue;
        };
        let Some(dy) = person_year(p, "death") else {
            continue;
        };
        buckets
            .entry((display, by, dy))
            .or_default()
            .push(id.clone());
    }

    let parent_of = build_parent_of(b);
    let mut merged_count = 0;

    for ((display, by, dy), ids) in buckets {
        if ids.len() < 2 {
            continue;
        }
        // Ambiguity check: father/son homonym or same-name sibling/cousin.
        if is_ambiguous_person_group(&ids, &parent_of) {
            diags.push(Diagnostic {
                code: DiagnosticCode::ManualReviewRequired,
                severity: Severity::Warning,
                message: format!(
                    "persons {ids:?} share name {display:?} and dates {by}/{dy} but are in an \
                     ancestor or sibling relationship — manual review required, not merged"
                ),
                entity_ref: Some(format!("persons/{}", ids[0])),
            });
            continue;
        }
        let mut sorted = ids.clone();
        sorted.sort();
        let Some((keeper, victims)) = sorted.split_first() else {
            continue;
        };
        merge_person_records(b, keeper, victims);
        rewrite_refs(b, victims, keeper);
        for v in victims {
            b.persons.remove(v);
            merged_count += 1;
        }
    }
    merged_count
}

fn is_ambiguous_person_group(
    ids: &[String],
    parent_of: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    for i in 0..ids.len() {
        for j in i + 1..ids.len() {
            let a = &ids[i];
            let b = &ids[j];
            if is_ancestor(a, b, parent_of) || is_ancestor(b, a, parent_of) {
                return true;
            }
            if shares_direct_parent(a, b, parent_of) {
                return true;
            }
        }
    }
    false
}

/// True if `a` is an ancestor of `b` following parent→child edges.
fn is_ancestor(a: &str, b: &str, edges: &BTreeMap<String, BTreeSet<String>>) -> bool {
    let mut stack: Vec<String> = vec![a.to_string()];
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while let Some(cur) = stack.pop() {
        if !seen.insert(cur.clone()) {
            continue;
        }
        if let Some(children) = edges.get(&cur) {
            for c in children {
                if c == b {
                    return true;
                }
                stack.push(c.clone());
            }
        }
    }
    false
}

/// True if `a` and `b` are both children of at least one common
/// parent.
fn shares_direct_parent(a: &str, b: &str, edges: &BTreeMap<String, BTreeSet<String>>) -> bool {
    let parents_of = |x: &str| -> BTreeSet<String> {
        edges
            .iter()
            .filter_map(|(p, kids)| {
                if kids.contains(x) {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect()
    };
    let pa = parents_of(a);
    let pb = parents_of(b);
    !pa.is_disjoint(&pb)
}

/// Field-by-field merge: for every top-level key on a victim record
/// that the keeper does not already have set to a non-null value,
/// copy it over. `identity.names[]` is unioned. Everything else is
/// keep-if-empty.
fn merge_person_records(b: &mut FlatBundle, keeper: &str, victims: &[String]) {
    let victim_vals: Vec<Value> = victims
        .iter()
        .filter_map(|v| b.persons.get(v).cloned())
        .collect();
    let Some(k) = b.persons.get_mut(keeper) else {
        return;
    };
    let Some(k_obj) = k.as_object_mut() else {
        return;
    };
    for v in victim_vals {
        let Some(v_obj) = v.as_object() else {
            continue;
        };
        for (key, val) in v_obj {
            if key == "id" {
                continue;
            }
            if key == "identity" {
                merge_identity_names(k_obj.get_mut("identity"), val);
                continue;
            }
            match k_obj.get(key) {
                None | Some(Value::Null) => {
                    k_obj.insert(key.clone(), val.clone());
                }
                _ => {}
            }
        }
    }
}

fn merge_identity_names(keeper_identity: Option<&mut Value>, victim_identity: &Value) {
    let (Some(k), Some(v)) = (keeper_identity, victim_identity.as_object()) else {
        return;
    };
    let (Some(k_obj), Some(v_names)) =
        (k.as_object_mut(), v.get("names").and_then(Value::as_array))
    else {
        return;
    };
    let names_entry = k_obj
        .entry("names".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(k_names) = names_entry.as_array_mut() {
        for n in v_names {
            if !k_names.contains(n) {
                k_names.push(n.clone());
            }
        }
    }
}

// -------------------------------------------------------------------------
// Family merge
// -------------------------------------------------------------------------

fn merge_families(b: &mut FlatBundle, diags: &mut Vec<Diagnostic>) -> usize {
    let mut buckets: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for (fid, f) in &b.families {
        let mut sig: Vec<String> = parent_ids(f);
        if sig.is_empty() {
            continue;
        }
        sig.sort();
        sig.dedup();
        buckets.entry(sig).or_default().push(fid.clone());
    }

    let mut merged = 0;
    for (sig, fids) in buckets {
        if fids.len() < 2 {
            continue;
        }
        if is_ambiguous_family_group(&fids, b) {
            diags.push(Diagnostic {
                code: DiagnosticCode::ManualReviewRequired,
                severity: Severity::Warning,
                message: format!(
                    "families {fids:?} share spouse set {sig:?} but disagree on union.type or \
                     start.date year (> 1 year apart) — manual review required, not merged"
                ),
                entity_ref: Some(format!("families/{}", fids[0])),
            });
            continue;
        }
        let mut sorted = fids.clone();
        sorted.sort();
        let Some((keeper, victims)) = sorted.split_first() else {
            continue;
        };
        merge_family_records(b, keeper, victims);
        rewrite_refs(b, victims, keeper);
        for v in victims {
            b.families.remove(v);
            merged += 1;
        }
    }
    merged
}

fn is_ambiguous_family_group(fids: &[String], b: &FlatBundle) -> bool {
    let mut union_types: BTreeSet<String> = BTreeSet::new();
    let mut start_years: Vec<i32> = Vec::new();
    for fid in fids {
        let Some(f) = b.families.get(fid) else {
            continue;
        };
        if let Some(t) = f
            .get("union")
            .and_then(|u| u.get("type"))
            .and_then(Value::as_str)
        {
            union_types.insert(t.to_string());
        }
        if let Some(y) = f
            .get("union")
            .and_then(|u| u.get("start"))
            .and_then(|s| s.get("date"))
            .and_then(extract_year_from_date)
        {
            start_years.push(y);
        }
    }
    if union_types.len() > 1 {
        return true;
    }
    if let (Some(&min), Some(&max)) = (start_years.iter().min(), start_years.iter().max()) {
        if max - min > 1 {
            return true;
        }
    }
    false
}

fn merge_family_records(b: &mut FlatBundle, keeper: &str, victims: &[String]) {
    let victim_vals: Vec<Value> = victims
        .iter()
        .filter_map(|v| b.families.get(v).cloned())
        .collect();
    let Some(k) = b.families.get_mut(keeper) else {
        return;
    };
    let Some(k_obj) = k.as_object_mut() else {
        return;
    };
    for v in victim_vals {
        let Some(v_obj) = v.as_object() else {
            continue;
        };
        for (key, val) in v_obj {
            if key == "id" {
                continue;
            }
            if key == "children" {
                union_children(k_obj, val);
                continue;
            }
            if key == "documents" {
                union_docs(k_obj, val);
                continue;
            }
            match k_obj.get(key) {
                None | Some(Value::Null) => {
                    k_obj.insert(key.clone(), val.clone());
                }
                _ => {}
            }
        }
    }
    // Post-merge: dedupe union.persons on person_id.
    if let Some(u) = k_obj.get_mut("union").and_then(Value::as_object_mut) {
        if let Some(arr) = u.get_mut("persons").and_then(Value::as_array_mut) {
            let mut seen = BTreeSet::new();
            arr.retain(|entry| {
                let id = entry
                    .get("person_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() {
                    return true;
                }
                seen.insert(id)
            });
        }
    }
}

fn union_children(k_obj: &mut Map<String, Value>, victim_children: &Value) {
    let Some(v_arr) = victim_children.as_array() else {
        return;
    };
    let entry = k_obj
        .entry("children".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(k_arr) = entry.as_array_mut() else {
        return;
    };
    let existing: BTreeSet<String> = k_arr
        .iter()
        .filter_map(|c| c.get("person_id").and_then(Value::as_str).map(String::from))
        .collect();
    for c in v_arr {
        let id = c
            .get("person_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if !id.is_empty() && !existing.contains(&id) {
            k_arr.push(c.clone());
        }
    }
}

fn union_docs(k_obj: &mut Map<String, Value>, victim_docs: &Value) {
    let Some(v_arr) = victim_docs.as_array() else {
        return;
    };
    let entry = k_obj
        .entry("documents".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(k_arr) = entry.as_array_mut() else {
        return;
    };
    let existing: BTreeSet<String> = k_arr
        .iter()
        .filter_map(|d| {
            d.get("document_id")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .collect();
    for d in v_arr {
        let id = d
            .get("document_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if !id.is_empty() && !existing.contains(&id) {
            k_arr.push(d.clone());
        }
    }
}

// -------------------------------------------------------------------------
// Reference rewriting
// -------------------------------------------------------------------------

fn rewrite_refs(b: &mut FlatBundle, victims: &[String], keeper: &str) {
    let victim_set: BTreeSet<String> = victims.iter().cloned().collect();
    for map in [
        &mut b.persons,
        &mut b.families,
        &mut b.events,
        &mut b.links,
        &mut b.occupations,
        &mut b.sources,
        &mut b.places,
        &mut b.documents,
    ] {
        for value in map.values_mut() {
            rewrite_in_value(value, &victim_set, keeper);
        }
    }
}

fn rewrite_in_value(v: &mut Value, victims: &BTreeSet<String>, keeper: &str) {
    match v {
        Value::Object(m) => {
            for (k, val) in m.iter_mut() {
                if k != "id" && k.ends_with("_id") {
                    if let Some(s) = val.as_str() {
                        if victims.contains(s) {
                            *val = Value::String(keeper.to_string());
                        }
                    }
                }
                rewrite_in_value(val, victims, keeper);
            }
        }
        Value::Array(a) => {
            for item in a.iter_mut() {
                rewrite_in_value(item, victims, keeper);
            }
        }
        _ => {}
    }
}

// -------------------------------------------------------------------------
// Shared helpers
// -------------------------------------------------------------------------

fn refresh_manifest(b: &mut FlatBundle) {
    let stats = compute_stats(b);
    let now = now_iso8601_utc();
    if let Value::Object(ref mut m) = b.manifest {
        m.insert("stats".into(), stats);
        m.insert("updated_at".into(), Value::String(now));
    }
}

fn parent_ids(f: &Value) -> Vec<String> {
    f.get("union")
        .and_then(|u| u.get("persons"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.get("person_id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn build_parent_of(b: &FlatBundle) -> BTreeMap<String, BTreeSet<String>> {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for f in b.families.values() {
        let parents = parent_ids(f);
        let children: Vec<String> = f
            .get("children")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("person_id").and_then(Value::as_str).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        for p in &parents {
            for c in &children {
                edges.entry(p.clone()).or_default().insert(c.clone());
            }
        }
    }
    edges
}

fn person_year(p: &Value, key: &str) -> Option<i32> {
    extract_year_from_date(p.get(key)?.get("date")?)
}

fn extract_year_from_date(date: &Value) -> Option<i32> {
    let s = date.get("value")?.as_str()?;
    let year: String = s.chars().take_while(char::is_ascii_digit).take(4).collect();
    year.parse().ok()
}

fn normalize_name(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
