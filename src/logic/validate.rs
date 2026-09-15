// SPDX-License-Identifier: Apache-2.0
//! # validate — structural and semantic bundle validation
//!
//! `validate` is the library's read-only quality report. It never
//! mutates the bundle and always returns [`crate::boundary::envelope::Status::Ok`]
//! when it can parse the input and the spec version is supported. The
//! findings live in the envelope's `diagnostics`; the `data` payload
//! summarizes counts by severity so a caller can dispatch on a single
//! integer without walking the list.
//!
//! ## Layers
//!
//! **Structural (JSON Schema).** Every entity in the bundle — plus the
//! manifest — is validated against the embedded schema for the version the
//! manifest declares: `schema/axgf-1.0.schema.json` or, for a 1.1 bundle,
//! `schema/axgf-1.1.schema.json`. Failures surface as
//! `SCHEMA_VALIDATION_FAILED` warnings (severity `Warning`). The spec
//! says (§12.1) "conformant parsers SHOULD validate entities against
//! this schema" — SHOULD, not MUST, so a structurally-questionable
//! bundle is reported but not blocking. An enumeration failure inside 1.1
//! content is reported once, by the profile checks below, as
//! `OUT_OF_VOCABULARY` rather than twice.
//!
//! **AXGF 1.1.** `OUT_OF_VOCABULARY`, `CLAIM_INCONSISTENT`,
//! `SPEC_VERSION_MISMATCH` (warnings) and `UNKNOWN_ATTRIBUTE` (info): see
//! `logic/profile.rs`. They run on content, not on the declared version, so a
//! 1.1 attribute in a bundle that forgot to say 1.1 is still checked — and
//! the forgetting is reported.
//!
//! **Semantic.**
//!
//! - **Dangling references** (`DANGLING_REFERENCE`, warning) — every
//!   `*_id` field inside every entity must resolve to a UUID present
//!   in *some* collection of the bundle. The check is generic: any
//!   value keyed with a name ending in `_id` whose value looks like a
//!   UUID v4 is checked.
//! - **Parent/child cycles** (`CYCLE_DETECTED`, error) — the directed
//!   graph parent→child (built from every `Family`) must be a DAG. A
//!   person appearing as both parent and child of the same family, or
//!   an ancestor loop across multiple families, is flagged. Severity is
//!   `Error` even though the envelope status remains `Ok`: validation
//!   reports, it does not refuse.
//! - **Chronology conflicts** (`CHRONOLOGY_CONFLICT`, warning) — when
//!   both parent and child have a birth date parseable to a year, the
//!   child MUST NOT be born earlier than the parent. Comparison uses
//!   only the leading four-digit year, so partial dates like `1923`
//!   and `1923-04-12` work uniformly.
//! - **Duplicate spouse sets** (`DUPLICATE_UNIQUE_REF`, warning) — two
//!   different `Family` records whose `union.persons` reference the
//!   same set of persons are almost certainly duplicates and should
//!   be merged by [`crate::deduplicate`].
//!
//! The `data` payload has the shape:
//! `{"errors": u, "warnings": u, "infos": u, "total": u}`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use jsonschema::error::ValidationErrorKind;
use jsonschema::JSONSchema;
use serde_json::{json, Value};

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::FlatBundle;
use crate::boundary::lifecycle::{check_manifest_version, parse_flat, schema_for};
use crate::logic::crud::EntityKind;
use crate::logic::profile;

/// See [`crate::validate`].
pub fn validate(flat_json: &str) -> Envelope {
    let bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }

    let mut diags: Vec<Diagnostic> = Vec::new();
    let version = bundle
        .manifest
        .get("axgf")
        .and_then(Value::as_str)
        .unwrap_or("1.0")
        .to_string();

    // Structural: manifest + every entity.
    structural(&bundle, &version, &mut diags);

    // AXGF 1.1: vocabularies, the semantic rules of SPEC_1.1 §7.3, and the
    // versions entities declare.
    profile::check_manifest(&bundle.manifest, &mut diags);
    for (kind, _, map) in typed_collections(&bundle) {
        for (id, value) in map {
            profile::check_entity(kind, id, value, Some(&version), &mut diags);
        }
    }

    // Semantic passes.
    let all_ids = collect_all_ids(&bundle);
    dangling_refs(&bundle, &all_ids, &mut diags);
    cycles(&bundle, &mut diags);
    chronology(&bundle, &mut diags);
    duplicate_spouse_sets(&bundle, &mut diags);

    let (errors, warnings, infos) = counts(&diags);
    Envelope::ok_with(
        json!({
            "errors":   errors,
            "warnings": warnings,
            "infos":    infos,
            "total":    diags.len(),
        }),
        diags,
    )
}

