// SPDX-License-Identifier: Apache-2.0
//! # merge — two records that are one, by plan and decision
//!
//! Someone always enters the same grandmother twice. [`plan_merge`] says what
//! merging two records of one kind would do, and changes nothing;
//! [`merge_entities`] does it, with every disagreement decided by the caller.
//! Kinds: person, family, place and source.
//!
//! ## Nothing is chosen silently, and nothing is lost
//!
//! For every field:
//!
//! - a value only one record holds is taken;
//! - values both hold alike are kept once, and two objects that do not
//!   contradict each other (one has a date, the other the same date and a
//!   place) are combined;
//! - lists — other names, titles, a profile attribute recorded as a series,
//!   documents, children, a place's names, a source's conflicts — are joined;
//! - values that differ are a **conflict**, which the caller resolves. Each
//!   conflict carries both values, the choices allowed and a suggestion.
//!
//! Where AXGF has room for the value not chosen, it is kept there, in
//! structures 1.0 and 1.1 already define:
//!
//! - a display name not chosen joins `identity.names` as an `alias`;
//! - a birth or a death not chosen becomes an **event** of that category,
//!   with the kept person as its `subject` and its own date, place, source
//!   and confidence; when both values cite a source, the chosen source gains
//!   a `conflicts` entry (`resolution: "this_preferred"`) — the
//!   specification's own record of two sources disagreeing;
//! - free text (notes, biography, a description) may keep both.
//!
//! Whatever is chosen, the report carries the absorbed record **verbatim**,
//! so a caller can keep it beside the archive (axgf-cms writes it into its
//! journal).
//!
//! ## Suggestions fail on the safe side
//!
//! - a visibility conflict suggests the more restrictive of the two;
//! - "living or not" suggests *living*, which protects;
//! - free text suggests keeping both;
//! - a birth or a death suggests the better attested value: the higher
//!   confidence, then the one that cites a source, then the kept record's.
//!
//! ## Merges that cannot be
//!
//! Refused outright, because the result would be impossible rather than
//! doubtful:
//!
//! - one person is the other's ancestor (a father and son of one name, the
//!   commonest homonym: the merged person would be their own parent);
//! - in one family, one is a spouse and the other a child;
//! - the two are the spouses of one union;
//! - two families of different couples, or a family whose child is a spouse
//!   in the other.
//!
//! Refused unless the caller confirms by **typing the absorbed person's
//! name**: two children of the same parents. In the registers of the 18th and
//! 19th centuries a child dies young and the next one is given the name; to
//! merge them is to erase a child. But the same child entered twice in one
//! family is real too — a GEDCOM import does it — and only a person can tell
//! the two apart.
//!
//! ## References
//!
//! Every reference equal to the absorbed id, in all eight kinds, becomes the
//! kept id: every `*_id` field, and a name component's `derived_from` (the
//! person a patronymic is derived from), which is a reference too. Then the duplicates this creates are tidied, and reported: a
//! family listing the person twice keeps one entry (the child with the higher
//! confidence), a link from the person to themselves is removed, an event
//! naming them twice keeps one participant. Families that now describe the
//! same couple are listed, not merged on the side: one merge is one decision.
//!
//! ## Two calls, and a fingerprint between them
//!
//! The plan carries a `fingerprint` of the two records and of every record
//! that refers to either. [`merge_entities`] recomputes the plan and refuses
//! with `STALE_PLAN`, changing nothing, when the fingerprint it is given no
//! longer matches — someone edited one of those records in between.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::FlatBundle;
use crate::boundary::lifecycle::{
    check_manifest_version, compute_stats, now_iso8601_utc, parse_flat, raise_manifest_version,
};
use crate::logic::crud::EntityKind;
use crate::logic::profile;
use crate::model::profile::registry::{self, Cardinality};

// -------------------------------------------------------------------------
// The boundary
// -------------------------------------------------------------------------

/// See [`crate::plan_merge`].
pub fn plan_merge(flat_json: &str, kind: EntityKind, keep_id: &str, absorb_id: &str) -> Envelope {
    let bundle = match open(flat_json, kind) {
        Ok(b) => b,
        Err(env) => return env,
    };
    match plan(&bundle, kind, keep_id, absorb_id) {
        Ok(p) => Envelope::ok(p.to_json()),
        Err(env) => env,
    }
}

