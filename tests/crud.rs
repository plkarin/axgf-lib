// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the Phase 5 CRUD operations.
//!
//! Contract covered:
//! - add generates a UUID v4 when id is missing.
//! - add refuses a duplicate id with `ENTITY_ALREADY_EXISTS`.
//! - update refuses a missing id with `ENTITY_NOT_FOUND`.
//! - delete under `Reject` is blocked by any referrer and leaves
//!   the bundle unchanged with `DELETE_BLOCKED_BY_REFERENCE`.
//! - delete under `Cascade` removes the entity and physically strips
//!   references (array items dropped, scalar `_id` fields removed).
//! - delete under `Orphan` removes the entity but nulls references
//!   (keys preserved; array items preserved with `_id` = null).
//! - Every successful mutation refreshes `manifest.stats`.

use axgf_rs::boundary::envelope::{Envelope, Status};
use axgf_rs::logic::crud::{DeletePolicy, EntityKind};
use axgf_rs::{add_entity, create_bundle, delete_entity, update_entity};
use serde_json::{json, Value};

fn to_str(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}

fn bundle_of(env: &Envelope) -> &Value {
    &env.data["bundle"]
}

fn minimal_person_json(id: Option<&str>, display: &str) -> Value {
    let mut p = json!({
        "type": "person", "axgf_version": "1.0",
        "identity": {"name": {"display": display, "components": []},
                     "gender": {"value": "U"}, "is_living": false}
    });
    if let Some(i) = id {
        p["id"] = json!(i);
    }
    p
}

fn minimal_family_json(id: &str, spouses: &[&str], children: &[&str]) -> Value {
    json!({
        "id": id, "type": "family", "axgf_version": "1.0",
        "union": {
            "type": "marriage",
            "persons": spouses.iter().map(|s| json!({"person_id": s, "role": "spouse"})).collect::<Vec<_>>()
        },
        "children": children.iter().map(|c| json!({"person_id": c, "birth_order": 1})).collect::<Vec<_>>()
    })
}

// ---------- add ----------

#[test]
fn add_generates_uuid_when_id_missing() {
    let b = create_bundle(None).data;
    let entity = minimal_person_json(None, "New");
    let env = add_entity(&to_str(&b), EntityKind::Person, &to_str(&entity));
    assert_eq!(env.status, Status::Ok);
    let id = env.data["id"].as_str().unwrap();
    assert_eq!(id.len(), 36, "generated id should be a UUID: {id}");
    // Version 4 uuid: 15th char (index 14) is '4'.
    assert_eq!(id.chars().nth(14), Some('4'), "expected UUID v4: {id}");
    // Bundle contains the new person.
    assert_eq!(
        bundle_of(&env)["persons"][id]["identity"]["name"]["display"],
        "New"
    );
    // Stats updated.
    assert_eq!(bundle_of(&env)["manifest"]["stats"]["persons"], 1);
    // updated_at was refreshed to some ISO-looking string.
    assert!(bundle_of(&env)["manifest"]["updated_at"]
        .as_str()
        .unwrap()
        .starts_with("20"));
}

#[test]
fn add_respects_provided_id_and_fills_type_and_axgf_version() {
    let b = create_bundle(None).data;
    let id = "550e8400-e29b-41d4-a716-446655440042";
    let entity = json!({
        "id": id,
        "identity": {"name": {"display": "Fixed", "components": []},
                     "gender": {"value": "U"}, "is_living": false}
    });
    let env = add_entity(&to_str(&b), EntityKind::Person, &to_str(&entity));
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["id"], id);
    // The library filled in type and axgf_version.
    assert_eq!(bundle_of(&env)["persons"][id]["type"], "person");
    assert_eq!(bundle_of(&env)["persons"][id]["axgf_version"], "1.0");
}

#[test]
fn add_refuses_duplicate_id() {
    let mut b = create_bundle(None).data;
    let id = "550e8400-e29b-41d4-a716-446655440042";
    b["persons"] = json!({ id: minimal_person_json(Some(id), "Old") });

    let entity = minimal_person_json(Some(id), "Copy");
    let env = add_entity(&to_str(&b), EntityKind::Person, &to_str(&entity));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "ENTITY_ALREADY_EXISTS");
}

#[test]
fn add_emits_warning_but_succeeds_on_schema_deficient_entity() {
    let b = create_bundle(None).data;
    // Person missing required `identity.gender`. Add succeeds with warning.
    let bad = json!({
        "identity": {"name": {"display": "X", "components": []}, "is_living": false}
    });
    let env = add_entity(&to_str(&b), EntityKind::Person, &to_str(&bad));
    assert_eq!(env.status, Status::Ok);
    assert!(env
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "SCHEMA_VALIDATION_FAILED"));
    // Still landed in the bundle.
    assert_eq!(bundle_of(&env)["manifest"]["stats"]["persons"], 1);
}

