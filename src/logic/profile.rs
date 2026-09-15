// SPDX-License-Identifier: Apache-2.0
//! # profile — what AXGF 1.1 asks a validator and a writer to check
//!
//! The JSON Schema already rejects a term outside its vocabulary. This module
//! exists because rejecting is not reporting: a schema failure for a blood
//! group of `Z` arrives as "`\"Z\"` is not one of …" under
//! `SCHEMA_VALIDATION_FAILED`, indistinguishable from a missing required
//! field, and SPEC_1.1 §7.2 asks for it to be reported as what it is. So the
//! 1.1 checks walk the attribute registry
//! ([`crate::model::profile::registry`]) over the entity's JSON and speak in
//! its terms:
//!
//! - **`OUT_OF_VOCABULARY`** (warning) — a value that is not a term of its
//!   vocabulary, naming the attribute path, the value and the vocabulary.
//!   Also: a rank outside the vocabulary its country selects, a rank for a
//!   country 1.1 registers no vocabulary for, an analyte outside its panel,
//!   an artefact type the attribute does not allow.
//! - **`CLAIM_INCONSISTENT`** (warning) — the semantic rules of SPEC_1.1 §7.3
//!   that a schema cannot express: a haplogroup subclade that does not begin
//!   with its major clade, a rank whose stated category is not the one its
//!   vocabulary gives it, an epigenetic clock with the wrong kind of result,
//!   a period that ends before it starts, two causes of death with one
//!   sequence number.
//! - **`SPEC_VERSION_MISMATCH`** (warning) — an entity that carries 1.1
//!   attributes but declares 1.0, or declares a version newer than its
//!   bundle (SPEC_1.1 §2.1).
//! - **`UNKNOWN_ATTRIBUTE`** (info) — a key inside a 1.1 block that 1.1 does
//!   not define. Not an error — 1.0 P9 applies — but a misspelt attribute is
//!   otherwise invisible.
//!
//! To keep one finding from being reported twice, [`is_vocabulary_error`]
//! tells the structural pass which schema errors this module already covers:
//! enumeration failures at locations 1.1 owns. Everything else the schema
//! finds is still reported as it always was.
//!
//! Every function here is a pure function of JSON. None of them mutates
//! anything, reads anything but its arguments, or knows which caller asked.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Severity};
use crate::logic::crud::EntityKind;
use crate::model::profile::registry::{self, Attribute, Cardinality, Shape};
use crate::model::profile::vocab::{self, Vocabulary};
use crate::SUPPORTED_SPEC_VERSIONS;

/// 1.0's visibility vocabulary, which `identity.class_visibility` reuses for
/// its values.
const VISIBILITY: &[&str] = &["public", "members", "contributors", "private"];

/// The 1.1 attributes inside the 1.0 `identity`, `birth` and `death` blocks.
fn extends_1_0_block(a: &Attribute) -> bool {
    matches!(a.block, "identity" | "birth" | "death")
}

/// The position of a spec version in [`SUPPORTED_SPEC_VERSIONS`], which lists
/// them oldest first. `None` for a version this build does not know.
pub(crate) fn version_rank(v: &str) -> Option<usize> {
    SUPPORTED_SPEC_VERSIONS.iter().position(|s| *s == v)
}

/// Whether an entity carries anything 1.1 defines — which is what obliges it
/// to declare `"1.1"`.
pub(crate) fn uses_1_1(kind: EntityKind, entity: &Value) -> bool {
    match kind {
        EntityKind::Person => {
            registry::PROFILE_BLOCKS
                .iter()
                .any(|b| entity.get(*b).is_some())
                || registry::attributes()
                    .filter(|a| extends_1_0_block(a))
                    .any(|a| entity.get(a.block).and_then(|b| b.get(a.key)).is_some())
                || entity.pointer("/identity/class_visibility").is_some()
        }
        EntityKind::Family => entity
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|cs| cs.iter().any(|c| c.get("lineage").is_some())),
        EntityKind::Link => entity.get("relation").is_some(),
        EntityKind::Occupation => entity.get("position").is_some(),
        _ => false,
    }
}

/// The oldest version an entity's content fits in: `"1.1"` when it carries
/// 1.1 attributes, `"1.0"` otherwise.
pub(crate) fn required_version(kind: EntityKind, entity: &Value) -> &'static str {
    if uses_1_1(kind, entity) {
        "1.1"
    } else {
        "1.0"
    }
}