// -------------------------------------------------------------------------
// Structural
// -------------------------------------------------------------------------

/// The schema that pins one entity kind (or `manifest`) for one spec
/// version, compiled once per process.
///
/// The embedded schemas are compile-time constants, so compiling one is a
/// pure function of its arguments; doing it on every `add_entity` cost more
/// than the add itself once the 1.1 schema's hundred vocabularies were in
/// it. Returns `None` for an unknown version, or if compilation fails — the
/// embedded schema being malformed would be a build-time bug, and a test
/// compiles every kind of both versions to rule it out — in which case
/// structural checks are skipped rather than propagating an internal error.
pub(crate) fn compiled(version: &str, kind: &str) -> Option<&'static JSONSchema> {
    static CACHE: OnceLock<HashMap<(&'static str, &'static str), JSONSchema>> = OnceLock::new();
    const KINDS: [&str; 9] = [
        "manifest",
        "person",
        "family",
        "event",
        "link",
        "occupation",
        "source",
        "place",
        "document",
    ];
    let cache = CACHE.get_or_init(|| {
        let mut out = HashMap::new();
        for v in crate::SUPPORTED_SPEC_VERSIONS {
            let Some((_, text)) = schema_for(v) else {
                continue;
            };
            let Ok(root) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            let Some(defs) = root.get("$defs") else {
                continue;
            };
            for k in KINDS {
                let wrapper = json!({ "$defs": defs, "$ref": format!("#/$defs/{k}") });
                if let Ok(schema) = JSONSchema::compile(&wrapper) {
                    out.insert((*v, k), schema);
                }
            }
        }
        out
    });
    let v = crate::SUPPORTED_SPEC_VERSIONS
        .iter()
        .find(|s| **s == version)?;
    let k = KINDS.iter().find(|s| **s == kind)?;
    cache.get(&(*v, *k))
}

/// Schema findings for one value, minus the enumeration failures inside 1.1
/// content that the profile checks report as `OUT_OF_VOCABULARY`.
pub(crate) fn schema_diagnostics(
    version: &str,
    kind_name: &str,
    kind: Option<EntityKind>,
    value: &Value,
    entity_ref: Option<String>,
) -> Vec<Diagnostic> {
    let Some(sch) = compiled(version, kind_name) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Err(errors) = sch.validate(value) {
        for e in errors {
            let is_enum = matches!(
                e.kind,
                ValidationErrorKind::Enum { .. } | ValidationErrorKind::PropertyNames { .. }
            );
            let path = e.instance_path.to_string();
            if profile::is_vocabulary_error(kind, &path, is_enum)
                || (kind.is_some() && profile::is_declaration_error(&path, value, version))
            {
                continue;
            }
            out.push(Diagnostic {
                code: DiagnosticCode::SchemaValidationFailed,
                severity: Severity::Warning,
                message: format!("{kind_name}: {e}"),
                entity_ref: entity_ref.clone(),
            });
        }
    }
    out
}

fn structural(b: &FlatBundle, version: &str, out: &mut Vec<Diagnostic>) {
    // Manifest — special: no entity id, no collection ref.
    out.extend(schema_diagnostics(
        version,
        "manifest",
        None,
        &b.manifest,
        None,
    ));
    // Entities — one compiled schema reused across all instances of a kind.
    for (kind, coll, map) in typed_collections(b) {
        for (id, value) in map {
            out.extend(schema_diagnostics(
                version,
                kind.singular(),
                Some(kind),
                value,
                Some(format!("{coll}/{id}")),
            ));
        }
    }
}

// -------------------------------------------------------------------------
// Semantic — dangling references
// -------------------------------------------------------------------------