// ---------- update ----------

#[test]
fn update_replaces_existing_entity() {
    let mut b = create_bundle(None).data;
    let id = "550e8400-e29b-41d4-a716-446655440042";
    b["persons"] = json!({ id: minimal_person_json(Some(id), "Before") });

    let mut updated = minimal_person_json(Some(id), "After");
    updated["identity"]["gender"]["value"] = json!("F");
    let env = update_entity(&to_str(&b), EntityKind::Person, &to_str(&updated));
    assert_eq!(env.status, Status::Ok);
    assert_eq!(
        bundle_of(&env)["persons"][id]["identity"]["name"]["display"],
        "After"
    );
    assert_eq!(
        bundle_of(&env)["persons"][id]["identity"]["gender"]["value"],
        "F"
    );
}

#[test]
fn update_refuses_missing_entity() {
    let b = create_bundle(None).data;
    let entity = minimal_person_json(Some("550e8400-e29b-41d4-a716-446655440099"), "Ghost");
    let env = update_entity(&to_str(&b), EntityKind::Person, &to_str(&entity));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "ENTITY_NOT_FOUND");
}

#[test]
fn update_requires_id() {
    let mut b = create_bundle(None).data;
    let id = "550e8400-e29b-41d4-a716-446655440042";
    b["persons"] = json!({ id: minimal_person_json(Some(id), "X") });
    let no_id = minimal_person_json(None, "No id here");
    let env = update_entity(&to_str(&b), EntityKind::Person, &to_str(&no_id));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "INVALID_BUNDLE_STRUCTURE");
}

// ---------- delete: Reject ----------

#[test]
fn delete_reject_blocks_when_referred_from_family() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({
        p1: minimal_person_json(Some(p1), "A"),
        p2: minimal_person_json(Some(p2), "B"),
    });
    b["families"] = json!({ fam: minimal_family_json(fam, &[p1, p2], &[]) });
    let original = b.clone();

    let env = delete_entity(&to_str(&b), EntityKind::Person, p1, DeletePolicy::Reject);
    assert_eq!(env.status, Status::Error);
    assert_eq!(
        env.diagnostics[0].code.as_str(),
        "DELETE_BLOCKED_BY_REFERENCE"
    );
    assert!(
        env.diagnostics[0].message.contains(fam),
        "should name the referrer"
    );
    // Bundle is unchanged — data is null under error.
    assert!(env.data.is_null());
    // Sanity: original bundle would have had p1.
    assert!(original["persons"].as_object().unwrap().contains_key(p1));
}

#[test]
fn delete_reject_succeeds_when_no_referrers() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    b["persons"] = json!({ p1: minimal_person_json(Some(p1), "Solo") });
    let env = delete_entity(&to_str(&b), EntityKind::Person, p1, DeletePolicy::Reject);
    assert_eq!(env.status, Status::Ok);
    assert!(bundle_of(&env)["persons"].as_object().unwrap().is_empty());
    assert_eq!(bundle_of(&env)["manifest"]["stats"]["persons"], 0);
}

// ---------- delete: Cascade ----------