/// See [`crate::merge_entities`].
pub fn merge_entities(
    flat_json: &str,
    kind: EntityKind,
    keep_id: &str,
    absorb_id: &str,
    resolutions_json: &str,
) -> Envelope {
    let mut bundle = match open(flat_json, kind) {
        Ok(b) => b,
        Err(env) => return env,
    };
    let resolutions: Value = match serde_json::from_str(resolutions_json) {
        Ok(v @ Value::Object(_)) => v,
        Ok(_) => {
            return Envelope::error(
                DiagnosticCode::InvalidJson,
                "resolutions must be a JSON object: {\"fingerprint\": …, \"choices\": {…}}",
            )
        }
        Err(e) => {
            return Envelope::error(
                DiagnosticCode::InvalidJson,
                format!("cannot parse resolutions JSON: {e}"),
            )
        }
    };
    let p = match plan(&bundle, kind, keep_id, absorb_id) {
        Ok(p) => p,
        Err(env) => return env,
    };

    // A plan made on other records is not this one.
    let given = resolutions.get("fingerprint").and_then(Value::as_str);
    if given != Some(p.fingerprint.as_str()) {
        return Envelope::error(
            DiagnosticCode::StalePlan,
            format!(
                "the plan's fingerprint {} does not match the bundle's {}: one of the records \
                 changed since the plan was made — plan again",
                given.unwrap_or("(none)"),
                p.fingerprint
            ),
        );
    }

    // Refusals: the impossible are refused whatever the caller says; two
    // siblings only with the absorbed person's name typed back.
    let confirm = resolutions.get("confirm").and_then(Value::as_str);
    let mut refused = Vec::new();
    for r in &p.refusals {
        let confirmed = r.confirm.as_deref().is_some_and(|c| confirm == Some(c));
        if !confirmed {
            refused.push(Diagnostic {
                code: DiagnosticCode::MergeRefused,
                severity: Severity::Error,
                message: match &r.confirm {
                    Some(phrase) => format!(
                        "{} — merging needs the confirmation {phrase:?}, typed",
                        r.message
                    ),
                    None => r.message.clone(),
                },
                entity_ref: Some(format!("{}/{absorb_id}", kind.collection())),
            });
        }
    }
    if !refused.is_empty() {
        return Envelope::error_many(refused);
    }

    // Every conflict needs a choice it allows.
    let choices = resolutions
        .get("choices")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut unresolved = Vec::new();
    let mut chosen: BTreeMap<String, &'static str> = BTreeMap::new();
    for c in &p.conflicts {
        match choices.get(&c.field).and_then(Value::as_str) {
            Some(choice) => match c.choices.iter().find(|allowed| **allowed == choice) {
                Some(allowed) => {
                    chosen.insert(c.field.clone(), allowed);
                }
                None => unresolved.push(Diagnostic {
                    code: DiagnosticCode::MergeUnresolved,
                    severity: Severity::Error,
                    message: format!("{}: {choice:?} is not one of {:?}", c.field, c.choices),
                    entity_ref: Some(format!("{}/{keep_id}", kind.collection())),
                }),
            },
            None => unresolved.push(Diagnostic {
                code: DiagnosticCode::MergeUnresolved,
                severity: Severity::Error,
                message: format!("{}: no choice given (one of {:?})", c.field, c.choices),
                entity_ref: Some(format!("{}/{keep_id}", kind.collection())),
            }),
        }
    }
    if !unresolved.is_empty() {
        return Envelope::error_many(unresolved);
    }
    let mut diags = Vec::new();
    for field in choices.keys() {
        if !p.conflicts.iter().any(|c| &c.field == field) {
            diags.push(Diagnostic {
                code: DiagnosticCode::MergeUnresolved,
                severity: Severity::Info,
                message: format!("{field}: not a conflict in this plan; the choice was ignored"),
                entity_ref: None,
            });
        }
    }

    let report = apply(&mut bundle, kind, &p, &chosen);
    refresh_manifest(&mut bundle);
    diags.extend(check(&bundle, kind, keep_id, &report));
    let flat = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok_with(json!({"bundle": flat, "report": report}), diags)
}

fn open(flat_json: &str, kind: EntityKind) -> Result<FlatBundle, Envelope> {
    let bundle = parse_flat(flat_json)?;
    check_manifest_version(&bundle.manifest)?;
    if !matches!(
        kind,
        EntityKind::Person | EntityKind::Family | EntityKind::Place | EntityKind::Source
    ) {
        return Err(Envelope::error(
            DiagnosticCode::MergeUnsupported,
            format!(
                "{} are not merged: in practice they are not entered twice, and a duplicate one \
                 is deleted. Persons, families, places and sources are merged.",
                kind.collection()
            ),
        ));
    }
    Ok(bundle)
}

// -------------------------------------------------------------------------
// The plan
// -------------------------------------------------------------------------

/// One field both records hold differently.
#[derive(Debug, Clone)]
struct Conflict {
    field: String,
    keep: Value,
    absorb: Value,
    choices: &'static [&'static str],
    suggestion: &'static str,
    because: &'static str,
    /// What becomes of the value not chosen, when AXGF has room for it.
    kept_as: Option<&'static str>,
}

/// One field the records do not disagree on, and where its value comes from.
#[derive(Debug, Clone)]
struct Agreement {
    field: String,
    from: &'static str,
}

#[derive(Debug, Clone)]
struct Refusal {
    reason: &'static str,
    message: String,
    /// When set, the refusal is lifted by typing this back.
    confirm: Option<String>,
}

#[derive(Debug, Clone)]
struct Plan {
    kind: EntityKind,
    keep: String,
    absorb: String,
    keep_record: Value,
    absorb_record: Value,
    /// The kept record with every agreement applied; a conflicted field still
    /// holds the kept record's value.
    merged: Value,
    agreements: Vec<Agreement>,
    conflicts: Vec<Conflict>,
    refusals: Vec<Refusal>,
    effects: Value,
    fingerprint: String,
}

const KEEP_ABSORB: &[&str] = &["keep", "absorb"];
const KEEP_ABSORB_BOTH: &[&str] = &["keep", "absorb", "both"];

impl Plan {
    fn to_json(&self) -> Value {
        json!({
            "kind": self.kind.singular(),
            "keep_id": self.keep,
            "absorb_id": self.absorb,
            "fingerprint": self.fingerprint,
            "agreements": self.agreements.iter().map(|a| json!({"field": a.field, "from": a.from})).collect::<Vec<_>>(),
            "conflicts": self.conflicts.iter().map(|c| {
                let mut o = json!({
                    "field": c.field,
                    "keep": c.keep,
                    "absorb": c.absorb,
                    "choices": c.choices,
                    "suggestion": c.suggestion,
                    "because": c.because,
                });
                if let Some(k) = c.kept_as {
                    o["not_chosen_becomes"] = json!(k);
                }
                o
            }).collect::<Vec<_>>(),
            "effects": self.effects,
            "refusals": self.refusals.iter().map(|r| {
                json!({"reason": r.reason, "message": r.message, "confirm": r.confirm})
            }).collect::<Vec<_>>(),
            "mergeable": self.refusals.iter().all(|r| r.confirm.is_some()),
        })
    }
}