fn collect_all_ids(b: &FlatBundle) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (_, _, map) in entity_collections(b) {
        for k in map.keys() {
            ids.insert(k.clone());
        }
    }
    ids
}

fn dangling_refs(b: &FlatBundle, known: &BTreeSet<String>, out: &mut Vec<Diagnostic>) {
    for (_kind, coll, map) in entity_collections(b) {
        for (id, value) in map {
            let mut refs = Vec::new();
            walk_ids(value, None, &mut refs);
            let mut seen_missing: BTreeSet<(String, String)> = BTreeSet::new();
            for (target, key) in refs {
                if !known.contains(&target) && seen_missing.insert((key.clone(), target.clone())) {
                    out.push(Diagnostic {
                        code: DiagnosticCode::DanglingReference,
                        severity: Severity::Warning,
                        message: format!(
                            "{coll}/{id}.{key} → {target} does not exist in this bundle"
                        ),
                        entity_ref: Some(format!("{coll}/{id}")),
                    });
                }
            }
        }
    }
}

/// Walk `value` looking for string leaves whose parent-object key ends
/// in `_id` and whose value is UUID-shaped. Skips the entity's own
/// `id` field at every level (an id is a definition, not a reference).
fn walk_ids(value: &Value, key: Option<&str>, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, sub) in map {
                if k == "id" {
                    continue;
                }
                walk_ids(sub, Some(k), out);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_ids(item, key, out);
            }
        }
        Value::String(s) => {
            if let Some(k) = key {
                if k.ends_with("_id") && is_uuid_like(s) {
                    out.push((s.clone(), k.to_string()));
                }
            }
        }
        _ => {}
    }
}