/// Whether the manifest carries anything 1.1 defines.
pub(crate) fn manifest_uses_1_1(manifest: &Value) -> bool {
    manifest.pointer("/privacy/withheld_classes").is_some()
}

/// Whether a schema error at `path` is an older schema refusing an
/// `axgf_version` newer than itself. [`check_entity`] reports that as
/// `SPEC_VERSION_MISMATCH`, naming both versions, so the schema's bare
/// "\"1.0\" was expected" would only say it again, less clearly.
pub(crate) fn is_declaration_error(path: &str, entity: &Value, manifest_version: &str) -> bool {
    path == "/axgf_version"
        && entity
            .get("axgf_version")
            .and_then(Value::as_str)
            .and_then(version_rank)
            .zip(version_rank(manifest_version))
            .is_some_and(|(declared, manifest)| declared > manifest)
}

/// Whether a schema error at `instance_path` is an enumeration failure this
/// module reports itself as `OUT_OF_VOCABULARY`.
///
/// `path` is the error's JSON pointer inside the entity (`/health/blood_group/value`).
/// Only locations 1.1 owns qualify: a 1.0 enumeration — a gender, a union
/// type — keeps its `SCHEMA_VALIDATION_FAILED`, exactly as before.
pub(crate) fn is_vocabulary_error(kind: Option<EntityKind>, path: &str, is_enum: bool) -> bool {
    if !is_enum {
        return false;
    }
    let under = |prefix: &str| path == prefix || path.starts_with(&format!("{prefix}/"));
    match kind {
        None => under("/privacy/withheld_classes"),
        Some(EntityKind::Person) => {
            registry::PROFILE_BLOCKS
                .iter()
                .any(|b| under(&format!("/{b}")))
                || registry::attributes()
                    .filter(|a| extends_1_0_block(a))
                    .any(|a| under(&format!("/{}/{}", a.block, a.key)))
                || under("/identity/class_visibility")
        }
        Some(EntityKind::Family) => {
            let mut parts = path.split('/').skip(1);
            parts.next() == Some("children")
                && parts.next().is_some_and(|i| i.parse::<usize>().is_ok())
                && parts.next() == Some("lineage")
        }
        Some(EntityKind::Link) => under("/relation"),
        _ => false,
    }
}

/// Everything 1.1 asks about one entity, appended to `out`.
///
/// `manifest_version` is the version of the bundle the entity is in, for the
/// declaration checks of SPEC_1.1 §2.1.
pub(crate) fn check_entity(
    kind: EntityKind,
    id: &str,
    entity: &Value,
    manifest_version: Option<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let entity_ref = format!("{}/{id}", kind.collection());
    let mut ctx = Ctx {
        entity_ref: &entity_ref,
        out,
    };
    match kind {
        EntityKind::Person => check_person(&mut ctx, entity),
        EntityKind::Family => {
            if let Some(children) = entity.get("children").and_then(Value::as_array) {
                for (i, c) in children.iter().enumerate() {
                    if let Some(v) = c.get("lineage") {
                        ctx.term(&format!("children[{i}].lineage"), v, &vocab::LINEAGE);
                    }
                }
            }
        }
        EntityKind::Link => {
            if let Some(v) = entity.get("relation") {
                ctx.term("relation", v, &vocab::LINK_RELATION);
            }
        }
        _ => {}
    }
    check_declaration(&mut ctx, kind, entity, manifest_version);
}

/// The 1.1 checks on a manifest.
pub(crate) fn check_manifest(manifest: &Value, out: &mut Vec<Diagnostic>) {
    let mut ctx = Ctx {
        entity_ref: "manifest",
        out,
    };
    if let Some(list) = manifest
        .pointer("/privacy/withheld_classes")
        .and_then(Value::as_array)
    {
        for (i, v) in list.iter().enumerate() {
            ctx.term(
                &format!("privacy.withheld_classes[{i}]"),
                v,
                &vocab::SENSITIVE_CLASS,
            );
        }
    }
    if manifest_uses_1_1(manifest) && manifest.get("axgf").and_then(Value::as_str) == Some("1.0") {
        ctx.push(
            DiagnosticCode::SpecVersionMismatch,
            Severity::Warning,
            "privacy.withheld_classes is AXGF 1.1, but the manifest declares axgf 1.0".to_string(),
        );
    }
}

