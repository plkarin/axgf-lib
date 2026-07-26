// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the Phase 4 validate() surface.
//!
//! Contract covered:
//! - A well-formed empty bundle validates with `Status::Ok` and no
//!   diagnostics.
//! - Structural (JSON Schema) violations produce
//!   `SCHEMA_VALIDATION_FAILED` warnings, non-blocking.
//! - Dangling references produce `DANGLING_REFERENCE` warnings,
//!   non-blocking (severity Warning).
//! - Parent/child cycles produce `CYCLE_DETECTED` errors (severity
//!   Error, but envelope status remains Ok — validate is a *report*).
//! - Child born before parent produces `CHRONOLOGY_CONFLICT` warnings.
//! - Two families sharing the same spouse set produce
//!   `DUPLICATE_UNIQUE_REF` warnings.
//! - Unsupported spec version is refused up front with
//!   `UNSUPPORTED_SPEC_VERSION` and never reaches the semantic layer.

use axgf_rs::boundary::envelope::{Severity, Status};
use axgf_rs::{create_bundle, validate};
use serde_json::{json, Value};

fn ok_env(v: &Value) -> axgf_rs::boundary::envelope::Envelope {
    validate(&serde_json::to_string(v).unwrap())
}

fn codes(env: &axgf_rs::boundary::envelope::Envelope) -> Vec<&'static str> {
    env.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn minimal_person(id: &str, display: &str, birth_year: Option<&str>) -> Value {
    let mut p = json!({
        "id": id, "type": "person", "axgf_version": "1.0",
        "identity": {
            "name": {"display": display, "components": []},
            "gender": {"value": "U"},
            "is_living": false
        }
    });
    if let Some(y) = birth_year {
        p["birth"] = json!({"date": {"value": y}});
    }
    p
}

fn minimal_family(id: &str, spouses: &[&str], children: &[&str]) -> Value {
    json!({
        "id": id, "type": "family", "axgf_version": "1.0",
        "union": {
            "type": "marriage",
            "persons": spouses.iter().map(|s| json!({"person_id": s, "role": "spouse"})).collect::<Vec<_>>()
        },
        "children": children.iter().map(|c| json!({"person_id": c})).collect::<Vec<_>>()
    })
}

// ---------- Well-formed / trivial ----------

#[test]
fn clean_empty_bundle_has_no_diagnostics() {
    let flat = create_bundle(None).data;
    let env = ok_env(&flat);
    assert_eq!(env.status, Status::Ok, "diags: {:?}", env.diagnostics);
    assert!(
        env.diagnostics.is_empty(),
        "expected zero diagnostics on a fresh empty bundle, got {:?}",
        env.diagnostics
    );
    // Data payload counters all zero.
    assert_eq!(env.data["errors"], 0);
    assert_eq!(env.data["warnings"], 0);
    assert_eq!(env.data["total"], 0);
}

#[test]
fn clean_two_person_family_bundle_validates() {
    let mut flat = create_bundle(Some("Clean")).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let ch = "550e8400-e29b-41d4-a716-446655440003";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    flat["persons"] = json!({
        p1: minimal_person(p1, "Ann", Some("1900")),
        p2: minimal_person(p2, "Bob", Some("1902")),
        ch: minimal_person(ch, "Kid", Some("1930")),
    });
    flat["families"] = json!({
        fam: minimal_family(fam, &[p1, p2], &[ch]),
    });
    let env = ok_env(&flat);
    assert_eq!(env.status, Status::Ok);
    assert!(
        env.diagnostics.is_empty(),
        "expected clean: {:?}",
        env.diagnostics
    );
}

// ---------- Structural ----------