#[test]
fn delete_cascade_removes_referring_array_items_and_scalar_fields() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let ch = "550e8400-e29b-41d4-a716-446655440003";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    let evt = "aaaa1234-e29b-41d4-a716-446655440100";
    let plc = "aaaa1234-e29b-41d4-a716-446655440099";

    b["persons"] = json!({
        p1: minimal_person_json(Some(p1), "A"),
        p2: minimal_person_json(Some(p2), "B"),
        ch: minimal_person_json(Some(ch), "K"),
    });
    b["families"] = json!({ fam: minimal_family_json(fam, &[p1, p2], &[ch]) });
    // Event references p1 (participant, in an array item) and plc (scalar).
    b["events"] = json!({
        evt: {
            "id": evt, "type": "event", "axgf_version": "1.0",
            "category": "birth", "date": {"value": "1900"},
            "place_id": plc,
            "participants": [
                {"entity_type": "person", "entity_id": p1, "role": "subject"},
                {"entity_type": "person", "entity_id": p2, "role": "witness"}
            ]
        }
    });
    // Place referenced by the event as a scalar `_id`.
    b["places"] = json!({
        plc: {"id": plc, "type": "place", "axgf_version": "1.0",
              "names": [{"lang": "fr", "value": "X"}]}
    });

    // Delete p1 with cascade.
    let env = delete_entity(&to_str(&b), EntityKind::Person, p1, DeletePolicy::Cascade);
    assert_eq!(env.status, Status::Ok, "diags: {:?}", env.diagnostics);
    let out = bundle_of(&env);

    // p1 gone from persons.
    assert!(!out["persons"].as_object().unwrap().contains_key(p1));
    // Family's union.persons list lost the p1 entry, kept p2.
    let union_persons = out["families"][fam]["union"]["persons"].as_array().unwrap();
    assert_eq!(union_persons.len(), 1);
    assert_eq!(union_persons[0]["person_id"], p2);
    // Event participants lost the p1 entry, kept p2.
    let parts = out["events"][evt]["participants"].as_array().unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["entity_id"], p2);

    // Delete the place with cascade — scalar `place_id` field on the
    // event should be REMOVED (not just nulled).
    let env2 = delete_entity(&to_str(out), EntityKind::Place, plc, DeletePolicy::Cascade);
    assert_eq!(env2.status, Status::Ok);
    let out2 = bundle_of(&env2);
    assert!(
        out2["events"][evt]
            .as_object()
            .unwrap()
            .get("place_id")
            .is_none(),
        "cascade should REMOVE scalar place_id, not null it. Got: {:?}",
        out2["events"][evt]
    );
    // And place is gone.
    assert!(!out2["places"].as_object().unwrap().contains_key(plc));
}

// ---------- delete: Orphan ----------

#[test]
fn delete_orphan_nulls_scalar_and_keeps_array_shape() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    let evt = "aaaa1234-e29b-41d4-a716-446655440100";
    let plc = "aaaa1234-e29b-41d4-a716-446655440099";

    b["persons"] = json!({
        p1: minimal_person_json(Some(p1), "A"),
        p2: minimal_person_json(Some(p2), "B"),
    });
    b["families"] = json!({ fam: minimal_family_json(fam, &[p1, p2], &[]) });
    b["events"] = json!({
        evt: {
            "id": evt, "type": "event", "axgf_version": "1.0",
            "category": "birth", "date": {"value": "1900"},
            "place_id": plc,
            "participants": [
                {"entity_type": "person", "entity_id": p1, "role": "subject"}
            ]
        }
    });
    b["places"] = json!({
        plc: {"id": plc, "type": "place", "axgf_version": "1.0",
              "names": [{"lang": "fr", "value": "X"}]}
    });

    // Orphan-delete p1: participant array item survives with entity_id = null.
    let env = delete_entity(&to_str(&b), EntityKind::Person, p1, DeletePolicy::Orphan);
    assert_eq!(env.status, Status::Ok);
    let out = bundle_of(&env);
    let parts = out["events"][evt]["participants"].as_array().unwrap();
    assert_eq!(parts.len(), 1, "orphan should keep array shape");
    assert!(parts[0]["entity_id"].is_null(), "entity_id must be nulled");
    assert_eq!(parts[0]["role"], "subject", "surrounding fields preserved");

    // Same for family.union.persons — entry survives with person_id nulled.
    let uni = out["families"][fam]["union"]["persons"].as_array().unwrap();
    assert_eq!(uni.len(), 2);
    let nulled = uni.iter().filter(|e| e["person_id"].is_null()).count();
    assert_eq!(
        nulled, 1,
        "one entry should be nulled; the p2 entry stays intact"
    );

    // Orphan-delete plc: scalar place_id field becomes null, key preserved.
    let env2 = delete_entity(&to_str(out), EntityKind::Place, plc, DeletePolicy::Orphan);
    assert_eq!(env2.status, Status::Ok);
    let ev = &bundle_of(&env2)["events"][evt];
    assert!(
        ev.as_object().unwrap().contains_key("place_id"),
        "orphan preserves the key; only the value is null"
    );
    assert!(ev["place_id"].is_null());
}

// ---------- misc ----------

#[test]
fn delete_missing_id_returns_entity_not_found() {
    let b = create_bundle(None).data;
    let env = delete_entity(
        &to_str(&b),
        EntityKind::Person,
        "550e8400-e29b-41d4-a716-446655440099",
        DeletePolicy::Cascade,
    );
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "ENTITY_NOT_FOUND");
}

#[test]
fn crud_rejects_unsupported_spec_version() {
    let mut b = create_bundle(None).data;
    b["manifest"]["axgf"] = json!("9.9");
    let e = add_entity(
        &to_str(&b),
        EntityKind::Person,
        &to_str(&minimal_person_json(None, "X")),
    );
    assert_eq!(e.status, Status::Error);
    assert_eq!(e.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
}