fn is_uuid_like(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

// -------------------------------------------------------------------------
// Semantic — parent/child cycles
// -------------------------------------------------------------------------

fn cycles(b: &FlatBundle, out: &mut Vec<Diagnostic>) {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (fid, f) in &b.families {
        let parents = parent_ids(f);
        let children = child_ids(f);
        for p in &parents {
            for c in &children {
                if p == c {
                    out.push(Diagnostic {
                        code: DiagnosticCode::CycleDetected,
                        severity: Severity::Error,
                        message: format!(
                            "person {p} appears as both parent and child in family {fid}"
                        ),
                        entity_ref: Some(format!("persons/{p}")),
                    });
                    continue;
                }
                edges.entry(p.clone()).or_default().insert(c.clone());
            }
        }
    }

    // Iterative 3-color DFS. Reports each back-edge once; deep ancestor
    // chains do not overflow the stack.
    #[derive(Copy, Clone, Eq, PartialEq)]
    enum Color {
        White,
        Gray,
        Black,
    }
    let mut color: BTreeMap<String, Color> = BTreeMap::new();
    let all_nodes: BTreeSet<String> = edges
        .keys()
        .cloned()
        .chain(edges.values().flat_map(|s| s.iter().cloned()))
        .collect();
    for n in &all_nodes {
        color.insert(n.clone(), Color::White);
    }
    let mut reported: BTreeSet<(String, String)> = BTreeSet::new();

    for start in &all_nodes {
        if color.get(start).copied().unwrap_or(Color::White) != Color::White {
            continue;
        }
        // Stack entries: (node, children snapshot, next-child index).
        let mut stack: Vec<(String, Vec<String>, usize)> = Vec::new();
        color.insert(start.clone(), Color::Gray);
        let init: Vec<String> = edges
            .get(start)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        stack.push((start.clone(), init, 0));

        while let Some((node, children, mut idx)) = stack.pop() {
            let mut descended = false;
            while idx < children.len() {
                let child = children[idx].clone();
                idx += 1;
                match color.get(&child).copied().unwrap_or(Color::White) {
                    Color::Gray => {
                        let key = (node.clone(), child.clone());
                        if reported.insert(key) {
                            out.push(Diagnostic {
                                code: DiagnosticCode::CycleDetected,
                                severity: Severity::Error,
                                message: format!(
                                    "parent/child cycle: {node} → {child} closes an ancestor loop"
                                ),
                                entity_ref: Some(format!("persons/{child}")),
                            });
                        }
                    }
                    Color::Black => {}
                    Color::White => {
                        color.insert(child.clone(), Color::Gray);
                        stack.push((node.clone(), children.clone(), idx));
                        let sub: Vec<String> = edges
                            .get(&child)
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .collect();
                        stack.push((child, sub, 0));
                        descended = true;
                        break;
                    }
                }
            }
            if !descended {
                color.insert(node, Color::Black);
            }
        }
    }
}

// -------------------------------------------------------------------------
// Semantic — chronology
// -------------------------------------------------------------------------

fn chronology(b: &FlatBundle, out: &mut Vec<Diagnostic>) {
    for (fid, f) in &b.families {
        let parents = parent_ids(f);
        let children = child_ids(f);
        for p in &parents {
            let Some(py) = b.persons.get(p).and_then(person_birth_year) else {
                continue;
            };
            for c in &children {
                let Some(cy) = b.persons.get(c).and_then(person_birth_year) else {
                    continue;
                };
                if cy < py {
                    out.push(Diagnostic {
                        code: DiagnosticCode::ChronologyConflict,
                        severity: Severity::Warning,
                        message: format!(
                            "child {c} born {cy} but parent {p} born {py} in family {fid}"
                        ),
                        entity_ref: Some(format!("persons/{c}")),
                    });
                }
            }
        }
    }
}

fn person_birth_year(p: &Value) -> Option<i32> {
    extract_year(p.get("birth")?.get("date")?)
}

fn extract_year(date: &Value) -> Option<i32> {
    let s = date.get("value")?.as_str()?;
    let year: String = s.chars().take_while(char::is_ascii_digit).take(4).collect();
    year.parse().ok()
}

// -------------------------------------------------------------------------
// Semantic — duplicate spouse sets
// -------------------------------------------------------------------------

fn duplicate_spouse_sets(b: &FlatBundle, out: &mut Vec<Diagnostic>) {
    let mut by_sig: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for (fid, f) in &b.families {
        let mut sig: Vec<String> = parent_ids(f);
        if sig.is_empty() {
            continue;
        }
        sig.sort();
        sig.dedup();
        by_sig.entry(sig).or_default().push(fid.clone());
    }
    for (sig, fids) in by_sig {
        if fids.len() > 1 {
            out.push(Diagnostic {
                code: DiagnosticCode::DuplicateUniqueRef,
                severity: Severity::Warning,
                message: format!(
                    "families {fids:?} share the same spouse set {sig:?} — consider merging"
                ),
                entity_ref: Some(format!("families/{}", fids[0])),
            });
        }
    }
}

// -------------------------------------------------------------------------
// Shared helpers
// -------------------------------------------------------------------------

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

fn child_ids(f: &Value) -> Vec<String> {
    f.get("children")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.get("person_id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// [`entity_collections`], with each collection's [`EntityKind`].
fn typed_collections(b: &FlatBundle) -> [(EntityKind, &'static str, &BTreeMap<String, Value>); 8] {
    [
        (EntityKind::Person, "persons", &b.persons),
        (EntityKind::Family, "families", &b.families),
        (EntityKind::Event, "events", &b.events),
        (EntityKind::Link, "links", &b.links),
        (EntityKind::Occupation, "occupations", &b.occupations),
        (EntityKind::Source, "sources", &b.sources),
        (EntityKind::Place, "places", &b.places),
        (EntityKind::Document, "documents", &b.documents),
    ]
}

/// Yield `(kind, collection, &map)` for every entity collection in the
/// bundle in a canonical order. `kind` is the singular schema name;
/// `collection` is the plural (matches the flat-bundle field name and
/// the on-disk directory).
fn entity_collections(
    b: &FlatBundle,
) -> [(&'static str, &'static str, &BTreeMap<String, Value>); 8] {
    [
        ("person", "persons", &b.persons),
        ("family", "families", &b.families),
        ("event", "events", &b.events),
        ("link", "links", &b.links),
        ("occupation", "occupations", &b.occupations),
        ("source", "sources", &b.sources),
        ("place", "places", &b.places),
        ("document", "documents", &b.documents),
    ]
}

fn counts(diags: &[Diagnostic]) -> (usize, usize, usize) {
    let mut e = 0;
    let mut w = 0;
    let mut i = 0;
    for d in diags {
        match d.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Info => i += 1,
        }
    }
    (e, w, i)
}