#[test]
fn schema_violation_produces_warning_not_error_status() {
    // Person missing required identity.gender field.
    let mut flat = create_bundle(None).data;
    let id = "550e8400-e29b-41d4-a716-446655440077";
    flat["persons"] = json!({
        id: {
            "id": id, "type": "person", "axgf_version": "1.0",
            "identity": {"name": {"display": "X", "components": []}, "is_living": false}
        }
    });
    let env = ok_env(&flat);
    assert_eq!(env.status, Status::Ok, "validation is non-blocking");
    assert!(codes(&env).contains(&"SCHEMA_VALIDATION_FAILED"),
            "got codes: {:?}", codes(&env));
    let sv = env
        .diagnostics
        .iter()
        .find(|d| d.code.as_str() == "SCHEMA_VALIDATION_FAILED")
        .unwrap();
    assert_eq!(sv.severity, Severity::Warning);
    assert_eq!(sv.entity_ref.as_deref(), Some("persons/550e8400-e29b-41d4-a716-446655440077"));
}

// ---------- Dangling refs ----------

#[test]
fn dangling_reference_in_family_child_produces_warning() {
    let mut flat = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let ghost = "550e8400-e29b-41d4-a716-446655440099";
    flat["persons"] = json!({ p1: minimal_person(p1, "A", None) });
    flat["families"] = json!({
        "aaaa1234-e29b-41d4-a716-446655440001":
            minimal_family("aaaa1234-e29b-41d4-a716-446655440001", &[p1], &[ghost])
    });
    let env = ok_env(&flat);
    assert_eq!(env.status, Status::Ok);
    let dr: Vec<_> = env
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "DANGLING_REFERENCE")
        .collect();
    assert!(!dr.is_empty(), "expected DANGLING_REFERENCE: {:?}", env.diagnostics);
    assert_eq!(dr[0].severity, Severity::Warning);
    assert!(dr[0].message.contains(ghost), "message should name the missing id");
}

#[test]
fn dangling_place_and_source_refs_from_event_are_reported() {
    let mut flat = create_bundle(None).data;
    let ev = "aaaa1234-e29b-41d4-a716-446655440100";
    flat["events"] = json!({
        ev: {
            "id": ev, "type": "event", "axgf_version": "1.0",
            "category": "birth", "date": {"value": "1900"},
            "place_id":  "550e8400-e29b-41d4-a716-4466554400aa",
            "source_id": "550e8400-e29b-41d4-a716-4466554400bb"
        }
    });
    let env = ok_env(&flat);
    let dr: Vec<_> = env
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "DANGLING_REFERENCE")
        .collect();
    assert_eq!(dr.len(), 2, "expected place + source dangling refs, got: {:?}", env.diagnostics);
}

// ---------- Cycles ----------

#[test]
fn direct_self_parent_cycle_is_flagged_as_error_severity() {
    // Person listed as both parent and child of the same family.
    let mut flat = create_bundle(None).data;
    let p = "550e8400-e29b-41d4-a716-446655440001";
    flat["persons"] = json!({ p: minimal_person(p, "Loop", None) });
    flat["families"] = json!({
        "aaaa1234-e29b-41d4-a716-446655440001":
            minimal_family("aaaa1234-e29b-41d4-a716-446655440001", &[p], &[p])
    });
    let env = ok_env(&flat);
    // Envelope stays Ok (validate is a report), but severity is Error.
    assert_eq!(env.status, Status::Ok);
    let c = env
        .diagnostics
        .iter()
        .find(|d| d.code.as_str() == "CYCLE_DETECTED")
        .expect("expected a CYCLE_DETECTED diagnostic");
    assert_eq!(c.severity, Severity::Error);
    assert!(c.entity_ref.as_deref().unwrap().contains(p));
}