struct Ctx<'a> {
    entity_ref: &'a str,
    out: &'a mut Vec<Diagnostic>,
}

impl Ctx<'_> {
    fn push(&mut self, code: DiagnosticCode, severity: Severity, message: String) {
        self.out.push(Diagnostic {
            code,
            severity,
            message: format!("{}: {message}", self.entity_ref),
            entity_ref: (self.entity_ref != "manifest").then(|| self.entity_ref.to_string()),
        });
    }

    /// Report `v` if it is a string that is not a term of `voc`. A value of
    /// the wrong type is the schema's to report.
    fn term(&mut self, path: &str, v: &Value, voc: &Vocabulary) {
        if let Some(s) = v.as_str() {
            if !voc.contains(s) {
                self.push(
                    DiagnosticCode::OutOfVocabulary,
                    Severity::Warning,
                    format!("{path} {s:?} is not a term of vocabulary {}", voc.name),
                );
            }
        }
    }

    fn inconsistent(&mut self, message: String) {
        self.push(
            DiagnosticCode::ClaimInconsistent,
            Severity::Warning,
            message,
        );
    }
}

fn check_person(ctx: &mut Ctx<'_>, person: &Value) {
    for a in registry::attributes() {
        let Some(held) = person.get(a.block).and_then(|b| b.get(a.key)) else {
            continue;
        };
        match (a.cardinality, held) {
            (Cardinality::Series, Value::Array(claims)) => {
                for (i, claim) in claims.iter().enumerate() {
                    check_claim(ctx, a, &format!("{}[{i}]", a.path), claim);
                }
            }
            (Cardinality::Single, Value::Object(_)) => check_claim(ctx, a, a.path, held),
            // An array where one claim belongs, or one claim where a series
            // belongs, is a structural failure the schema reports.
            _ => {}
        }
        if a.path == "death.causes" {
            duplicate_sequences(ctx, held);
        }
    }

    if let Some(Value::Object(map)) = person.pointer("/identity/class_visibility") {
        for (class, visibility) in map {
            if !vocab::SENSITIVE_CLASS.contains(class) {
                ctx.push(
                    DiagnosticCode::OutOfVocabulary,
                    Severity::Warning,
                    format!(
                        "identity.class_visibility key {class:?} is not a term of vocabulary sensitive_class"
                    ),
                );
            }
            if let Some(v) = visibility.as_str() {
                if !VISIBILITY.contains(&v) {
                    ctx.push(
                        DiagnosticCode::OutOfVocabulary,
                        Severity::Warning,
                        format!(
                            "identity.class_visibility.{class} {v:?} is not a term of vocabulary visibility"
                        ),
                    );
                }
            }
        }
    }

    for block in registry::PROFILE_BLOCKS {
        let Some(Value::Object(map)) = person.get(*block) else {
            continue;
        };
        let known: BTreeSet<&str> = registry::attributes()
            .filter(|a| a.block == *block)
            .map(|a| a.key)
            .collect();
        for key in map.keys() {
            if !known.contains(key.as_str()) {
                ctx.push(
                    DiagnosticCode::UnknownAttribute,
                    Severity::Info,
                    format!("{block}.{key} is not an AXGF 1.1 attribute; it is preserved"),
                );
            }
        }
    }
}

fn check_claim(ctx: &mut Ctx<'_>, a: &Attribute, path: &str, claim: &Value) {
    if let Some(value) = claim.get("value") {
        check_value(ctx, a, &format!("{path}.value"), &a.shape, value);
    }
    let bound = |k: &str| {
        claim
            .get(k)
            .and_then(|b| b.get("date"))
            .and_then(|d| d.get("value"))
            .and_then(Value::as_str)
    };
    if let (Some(from), Some(until)) = (bound("valid_from"), bound("valid_until")) {
        // Partial dates compare on their common prefix: "1950" and "1950-03"
        // agree, and only an end strictly before the start is a finding.
        let n = from.len().min(until.len());
        if until.get(..n) < from.get(..n) {
            ctx.inconsistent(format!("{path} ends ({until}) before it starts ({from})"));
        }
    }
}