fn plan(b: &FlatBundle, kind: EntityKind, keep: &str, absorb: &str) -> Result<Plan, Envelope> {
    let coll = collection(b, kind);
    let Some(keep_record) = coll.get(keep).cloned() else {
        return Err(Envelope::error(
            DiagnosticCode::EntityNotFound,
            format!("{} does not contain id {keep}", kind.collection()),
        ));
    };
    let Some(absorb_record) = coll.get(absorb).cloned() else {
        return Err(Envelope::error(
            DiagnosticCode::EntityNotFound,
            format!("{} does not contain id {absorb}", kind.collection()),
        ));
    };
    let mut refusals = Vec::new();
    if keep == absorb {
        refusals.push(Refusal {
            reason: "same_record",
            message: format!("{keep} is one record: there is nothing to merge"),
            confirm: None,
        });
    }
    match kind {
        EntityKind::Person => person_refusals(b, keep, absorb, &absorb_record, &mut refusals),
        EntityKind::Family => family_refusals(&keep_record, &absorb_record, &mut refusals),
        _ => {}
    }

    let mut walk = Walk {
        kind,
        agreements: Vec::new(),
        conflicts: Vec::new(),
    };
    let merged = match (keep_record.as_object(), absorb_record.as_object()) {
        (Some(k), Some(a)) => Value::Object(walk.object("", k, a)),
        _ => keep_record.clone(),
    };
    let effects = effects(b, kind, keep, absorb);
    let fingerprint = fingerprint(b, kind, keep, absorb);
    Ok(Plan {
        kind,
        keep: keep.to_string(),
        absorb: absorb.to_string(),
        keep_record,
        absorb_record,
        merged,
        agreements: walk.agreements,
        conflicts: walk.conflicts,
        refusals,
        effects,
        fingerprint,
    })
}

/// How a field is merged.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Rule {
    /// `id`, `type`, versions and timestamps: settled after the merge.
    Bookkeeping,
    /// An object whose fields have rules of their own.
    Descend,
    /// A list: joined, an item both hold kept once.
    Join,
    /// A list of objects keyed by one field: joined on it.
    JoinBy(&'static str),
    /// A place's names: joined on language and value, one primary per language.
    PlaceNames,
    /// One value: taken, combined, or a conflict.
    Unit,
    /// A birth or a death: a conflict's losing side becomes an event.
    Vital,
    /// The display name: a conflict's losing side joins the other names.
    DisplayName,
    /// A visibility: a conflict suggests the more restrictive.
    Restrictive,
    /// Living or not: a conflict suggests living.
    Protective,
    /// Free text: a conflict may keep both.
    Text,
}

fn rule(kind: EntityKind, path: &str) -> Rule {
    use Rule::*;
    match path {
        "id" | "type" | "axgf_version" | "created_at" | "updated_at" | "version_num"
        | "created_by" => return Bookkeeping,
        "tags" => return Join,
        _ => {}
    }
    match kind {
        EntityKind::Person => match path {
            "identity" | "ai" | "identity.class_visibility" => Descend,
            "identity.name" => DisplayName,
            "identity.names" | "ai.hypotheses" => Join,
            "identity.is_living" => Protective,
            "identity.visibility" => Restrictive,
            p if p.starts_with("identity.class_visibility.") => Restrictive,
            "birth" | "death" => Vital,
            "bio" | "notes" => Text,
            "documents" => JoinBy("document_id"),
            p if !p.contains('.') && registry::PROFILE_BLOCKS.contains(&p) => Descend,
            p => match registry::attribute(p) {
                Some(a) if a.cardinality == Cardinality::Series => Join,
                _ => Unit,
            },
        },
        EntityKind::Family => match path {
            "union" => Descend,
            "union.persons" | "children" => JoinBy("person_id"),
            "documents" => JoinBy("document_id"),
            "notes" | "description" => Text,
            _ => Unit,
        },
        EntityKind::Place => match path {
            "names" => PlaceNames,
            "country_history" => Join,
            "identifiers" => Descend,
            "note" => Text,
            _ => Unit,
        },
        EntityKind::Source => match path {
            "conflicts" => Join,
            "transcription" | "note" => Text,
            _ => Unit,
        },
        _ => Unit,
    }
}

struct Walk {
    kind: EntityKind,
    agreements: Vec<Agreement>,
    conflicts: Vec<Conflict>,
}

impl Walk {
    fn object(
        &mut self,
        path: &str,
        k: &Map<String, Value>,
        a: &Map<String, Value>,
    ) -> Map<String, Value> {
        let mut out = Map::new();
        let keys: Vec<&String> = k
            .keys()
            .chain(a.keys().filter(|key| !k.contains_key(*key)))
            .collect();
        for key in keys {
            let p = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            let (kv, av) = (k.get(key), a.get(key));
            let r = rule(self.kind, &p);
            if r == Rule::Bookkeeping {
                if let Some(v) = kv.or(av) {
                    out.insert(key.clone(), v.clone());
                }
                continue;
            }
            if let Some(v) = self.field(&p, r, kv, av) {
                out.insert(key.clone(), v);
            }
        }
        out
    }

    /// The merged value of one field, recording an agreement or a conflict.
    fn field(&mut self, p: &str, r: Rule, kv: Option<&Value>, av: Option<&Value>) -> Option<Value> {
        let (kr, ar) = (recorded(p, kv), recorded(p, av));
        let (k, a) = match (kr, ar) {
            (None, None) => return kv.or(av).cloned(),
            (Some(k), None) => {
                self.agree(p, "keep");
                return Some(k.clone());
            }
            (None, Some(a)) => {
                self.agree(p, "absorb");
                return Some(a.clone());
            }
            (Some(k), Some(a)) => (k, a),
        };
        if k == a {
            self.agree(p, "both");
            return Some(k.clone());
        }
        match r {
            Rule::Descend => match (k.as_object(), a.as_object()) {
                (Some(ko), Some(ao)) => Some(Value::Object(self.object(p, ko, ao))),
                _ => Some(self.unit(p, r, k, a)),
            },
            Rule::Join => match (k.as_array(), a.as_array()) {
                (Some(ka), Some(aa)) => {
                    let mut out = ka.clone();
                    for item in aa {
                        if !out.contains(item) {
                            out.push(item.clone());
                        }
                    }
                    self.agree(p, "joined");
                    Some(Value::Array(out))
                }
                _ => Some(self.unit(p, r, k, a)),
            },
            Rule::JoinBy(by) => match (k.as_array(), a.as_array()) {
                (Some(ka), Some(aa)) => {
                    self.agree(p, "joined");
                    Some(Value::Array(join_by(ka, aa, by)))
                }
                _ => Some(self.unit(p, r, k, a)),
            },
            Rule::PlaceNames => match (k.as_array(), a.as_array()) {
                (Some(ka), Some(aa)) => {
                    self.agree(p, "joined");
                    Some(Value::Array(place_names(ka, aa)))
                }
                _ => Some(self.unit(p, r, k, a)),
            },
            _ => Some(self.unit(p, r, k, a)),
        }
    }