#[test]
fn cross_family_ancestor_loop_is_detected() {
    // A → B (family 1), B → A (family 2) forms a cycle.
    let mut flat = create_bundle(None).data;
    let a = "550e8400-e29b-41d4-a716-446655440001";
    let b = "550e8400-e29b-41d4-a716-446655440002";
    let f1 = "aaaa1234-e29b-41d4-a716-446655440001";
    let f2 = "aaaa1234-e29b-41d4-a716-446655440002";
    flat["persons"] = json!({
        a: minimal_person(a, "A", None),
        b: minimal_person(b, "B", None),
    });
    flat["families"] = json!({
        f1: minimal_family(f1, &[a], &[b]),
        f2: minimal_family(f2, &[b], &[a]),
    });
    let env = ok_env(&flat);
    let cs: Vec<_> = env
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "CYCLE_DETECTED")
        .collect();
    assert!(!cs.is_empty(), "expected cross-family cycle: {:?}", env.diagnostics);
}

// ---------- Chronology ----------

#[test]
fn child_born_before_parent_triggers_chronology_conflict_warning() {
    let mut flat = create_bundle(None).data;
    let parent = "550e8400-e29b-41d4-a716-446655440001";
    let child = "550e8400-e29b-41d4-a716-446655440002";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    flat["persons"] = json!({
        parent: minimal_person(parent, "Parent", Some("1950")),
        child:  minimal_person(child,  "Kid",    Some("1900")),
    });
    flat["families"] = json!({ fam: minimal_family(fam, &[parent], &[child]) });
    let env = ok_env(&flat);
    let cc = env
        .diagnostics
        .iter()
        .find(|d| d.code.as_str() == "CHRONOLOGY_CONFLICT")
        .expect("expected CHRONOLOGY_CONFLICT");
    assert_eq!(cc.severity, Severity::Warning);
    assert!(cc.message.contains("1900") && cc.message.contains("1950"),
            "message should carry both years: {}", cc.message);
}

#[test]
fn chronology_ok_when_dates_missing() {
    // No date on either → no conflict, no warning.
    let mut flat = create_bundle(None).data;
    let parent = "550e8400-e29b-41d4-a716-446655440001";
    let child = "550e8400-e29b-41d4-a716-446655440002";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    flat["persons"] = json!({
        parent: minimal_person(parent, "P", None),
        child:  minimal_person(child,  "K", None),
    });
    flat["families"] = json!({ fam: minimal_family(fam, &[parent], &[child]) });
    let env = ok_env(&flat);
    assert!(!codes(&env).contains(&"CHRONOLOGY_CONFLICT"),
            "no dates → no chronology warning: {:?}", env.diagnostics);
}

// ---------- Duplicates ----------

#[test]
fn two_families_with_same_spouse_set_are_flagged_as_duplicate() {
    let mut flat = create_bundle(None).data;
    let a = "550e8400-e29b-41d4-a716-446655440001";
    let b = "550e8400-e29b-41d4-a716-446655440002";
    let f1 = "aaaa1234-e29b-41d4-a716-446655440001";
    let f2 = "aaaa1234-e29b-41d4-a716-446655440002";
    flat["persons"] = json!({
        a: minimal_person(a, "A", None), b: minimal_person(b, "B", None),
    });
    flat["families"] = json!({
        f1: minimal_family(f1, &[a, b], &[]),
        f2: minimal_family(f2, &[b, a], &[]),  // same set, different order
    });
    let env = ok_env(&flat);
    let dup = env
        .diagnostics
        .iter()
        .find(|d| d.code.as_str() == "DUPLICATE_UNIQUE_REF")
        .expect("expected DUPLICATE_UNIQUE_REF");
    assert_eq!(dup.severity, Severity::Warning);
    assert!(dup.message.contains(f1) || dup.message.contains(f2));
}

// ---------- Version gating ----------

#[test]
fn validate_rejects_unsupported_spec_version_before_running_checks() {
    let mut flat = create_bundle(None).data;
    flat["manifest"]["axgf"] = json!("9.9");
    let env = ok_env(&flat);
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics.len(), 1);
    assert_eq!(env.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
    // data should be null on error
    assert!(env.data.is_null());
}

#[test]
fn validate_rejects_invalid_json() {
    let env = validate("this is not JSON");
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "INVALID_JSON");
}
