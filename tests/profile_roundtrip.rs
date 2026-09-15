// SPDX-License-Identifier: Apache-2.0
//! AXGF 1.1 round-trips through the typed model, group by group.
//!
//! Each group is exercised twice: once with every attribute and every field
//! filled from the registry — so a typed struct that has dropped or misspelt
//! a field loses data and fails here — and once with the worked example the
//! specification prints for it, so the tests also read like the format.

#[path = "common/profile.rs"]
mod fixtures;

use axgf_rs::model::family::Family;
use axgf_rs::model::link::Link;
use axgf_rs::model::manifest::Manifest;
use axgf_rs::model::occupation::Occupation;
use axgf_rs::model::person::Person;
use axgf_rs::model::profile::registry;
use axgf_rs::{add_entity, create_bundle, export_bundle, import_bundle, EntityKind};
use fixtures::{assert_contains, person_with_group, Fill};
use serde_json::{json, Value};

fn round_trip(person: &Value) -> Value {
    let typed: Person = serde_json::from_value(person.clone())
        .unwrap_or_else(|e| panic!("did not parse: {e}\n{person:#}"));
    serde_json::to_value(&typed).unwrap()
}

/// Every attribute of the group, every field filled, survives the typed model.
fn group_survives_the_typed_model(group: &str) {
    let person = person_with_group(group, Fill::Everything);
    let back = round_trip(&person);
    assert_contains(&person, &back, group);
}

macro_rules! group_tests {
    ($($name:ident => $group:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                group_survives_the_typed_model($group);
            }
        )*
    };
}

group_tests! {
    identity_and_civil_status_round_trips => "identity",
    morphology_round_trips => "morphology",
    biometrics_round_trips => "biometrics",
    health_round_trips => "health",
    genomics_round_trips => "genomics",
    death_round_trips => "death",
    residence_and_nationality_round_trips => "residence",
    education_and_work_round_trips => "education",
    military_and_honours_round_trips => "military",
    legal_round_trips => "legal",
    belief_and_affiliation_round_trips => "belief",
    personality_and_behaviour_round_trips => "personality",
    digital_legacy_round_trips => "digital_legacy",
}

#[test]
fn every_group_with_attributes_has_a_round_trip_test() {
    // The macro above names groups by hand; this is what stops a fifteenth
    // group being added without one.
    let tested = [
        "identity",
        "morphology",
        "biometrics",
        "health",
        "genomics",
        "death",
        "residence",
        "education",
        "military",
        "legal",
        "belief",
        "personality",
        "digital_legacy",
    ];
    for g in registry::GROUPS {
        if !g.attributes.is_empty() {
            assert!(tested.contains(&g.key), "{} has no round-trip test", g.key);
        }
    }
}

#[test]
fn relationships_round_trip_through_family_and_link() {
    // The fourteenth group has no person attributes: it is lineage on a
    // Family's children and relation on a Link.
    let family = json!({
        "id": "aaaa1234-e29b-41d4-a716-446655440001", "type": "family", "axgf_version": "1.1",
        "union": {"type": "marriage", "persons": [
            {"person_id": "11111111-1111-4111-8111-111111111111", "role": "spouse"}]},
        "children": [
            {"person_id": "33333333-3333-4333-8333-333333333333", "birth_order": 1, "lineage": "step"},
            {"person_id": "44444444-4444-4444-8444-444444444444", "birth_order": 2, "lineage": "adoptive"}
        ]
    });
    let f: Family = serde_json::from_value(family.clone()).unwrap();
    assert_eq!(f.children[0].lineage.as_deref(), Some("step"));
    assert_contains(&family, &serde_json::to_value(&f).unwrap(), "family");

    let link = json!({
        "id": "bbbb1234-e29b-41d4-a716-446655440002", "type": "link", "axgf_version": "1.1",
        "from": {"entity_type": "person", "entity_id": "55555555-5555-4555-8555-555555555555"},
        "to": {"entity_type": "person", "entity_id": "44444444-4444-4444-8444-444444444444"},
        "label": "matka chrzestna", "label_reverse": "chrześniak",
        "category": "spiritual", "relation": "godparent", "confidence": 0.9
    });
    let l: Link = serde_json::from_value(link.clone()).unwrap();
    assert_eq!(l.relation.as_deref(), Some("godparent"));
    assert_contains(&link, &serde_json::to_value(&l).unwrap(), "link");

    let occupation = json!({
        "id": "6fa459ea-ee8a-4ca4-894e-db77e160355e", "type": "occupation", "axgf_version": "1.1",
        "person_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
        "title": "Nauczyciel", "position": "Kierownik szkoły"
    });
    let o: Occupation = serde_json::from_value(occupation.clone()).unwrap();
    assert_eq!(o.position.as_deref(), Some("Kierownik szkoły"));
    assert_contains(
        &occupation,
        &serde_json::to_value(&o).unwrap(),
        "occupation",
    );
}