fn check_value(ctx: &mut Ctx<'_>, a: &Attribute, path: &str, shape: &Shape, v: &Value) {
    match shape {
        Shape::Vocab(voc) => ctx.term(path, v, voc),
        Shape::Artefact(allowed) => {
            if let Some(t) = v.get("artefact_type").and_then(Value::as_str) {
                if !allowed.contains(&t) {
                    ctx.push(
                        DiagnosticCode::OutOfVocabulary,
                        Severity::Warning,
                        format!(
                            "{path}.artefact_type {t:?} is not allowed here; {} takes {}",
                            a.path,
                            allowed.join(", ")
                        ),
                    );
                }
            }
            if let Some(c) = v.get("consent") {
                ctx.term(&format!("{path}.consent"), c, &vocab::CONSENT);
            }
        }
        Shape::Object { fields, .. } => {
            let Some(obj) = v.as_object() else {
                return;
            };
            for f in fields.iter() {
                if let Some(fv) = obj.get(f.key) {
                    if f.shape != Shape::Rank {
                        check_value(ctx, a, &format!("{path}.{}", f.key), &f.shape, fv);
                    }
                }
            }
            if fields.iter().any(|f| f.shape == Shape::Rank) {
                check_rank(ctx, path, obj);
            }
            match a.path {
                "health.lab_results" => check_analyte(ctx, path, obj),
                "genomics.y_haplogroup" | "genomics.mt_haplogroup" => {
                    check_subclade(ctx, path, obj)
                }
                "genomics.epigenetic_age" => check_clock(ctx, path, obj),
                _ => {}
            }
        }
        _ => {}
    }
}