    /// One value both records hold differently: combined when they do not
    /// contradict each other, a conflict when they do.
    fn unit(&mut self, p: &str, r: Rule, k: &Value, a: &Value) -> Value {
        if let Some(both) = compatible(k, a) {
            self.agree(p, "combined");
            return both;
        }
        let (choices, suggestion, because, kept_as) = match r {
            Rule::Restrictive => {
                let side = if rank(a) > rank(k) { "absorb" } else { "keep" };
                (KEEP_ABSORB, side, "more_restrictive", None)
            }
            Rule::Protective => {
                let side = if a == &Value::Bool(true) {
                    "absorb"
                } else {
                    "keep"
                };
                (KEEP_ABSORB, side, "protective", None)
            }
            Rule::Text => (KEEP_ABSORB_BOTH, "both", "both_kept", None),
            Rule::Vital => {
                let (side, because) = better_attested(k, a);
                (KEEP_ABSORB, side, because, Some("event"))
            }
            Rule::DisplayName => (KEEP_ABSORB, "keep", "kept_record", Some("alias")),
            _ => {
                let (side, because) = better_attested(k, a);
                (KEEP_ABSORB, side, because, None)
            }
        };
        self.conflicts.push(Conflict {
            field: p.to_string(),
            keep: k.clone(),
            absorb: a.clone(),
            choices,
            suggestion,
            because,
            kept_as,
        });
        k.clone()
    }

    fn agree(&mut self, p: &str, from: &'static str) {
        // A field inside one that descends is reported at its own path; the
        // parent is not reported again.
        self.agreements.push(Agreement {
            field: p.to_string(),
            from,
        });
    }
}

/// The value at `p`, unless it says nothing: absent, `null`, empty, the
/// schema's `unknown`, or a gender of `U`.
fn recorded<'a>(p: &str, v: Option<&'a Value>) -> Option<&'a Value> {
    let v = v?;
    let empty = match v {
        Value::Null => true,
        Value::String(s) => s.is_empty() || s == "unknown",
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => {
            o.is_empty()
                || (p == "identity.gender"
                    && o.get("value").and_then(Value::as_str) == Some("U")
                    && o.get("note").is_none_or(Value::is_null))
        }
        _ => false,
    };
    (!empty).then_some(v)
}

/// The two values combined, when neither contradicts the other: every field
/// both hold is equal (or combines, one level down), and every other field
/// is held by one of them. `None` when they contradict each other.
fn compatible(k: &Value, a: &Value) -> Option<Value> {
    if k == a {
        return Some(k.clone());
    }
    match (k, a) {
        (Value::Object(ko), Value::Object(ao)) => {
            let mut out = ko.clone();
            for (key, av) in ao {
                match ko.get(key) {
                    None => {
                        out.insert(key.clone(), av.clone());
                    }
                    Some(kv) => {
                        let (kr, ar) = (recorded("", Some(kv)), recorded("", Some(av)));
                        match (kr, ar) {
                            (_, None) => {}
                            (None, Some(av)) => {
                                out.insert(key.clone(), av.clone());
                            }
                            (Some(kv), Some(av)) => {
                                out.insert(key.clone(), compatible(kv, av)?);
                            }
                        }
                    }
                }
            }
            Some(Value::Object(out))
        }
        _ => None,
    }
}

/// Which side of a conflict is better attested, and why: the higher
/// confidence, then the one that cites a source, then the kept record's.
fn better_attested(k: &Value, a: &Value) -> (&'static str, &'static str) {
    let conf = |v: &Value| {
        v.get("confidence")
            .and_then(Value::as_f64)
            .or_else(|| v.pointer("/date/confidence").and_then(Value::as_f64))
    };
    if let (Some(kc), Some(ac)) = (conf(k), conf(a)) {
        if ac > kc {
            return ("absorb", "higher_confidence");
        }
        if kc > ac {
            return ("keep", "higher_confidence");
        }
    }
    let sourced = |v: &Value| v.get("source_id").and_then(Value::as_str).is_some();
    if sourced(a) && !sourced(k) {
        return ("absorb", "sourced");
    }
    if sourced(k) && !sourced(a) {
        return ("keep", "sourced");
    }
    ("keep", "kept_record")
}

/// A visibility's place on the scale, the most open first.
fn rank(v: &Value) -> usize {
    match v.as_str() {
        Some("public") => 0,
        Some("members") => 1,
        Some("contributors") => 2,
        Some("private") => 3,
        _ => 0,
    }
}

/// Two lists of objects joined on `by`. An entry both hold is kept once:
/// combined when the two do not contradict each other, otherwise the one
/// with the higher confidence (the kept record's on a tie).
fn join_by(k: &[Value], a: &[Value], by: &str) -> Vec<Value> {
    let mut out: Vec<Value> = k.to_vec();
    for item in a {
        let key = item.get(by).and_then(Value::as_str);
        let at = key.and_then(|key| {
            out.iter()
                .position(|o| o.get(by).and_then(Value::as_str) == Some(key))
        });
        match at {
            None => out.push(item.clone()),
            Some(i) => {
                if let Some(both) = compatible(&out[i], item) {
                    out[i] = both;
                } else if confidence(item) > confidence(&out[i]) {
                    out[i] = item.clone();
                }
            }
        }
    }
    out
}

fn confidence(v: &Value) -> f64 {
    v.get("confidence").and_then(Value::as_f64).unwrap_or(-1.0)
}

/// A place's names joined on language and value; a name that would be a
/// second primary in its language joins as a secondary one.
fn place_names(k: &[Value], a: &[Value]) -> Vec<Value> {
    let key = |n: &Value| {
        (
            n.get("lang")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            n.get("value")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        )
    };
    let mut out: Vec<Value> = k.to_vec();
    for n in a {
        let (lang, value) = key(n);
        if out.iter().any(|o| key(o) == (lang.clone(), value.clone())) {
            continue;
        }
        let mut n = n.clone();
        let primary_taken = out
            .iter()
            .any(|o| o.get("is_primary") == Some(&Value::Bool(true)) && key(o).0 == lang);
        if primary_taken && n.get("is_primary") == Some(&Value::Bool(true)) {
            n["is_primary"] = Value::Bool(false);
        }
        out.push(n);
    }
    out
}