#[test]
fn a_manifest_says_which_classes_were_withheld() {
    let manifest = json!({
        "axgf": "1.1", "created_at": "2026-09-15T09:00:00Z", "stats": {"persons": 1},
        "privacy": {"contains_living_persons": true, "withheld_classes": ["health", "genomics"]}
    });
    let m: Manifest = serde_json::from_value(manifest.clone()).unwrap();
    assert_eq!(
        m.privacy.as_ref().unwrap().withheld_classes,
        ["health", "genomics"]
    );
    assert_contains(&manifest, &serde_json::to_value(&m).unwrap(), "manifest");
}

#[test]
fn the_specifications_worked_example_round_trips() {
    // SPEC_1.1 §8, verbatim.
    let example: Value = serde_json::from_str(include_str!("fixtures/person-profile-1.1.json"))
        .expect("the example is JSON");
    let back = round_trip(&example);
    assert_contains(&example, &back, "example");
    let typed: Person = serde_json::from_value(example).unwrap();
    let health = typed.health.as_ref().expect("health");
    assert_eq!(health.blood_group.as_ref().unwrap().value, "A");
    assert_eq!(health.conditions.len(), 2);
    let morphology = typed.morphology.as_ref().expect("morphology");
    assert_eq!(morphology.height.len(), 2, "two heights, both kept");
    assert_eq!(morphology.height[0].value.to_string(), "158");
    assert_eq!(typed.death.as_ref().unwrap().causes.len(), 2);
    assert_eq!(
        typed
            .identity
            .class_visibility
            .get("health")
            .map(String::as_str),
        Some("contributors")
    );
}

#[test]
fn a_number_keeps_the_spelling_it_was_written_in() {
    // 158 and 158.0 are different JSON, and a typed f64 would turn the first
    // into the second. The model keeps numbers as serde_json::Number.
    let p = json!({
        "id": fixtures::UUID_A, "type": "person", "axgf_version": "1.1",
        "identity": {"name": {"display": "N", "components": []}, "gender": {"value": "U"}, "is_living": false},
        "morphology": {"height": [{"value": 158}, {"value": 158.5}], "weight": [{"value": 61.0}]}
    });
    let back = round_trip(&p);
    assert_eq!(back["morphology"]["height"][0]["value"].to_string(), "158");
    assert_eq!(
        back["morphology"]["height"][1]["value"].to_string(),
        "158.5"
    );
    assert_eq!(back["morphology"]["weight"][0]["value"].to_string(), "61.0");
}

#[test]
fn unknown_fields_survive_inside_claims_values_and_blocks() {
    // 1.0 P9, at every level 1.1 adds.
    let p = json!({
        "id": fixtures::UUID_A, "type": "person", "axgf_version": "1.1",
        "identity": {"name": {"display": "U", "components": []}, "gender": {"value": "U"},
                     "is_living": false, "future_identity_field": 1},
        "health": {
            "future_attribute": [{"value": "kept"}],
            "blood_pressure": [{
                "value": {"systolic": 120, "diastolic": 80, "future_value_field": "kept"},
                "future_claim_field": {"nested": true}
            }]
        },
        "death": {"causes": [{"value": {"description": "x", "future": 2}}], "future_death_field": 3}
    });
    let back = round_trip(&p);
    assert_contains(&p, &back, "unknown");
}

#[test]
fn a_1_1_person_survives_the_bundle_round_trip() {
    // Through the boundary this time: add, export to ZIP, import.
    let bundle = create_bundle(Some("Round trip")).data.to_string();
    let person = person_with_group("health", Fill::Everything);
    let added = add_entity(&bundle, EntityKind::Person, &person.to_string());
    let zip = export_bundle(&added.data["bundle"].to_string());
    let bytes = base64_decode(zip.data["zip_base64"].as_str().expect("zip"));
    let imported = import_bundle(&bytes);
    let back = &imported.data["persons"][fixtures::UUID_A];
    assert_contains(&person, back, "bundle");
    assert_eq!(imported.data["manifest"]["axgf"], "1.1");
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}