fn check_rank(ctx: &mut Ctx<'_>, path: &str, obj: &Map<String, Value>) {
    let Some(rank) = obj.get("rank").and_then(Value::as_str) else {
        return;
    };
    let country = obj.get("country").and_then(Value::as_str);
    let Some(ranks) = country.and_then(vocab::ranks_for) else {
        ctx.push(
            DiagnosticCode::OutOfVocabulary,
            Severity::Warning,
            format!(
                "{path}.rank {rank:?} needs a country with a registered rank vocabulary ({}); \
                 record a rank from anywhere else as rank_text",
                vocab::MILITARY_RANKS
                    .iter()
                    .map(|r| r.country)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        return;
    };
    let Some(entry) = ranks.ranks.iter().find(|r| r.term == rank) else {
        ctx.push(
            DiagnosticCode::OutOfVocabulary,
            Severity::Warning,
            format!(
                "{path}.rank {rank:?} is not a term of vocabulary {}",
                ranks.vocabulary.name
            ),
        );
        return;
    };
    if let Some(category) = obj.get("category").and_then(Value::as_str) {
        if category != entry.category {
            ctx.inconsistent(format!(
                "{path}.category {category:?} disagrees with rank {rank:?}, which is {:?}",
                entry.category
            ));
        }
    }
}

fn check_analyte(ctx: &mut Ctx<'_>, path: &str, obj: &Map<String, Value>) {
    let (Some(panel), Some(analyte)) = (
        obj.get("panel").and_then(Value::as_str),
        obj.get("analyte").and_then(Value::as_str),
    ) else {
        return;
    };
    let Some((_, allowed)) = vocab::LAB_PANEL_ANALYTES.iter().find(|(p, _)| *p == panel) else {
        return; // an unknown panel is already reported as a term
    };
    if vocab::LAB_ANALYTE.contains(analyte) && !allowed.contains(&analyte) {
        ctx.push(
            DiagnosticCode::OutOfVocabulary,
            Severity::Warning,
            format!("{path}.analyte {analyte:?} is not in the {panel} panel"),
        );
    }
}

fn check_subclade(ctx: &mut Ctx<'_>, path: &str, obj: &Map<String, Value>) {
    let (Some(major), Some(subclade)) = (
        obj.get("major").and_then(Value::as_str),
        obj.get("subclade").and_then(Value::as_str),
    ) else {
        return;
    };
    // `R-M269` and `R1b1a` are both under `R`; `HV0` is under `HV` and `H1a`
    // is not under `HV`, so a letter prefix alone is not enough when the
    // major clade is itself two characters and the next one is a letter.
    let under = subclade.starts_with(major)
        && subclade[major.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_uppercase());
    if !under {
        ctx.inconsistent(format!(
            "{path}.subclade {subclade:?} does not belong to major haplogroup {major:?}"
        ));
    }
}

fn check_clock(ctx: &mut Ctx<'_>, path: &str, obj: &Map<String, Value>) {
    let Some(clock) = obj.get("clock").and_then(Value::as_str) else {
        return;
    };
    match clock {
        "dunedinpace" if obj.get("pace").is_none() => ctx.inconsistent(format!(
            "{path}: dunedinpace measures a pace; give pace, not age_years"
        )),
        "other" | "dunedinpace" => {}
        c if vocab::EPIGENETIC_CLOCK.contains(c) && obj.get("age_years").is_none() => {
            ctx.inconsistent(format!("{path}: {c} measures an age; give age_years"))
        }
        _ => {}
    }
}

fn duplicate_sequences(ctx: &mut Ctx<'_>, causes: &Value) {
    let Some(list) = causes.as_array() else {
        return;
    };
    let mut seen = BTreeSet::new();
    for (i, c) in list.iter().enumerate() {
        if let Some(n) = c.pointer("/value/sequence").and_then(Value::as_i64) {
            if !seen.insert(n) {
                ctx.inconsistent(format!(
                    "death.causes[{i}].value.sequence {n} is used by an earlier cause"
                ));
            }
        }
    }
}

fn check_declaration(
    ctx: &mut Ctx<'_>,
    kind: EntityKind,
    entity: &Value,
    manifest_version: Option<&str>,
) {
    let declared = entity.get("axgf_version").and_then(Value::as_str);
    if uses_1_1(kind, entity) && declared == Some("1.0") {
        ctx.push(
            DiagnosticCode::SpecVersionMismatch,
            Severity::Warning,
            "carries AXGF 1.1 attributes but declares axgf_version 1.0".to_string(),
        );
    }
    if let (Some(d), Some(m)) = (declared, manifest_version) {
        if let (Some(dr), Some(mr)) = (version_rank(d), version_rank(m)) {
            if dr > mr {
                ctx.push(
                    DiagnosticCode::SpecVersionMismatch,
                    Severity::Warning,
                    format!("declares axgf_version {d}, newer than the bundle's manifest ({m})"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_vocabulary_error_is_one_the_profile_owns() {
        assert!(is_vocabulary_error(
            Some(EntityKind::Person),
            "/health/blood_group/value",
            true
        ));
        assert!(is_vocabulary_error(
            Some(EntityKind::Person),
            "/death/causes/0/value/icd10_chapter",
            true
        ));
        assert!(
            !is_vocabulary_error(Some(EntityKind::Person), "/identity/gender/value", true),
            "a 1.0 vocabulary keeps its SCHEMA_VALIDATION_FAILED"
        );
        assert!(
            !is_vocabulary_error(Some(EntityKind::Person), "/death/cause", true),
            "1.0's death.cause is not 1.1's death.causes"
        );
        assert!(is_vocabulary_error(
            Some(EntityKind::Family),
            "/children/3/lineage",
            true
        ));
        assert!(!is_vocabulary_error(
            Some(EntityKind::Person),
            "/health/blood_group/value",
            false
        ));
    }

    #[test]
    fn a_person_with_a_1_1_block_requires_1_1_and_one_without_does_not() {
        let bare = json!({"identity": {"name": {"display": "A"}}});
        assert_eq!(required_version(EntityKind::Person, &bare), "1.0");
        let with = json!({"identity": {"name": {"display": "A"}}, "health": {}});
        assert_eq!(required_version(EntityKind::Person, &with), "1.1");
        let time = json!({"birth": {"time": {"value": "05:40"}}});
        assert_eq!(required_version(EntityKind::Person, &time), "1.1");
        let lineage = json!({"children": [{"person_id": "x", "lineage": "step"}]});
        assert_eq!(required_version(EntityKind::Family, &lineage), "1.1");
    }

    #[test]
    fn a_subclade_belongs_to_its_major_clade_and_no_other() {
        let check = |major: &str, sub: &str| {
            let mut out = Vec::new();
            let mut ctx = Ctx {
                entity_ref: "persons/x",
                out: &mut out,
            };
            let obj = json!({"major": major, "subclade": sub});
            check_subclade(
                &mut ctx,
                "genomics.y_haplogroup.value",
                obj.as_object().unwrap(),
            );
            out.is_empty()
        };
        assert!(check("R", "R-M269"));
        assert!(check("R", "R1b1a1b"));
        assert!(check("HV", "HV0a"));
        assert!(check("H", "H1a1"));
        assert!(!check("H", "HV0a"), "HV0 is not under H");
        assert!(!check("R", "I-M253"));
        assert!(check("L3", "L3e2b"));
    }
}