// -------------------------------------------------------------------------
// Refusals
// -------------------------------------------------------------------------

fn person_refusals(
    b: &FlatBundle,
    keep: &str,
    absorb: &str,
    absorb_record: &Value,
    out: &mut Vec<Refusal>,
) {
    let edges = parent_of(b);
    if is_ancestor(keep, absorb, &edges) || is_ancestor(absorb, keep, &edges) {
        out.push(Refusal {
            reason: "ancestor",
            message: format!(
                "{keep} and {absorb} are ancestor and descendant: merged, the person would be \
                 their own forebear"
            ),
            confirm: None,
        });
    }
    for (fid, f) in &b.families {
        let spouses = spouse_ids(f);
        let children = child_ids(f);
        let (ks, as_) = (spouses.contains(keep), spouses.contains(absorb));
        let (kc, ac) = (children.contains(keep), children.contains(absorb));
        if (ks && ac) || (as_ && kc) {
            out.push(Refusal {
                reason: "spouse_and_child",
                message: format!(
                    "in family {fid} one is a spouse and the other a child: merged, the person \
                     would be their own parent"
                ),
                confirm: None,
            });
        }
        if ks && as_ && keep != absorb {
            out.push(Refusal {
                reason: "spouses",
                message: format!(
                    "{keep} and {absorb} are the two spouses of family {fid}: merged, the person \
                     would be married to themselves"
                ),
                confirm: None,
            });
        }
    }
    if keep != absorb && shares_parent(keep, absorb, &edges) {
        let name = absorb_record
            .pointer("/identity/name/display")
            .and_then(Value::as_str)
            .unwrap_or(absorb)
            .to_string();
        out.push(Refusal {
            reason: "siblings",
            message: format!(
                "{keep} and {absorb} are recorded as children of the same parents. Two children \
                 are often given one name — the first died young and the next took it — and \
                 merging them erases a child. If this is one child entered twice, confirm by \
                 typing the name of the record that goes"
            ),
            confirm: Some(name),
        });
    }
}

fn family_refusals(k: &Value, a: &Value, out: &mut Vec<Refusal>) {
    let (ks, as_) = (spouse_ids(k), spouse_ids(a));
    // One family may know only the father and the other only the mother:
    // that is one couple. Three spouses or more are two couples, unless a
    // union says it is polygamous.
    let polygamous = [k, a]
        .iter()
        .any(|f| f.pointer("/union/type").and_then(Value::as_str) == Some("polygamous"));
    if ks.union(&as_).count() > 2 && !polygamous {
        out.push(Refusal {
            reason: "different_couples",
            message: format!(
                "the two families record different couples ({ks:?} and {as_:?}): merged, one \
                 union would hold two marriages"
            ),
            confirm: None,
        });
    }
    let (kc, ac) = (child_ids(k), child_ids(a));
    if !ks.is_disjoint(&ac) || !as_.is_disjoint(&kc) {
        out.push(Refusal {
            reason: "child_is_spouse",
            message: "a child of one family is a spouse in the other: merged, a person would be \
                      their own parent"
                .to_string(),
            confirm: None,
        });
    }
}

fn spouse_ids(f: &Value) -> BTreeSet<String> {
    ids_in(f.pointer("/union/persons"), "person_id")
}

fn child_ids(f: &Value) -> BTreeSet<String> {
    ids_in(f.get("children"), "person_id")
}

fn ids_in(list: Option<&Value>, key: &str) -> BTreeSet<String> {
    list.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| e.get(key).and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn parent_of(b: &FlatBundle) -> BTreeMap<String, BTreeSet<String>> {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for f in b.families.values() {
        let children = child_ids(f);
        for p in spouse_ids(f) {
            edges.entry(p).or_default().extend(children.iter().cloned());
        }
    }
    edges
}

fn is_ancestor(a: &str, b: &str, edges: &BTreeMap<String, BTreeSet<String>>) -> bool {
    let mut stack = vec![a.to_string()];
    let mut seen = BTreeSet::new();
    while let Some(cur) = stack.pop() {
        if !seen.insert(cur.clone()) {
            continue;
        }
        for c in edges.get(&cur).into_iter().flatten() {
            if c == b {
                return true;
            }
            stack.push(c.clone());
        }
    }
    false
}

fn shares_parent(a: &str, b: &str, edges: &BTreeMap<String, BTreeSet<String>>) -> bool {
    edges
        .values()
        .any(|kids| kids.contains(a) && kids.contains(b))
}

// -------------------------------------------------------------------------
// Effects, and the fingerprint
// -------------------------------------------------------------------------

/// What the merge does beyond the two records: references rewritten per
/// kind, links that would point from a record to itself, entries that would
/// be listed twice, and families that would describe the same couple.
fn effects(b: &FlatBundle, kind: EntityKind, keep: &str, absorb: &str) -> Value {
    let mut references = Map::new();
    for (name, map) in collections(b) {
        let mut n = 0usize;
        for (id, v) in map {
            if name == kind.collection() && id == absorb {
                continue;
            }
            n += count_refs(v, absorb);
        }
        if n > 0 {
            references.insert(name.to_string(), json!(n));
        }
    }
    let same = |id: Option<&str>| id == Some(keep) || id == Some(absorb);
    let self_links: Vec<&String> = b
        .links
        .iter()
        .filter(|(_, l)| {
            same(l.pointer("/from/entity_id").and_then(Value::as_str))
                && same(l.pointer("/to/entity_id").and_then(Value::as_str))
                && l.pointer("/from/entity_type") == l.pointer("/to/entity_type")
        })
        .map(|(id, _)| id)
        .collect();
    let mut listed_twice = Vec::new();
    for (fid, f) in &b.families {
        let (s, c) = (spouse_ids(f), child_ids(f));
        if s.contains(keep) && s.contains(absorb) || c.contains(keep) && c.contains(absorb) {
            listed_twice.push(json!({"collection": "families", "id": fid}));
        }
    }
    for (eid, e) in &b.events {
        let ids = ids_in(e.get("participants"), "entity_id");
        if ids.contains(keep) && ids.contains(absorb) {
            listed_twice.push(json!({"collection": "events", "id": eid}));
        }
    }
    let mut same_couple = Vec::new();
    if kind == EntityKind::Person {
        let couple = |f: &Value| -> BTreeSet<String> {
            spouse_ids(f)
                .into_iter()
                .map(|p| if p == absorb { keep.to_string() } else { p })
                .collect()
        };
        let touched: Vec<(&String, BTreeSet<String>)> = b
            .families
            .iter()
            .filter(|(_, f)| {
                let s = spouse_ids(f);
                s.contains(keep) || s.contains(absorb)
            })
            .map(|(id, f)| (id, couple(f)))
            .collect();
        for i in 0..touched.len() {
            for j in i + 1..touched.len() {
                if touched[i].1.len() > 1 && touched[i].1 == touched[j].1 {
                    same_couple.push(json!([touched[i].0, touched[j].0]));
                }
            }
        }
    }
    json!({
        "references": references,
        "self_links_removed": self_links,
        "listed_twice": listed_twice,
        "same_couple_families": same_couple,
    })
}

/// Whether a key holds a reference: every `*_id` but the record's own `id`,
/// and `derived_from`, the one reference the schema does not name `*_id`.
pub(crate) fn is_reference(key: &str) -> bool {
    (key != "id" && key.ends_with("_id")) || key == "derived_from"
}

fn count_refs(v: &Value, target: &str) -> usize {
    match v {
        Value::Object(m) => m
            .iter()
            .map(|(k, val)| {
                let here = usize::from(is_reference(k) && val.as_str() == Some(target));
                here + count_refs(val, target)
            })
            .sum(),
        Value::Array(a) => a.iter().map(|x| count_refs(x, target)).sum(),
        _ => 0,
    }
}

fn mentions(v: &Value, a: &str, b: &str) -> bool {
    count_refs(v, a) > 0 || count_refs(v, b) > 0
}

/// A hash of the two records and of every record that refers to either,
/// in a canonical form (keys sorted), so that a caller who round-trips the
/// bundle through its own representation still matches.
fn fingerprint(b: &FlatBundle, kind: EntityKind, keep: &str, absorb: &str) -> String {
    let mut text = String::new();
    text.push_str(kind.collection());
    text.push('\n');
    let coll = collection(b, kind);
    for id in [keep, absorb] {
        text.push_str(id);
        text.push('\n');
        if let Some(v) = coll.get(id) {
            canonical(v, &mut text);
        }
        text.push('\n');
    }
    for (name, map) in collections(b) {
        for (id, v) in map {
            if name == kind.collection() && (id == keep || id == absorb) {
                continue;
            }
            if mentions(v, keep, absorb) {
                text.push_str(name);
                text.push('/');
                text.push_str(id);
                text.push('\n');
                canonical(v, &mut text);
                text.push('\n');
            }
        }
    }
    // FNV-1a, 64 bits, twice with two offsets: a check against edits made in
    // between, not a defence against an adversary.
    let fnv = |offset: u64| {
        text.bytes().fold(offset, |h, byte| {
            (h ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    };
    format!(
        "{:016x}{:016x}",
        fnv(0xcbf2_9ce4_8422_2325),
        fnv(0x6c62_272e_07bb_0142)
    )
}

fn canonical(v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            out.push('{');
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                canonical(&m[k], out);
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical(x, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

// -------------------------------------------------------------------------
// Applying a plan
// -------------------------------------------------------------------------

fn apply(
    b: &mut FlatBundle,
    kind: EntityKind,
    p: &Plan,
    chosen: &BTreeMap<String, &'static str>,
) -> Value {
    let now = now_iso8601_utc();
    let mut merged = p.merged.clone();
    let mut applied = Vec::new();
    let mut events_created = Vec::new();
    let mut source_conflicts = Vec::new();
    let mut vitals_lost: Vec<(String, Value, Value)> = Vec::new();

    for c in &p.conflicts {
        let choice = chosen.get(&c.field).copied().unwrap_or("keep");
        applied.push(json!({"field": c.field, "choice": choice}));
        let value = match choice {
            "absorb" => c.absorb.clone(),
            "both" => both_texts(
                &c.keep,
                &c.absorb,
                &label(kind, &p.absorb_record, &p.absorb),
            ),
            _ => c.keep.clone(),
        };
        let loser = if choice == "absorb" {
            &c.keep
        } else {
            &c.absorb
        };
        set_path(&mut merged, &c.field, value.clone());
        match c.kept_as {
            Some("alias") if choice != "both" => add_alias(&mut merged, loser),
            Some("event") if choice != "both" => {
                vitals_lost.push((c.field.clone(), value, loser.clone()))
            }
            _ => {}
        }
    }

    // A birth or a death not chosen: an event of its own, unless it was one
    // already; and the chosen source records that another disagreed.
    for (field, kept, lost) in &vitals_lost {
        if lost.get("event_id").and_then(Value::as_str).is_none() {
            let id = Uuid::new_v4().to_string();
            let mut e = json!({
                "id": id,
                "type": "event",
                "axgf_version": "1.0",
                "version_num": 1,
                "created_at": now,
                "updated_at": now,
                "category": field,
                "date": lost.get("date").cloned().unwrap_or_else(|| json!({})),
                "participants": [{"entity_type": "person", "entity_id": p.keep, "role": "subject"}],
            });
            for key in ["place_id", "source_id", "confidence"] {
                if let Some(v) = lost.get(key).filter(|v| !v.is_null()) {
                    e[key] = v.clone();
                }
            }
            if let Some(cause) = lost.get("cause").and_then(Value::as_str) {
                e["description"] = json!(cause);
            }
            b.events.insert(id.clone(), e);
            events_created.push(json!({"id": id, "category": field}));
        }
        let (ks, ls) = (
            kept.get("source_id").and_then(Value::as_str),
            lost.get("source_id").and_then(Value::as_str),
        );
        if let (Some(ks), Some(ls)) = (ks, ls) {
            if ks != ls {
                for (sub, this, other) in [
                    ("date", kept.get("date"), lost.get("date")),
                    ("place_id", kept.get("place_id"), lost.get("place_id")),
                ] {
                    if this == other {
                        continue;
                    }
                    let as_text = |v: Option<&Value>| match v {
                        Some(Value::String(s)) => s.clone(),
                        Some(d) => d
                            .get("value")
                            .and_then(Value::as_str)
                            .map(String::from)
                            .unwrap_or_else(|| d.to_string()),
                        None => String::new(),
                    };
                    let entry = json!({
                        "source_id": ls,
                        "field": format!("{field}.{sub}"),
                        "this_value": as_text(this),
                        "conflict_value": as_text(other),
                        "resolution": "this_preferred",
                    });
                    if let Some(src) = b.sources.get_mut(ks) {
                        let list = src
                            .as_object_mut()
                            .map(|o| o.entry("conflicts").or_insert_with(|| json!([])));
                        if let Some(Value::Array(list)) = list {
                            if !list.contains(&entry) {
                                list.push(entry);
                                source_conflicts.push(
                                    json!({"source_id": ks, "field": format!("{field}.{sub}")}),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Bookkeeping: the kept id, the newest version either declares or the
    // content needs, the earliest creation, one more version.
    if let Some(m) = merged.as_object_mut() {
        m.insert("id".into(), json!(p.keep));
        let declared = [&p.keep_record, &p.absorb_record]
            .iter()
            .filter_map(|r| r.get("axgf_version").and_then(Value::as_str))
            .filter_map(|v| profile::version_rank(v).map(|r| (r, v.to_string())))
            .max();
        let needed = profile::required_version(kind, &Value::Object(m.clone()));
        let version = match declared {
            Some((r, v)) if profile::version_rank(needed).is_none_or(|n| r >= n) => v,
            _ => needed.to_string(),
        };
        m.insert("axgf_version".into(), json!(version));
        let created = [&p.keep_record, &p.absorb_record]
            .iter()
            .filter_map(|r| r.get("created_at").and_then(Value::as_str))
            .min()
            .map(String::from);
        if let Some(c) = created {
            m.insert("created_at".into(), json!(c));
        }
        m.insert("updated_at".into(), json!(now));
        let version_num = [&p.keep_record, &p.absorb_record]
            .iter()
            .filter_map(|r| r.get("version_num").and_then(Value::as_u64))
            .max()
            .unwrap_or(1);
        m.insert("version_num".into(), json!(version_num + 1));
    }

    let coll = collection_mut(b, kind);
    coll.insert(p.keep.clone(), merged);
    coll.remove(&p.absorb);

    // Every reference to the absorbed record, now to the kept one.
    let mut rewritten = Map::new();
    for (name, map) in collections_mut(b) {
        let mut n = 0usize;
        for v in map.values_mut() {
            n += rewrite(v, &p.absorb, &p.keep);
        }
        if n > 0 {
            rewritten.insert(name.to_string(), json!(n));
        }
    }

    // The duplicates that makes.
    let mut tidied = Vec::new();
    for (fid, f) in b.families.iter_mut() {
        if dedupe_list(f.pointer_mut("/union/persons"), "person_id", &p.keep) {
            tidied.push(json!({"collection": "families", "id": fid, "field": "union.persons"}));
        }
        if dedupe_list(f.get_mut("children"), "person_id", &p.keep) {
            tidied.push(json!({"collection": "families", "id": fid, "field": "children"}));
        }
    }
    for (eid, e) in b.events.iter_mut() {
        if dedupe_list(e.get_mut("participants"), "entity_id", &p.keep) {
            tidied.push(json!({"collection": "events", "id": eid, "field": "participants"}));
        }
    }
    if kind == EntityKind::Source {
        if let Some(Value::Array(list)) = b
            .sources
            .get_mut(&p.keep)
            .and_then(|s| s.get_mut("conflicts"))
        {
            let before = list.len();
            list.retain(|c| c.get("source_id").and_then(Value::as_str) != Some(p.keep.as_str()));
            if list.len() != before {
                tidied.push(json!({"collection": "sources", "id": p.keep, "field": "conflicts"}));
            }
        }
    }
    let self_links: Vec<String> = b
        .links
        .iter()
        .filter(|(_, l)| {
            l.pointer("/from/entity_id") == l.pointer("/to/entity_id")
                && l.pointer("/from/entity_type") == l.pointer("/to/entity_type")
                && l.pointer("/from/entity_id").and_then(Value::as_str) == Some(p.keep.as_str())
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in &self_links {
        b.links.remove(id);
    }

    json!({
        "kind": kind.singular(),
        "kept_id": p.keep,
        "absorbed_id": p.absorb,
        "absorbed_record": p.absorb_record,
        "resolutions": applied,
        "references_rewritten": rewritten,
        "events_created": events_created,
        "source_conflicts_added": source_conflicts,
        "links_removed": self_links,
        "entries_tidied": tidied,
        "same_couple_families": p.effects.get("same_couple_families").cloned().unwrap_or(json!([])),
    })
}

/// Two texts kept together: the kept one first, then a line naming the
/// record the other came from.
fn both_texts(k: &Value, a: &Value, absorb_label: &str) -> Value {
    match (k.as_str(), a.as_str()) {
        (Some(k), Some(a)) => json!(format!("{k}\n\n— {absorb_label} —\n{a}")),
        _ => k.clone(),
    }
}

/// The display name not chosen, as an alias among the other names.
fn add_alias(merged: &mut Value, name: &Value) {
    let Some(display) = name.get("display").and_then(Value::as_str) else {
        return;
    };
    let Some(identity) = merged.get_mut("identity").and_then(Value::as_object_mut) else {
        return;
    };
    let names = identity.entry("names").or_insert_with(|| json!([]));
    let Some(list) = names.as_array_mut() else {
        return;
    };
    if list
        .iter()
        .any(|n| n.get("display").and_then(Value::as_str) == Some(display))
    {
        return;
    }
    let mut alias = name.clone();
    if let Some(o) = alias.as_object_mut() {
        o.insert("type".into(), json!("alias"));
    }
    list.push(alias);
}

/// Set the value at a dotted path, creating objects on the way.
fn set_path(root: &mut Value, path: &str, value: Value) {
    let mut cur = root;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, part) in parts.iter().enumerate() {
        if !cur.is_object() {
            *cur = json!({});
        }
        let Some(obj) = cur.as_object_mut() else {
            return;
        };
        if i == parts.len() - 1 {
            obj.insert((*part).to_string(), value);
            return;
        }
        cur = obj.entry((*part).to_string()).or_insert_with(|| json!({}));
    }
}

/// Replace every `*_id` equal to `from` with `to`; how many were replaced.
fn rewrite(v: &mut Value, from: &str, to: &str) -> usize {
    match v {
        Value::Object(m) => m
            .iter_mut()
            .map(|(k, val)| {
                let mut n = 0;
                if is_reference(k) && val.as_str() == Some(from) {
                    *val = json!(to);
                    n = 1;
                }
                n + rewrite(val, from, to)
            })
            .sum(),
        Value::Array(a) => a.iter_mut().map(|x| rewrite(x, from, to)).sum(),
        _ => 0,
    }
}

/// Keep one entry for `id` in a list keyed by `key`, the one with the higher
/// confidence. Whether anything was removed.
fn dedupe_list(list: Option<&mut Value>, key: &str, id: &str) -> bool {
    let Some(Value::Array(list)) = list else {
        return false;
    };
    let mut best: Option<(usize, f64)> = None;
    for (i, e) in list.iter().enumerate() {
        if e.get(key).and_then(Value::as_str) == Some(id) {
            let c = confidence(e);
            if best.is_none_or(|(_, bc)| c > bc) {
                best = Some((i, c));
            }
        }
    }
    let Some((keep_at, _)) = best else {
        return false;
    };
    let before = list.len();
    let mut i = 0;
    list.retain(|e| {
        let here = i;
        i += 1;
        here == keep_at || e.get(key).and_then(Value::as_str) != Some(id)
    });
    list.len() != before
}

/// The merged record and the events made for it, checked against the schema
/// as `add_entity` checks a new record: findings are warnings, not refusals.
fn check(b: &FlatBundle, kind: EntityKind, keep: &str, report: &Value) -> Vec<Diagnostic> {
    let version = b
        .manifest
        .get("axgf")
        .and_then(Value::as_str)
        .unwrap_or("1.0");
    let mut out = Vec::new();
    let mut one = |kind: EntityKind, id: &str, v: &Value| {
        out.extend(crate::logic::validate::schema_diagnostics(
            version,
            kind.singular(),
            Some(kind),
            v,
            Some(format!("{}/{id}", kind.collection())),
        ));
        profile::check_entity(kind, id, v, Some(version), &mut out);
    };
    if let Some(v) = collection(b, kind).get(keep) {
        one(kind, keep, v);
    }
    for e in report["events_created"].as_array().into_iter().flatten() {
        if let Some(id) = e["id"].as_str() {
            if let Some(v) = b.events.get(id) {
                one(EntityKind::Event, id, v);
            }
        }
    }
    out
}

fn refresh_manifest(b: &mut FlatBundle) {
    raise_manifest_version(b);
    let stats = compute_stats(b);
    if let Value::Object(ref mut m) = b.manifest {
        m.insert("stats".into(), stats);
        m.insert("updated_at".into(), Value::String(now_iso8601_utc()));
    }
}

/// How a record names itself, for a line in a joined text.
fn label(kind: EntityKind, v: &Value, id: &str) -> String {
    let named = match kind {
        EntityKind::Person => v.pointer("/identity/name/display").and_then(Value::as_str),
        EntityKind::Source => v.get("title").and_then(Value::as_str),
        EntityKind::Place => v.pointer("/names/0/value").and_then(Value::as_str),
        _ => v.get("name").and_then(Value::as_str),
    };
    let short: String = id.chars().take(8).collect();
    match named {
        Some(n) if !n.is_empty() => format!("{n} · {short}"),
        _ => short,
    }
}

fn collection(b: &FlatBundle, kind: EntityKind) -> &BTreeMap<String, Value> {
    match kind {
        EntityKind::Person => &b.persons,
        EntityKind::Family => &b.families,
        EntityKind::Event => &b.events,
        EntityKind::Link => &b.links,
        EntityKind::Occupation => &b.occupations,
        EntityKind::Source => &b.sources,
        EntityKind::Place => &b.places,
        EntityKind::Document => &b.documents,
    }
}

fn collection_mut(b: &mut FlatBundle, kind: EntityKind) -> &mut BTreeMap<String, Value> {
    match kind {
        EntityKind::Person => &mut b.persons,
        EntityKind::Family => &mut b.families,
        EntityKind::Event => &mut b.events,
        EntityKind::Link => &mut b.links,
        EntityKind::Occupation => &mut b.occupations,
        EntityKind::Source => &mut b.sources,
        EntityKind::Place => &mut b.places,
        EntityKind::Document => &mut b.documents,
    }
}

fn collections(b: &FlatBundle) -> [(&'static str, &BTreeMap<String, Value>); 8] {
    [
        ("persons", &b.persons),
        ("families", &b.families),
        ("events", &b.events),
        ("links", &b.links),
        ("occupations", &b.occupations),
        ("sources", &b.sources),
        ("places", &b.places),
        ("documents", &b.documents),
    ]
}

fn collections_mut(b: &mut FlatBundle) -> [(&'static str, &mut BTreeMap<String, Value>); 8] {
    [
        ("persons", &mut b.persons),
        ("families", &mut b.families),
        ("events", &mut b.events),
        ("links", &mut b.links),
        ("occupations", &mut b.occupations),
        ("sources", &mut b.sources),
        ("places", &mut b.places),
        ("documents", &mut b.documents),
    ]
}
