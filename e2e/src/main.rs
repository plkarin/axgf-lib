//! axgf-rs end-to-end test suite.
//! Run: cargo run
//! All tests must print [PASSED]. Any [FAILED] line is a regression.

use axgf_rs::{
    add_entity, convert_gedcom, create_bundle, deduplicate, delete_entity,
    export_bundle, import_bundle, inspect, update_entity, validate,
};
use axgf_rs::{DeletePolicy, EntityKind};
use base64::Engine as _;
use serde_json::{json, Value};

// -- helpers ------------------------------------------------------------------

fn parse(env_json: &str) -> Value {
    serde_json::from_str(env_json).expect("envelope is not valid JSON")
}

fn ok(env: &Value) -> bool {
    env["status"].as_str() == Some("ok")
}

fn data(env: &Value) -> Value {
    let d = &env["data"];
    // add_entity / update_entity / delete_entity wrap the result as
    // {"id": "...", "bundle": {...}} — unwrap to the flat bundle directly.
    if d.get("bundle").is_some() {
        d["bundle"].clone()
    } else {
        d.clone()
    }
}

fn diag_codes(env: &Value) -> Vec<String> {
    let empty = vec![];
    env["diagnostics"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter_map(|d| d["code"].as_str().map(|s| s.to_string()))
        .collect()
}

fn pass(name: &str) {
    println!("[PASSED] {name}");
}

fn fail(name: &str, reason: &str) {
    println!("[FAILED] {name} -- {reason}");
    std::process::exit(1);
}

fn check(name: &str, condition: bool, reason: &str) {
    if condition { pass(name) } else { fail(name, reason) }
}

// -- test cases ---------------------------------------------------------------

fn test_01_create_bundle() -> String {
    let env = parse(&create_bundle(Some("Pierre-Leonard Family")).to_json());
    check("T01 create_bundle returns ok", ok(&env), "status != ok");
    let bundle = data(&env).to_string();
    let b: Value = serde_json::from_str(&bundle).unwrap();
    check("T01 manifest.axgf == 1.0",
        b["manifest"]["axgf"].as_str() == Some("1.0"),
        "manifest.axgf mismatch");
    check("T01 manifest.family.name present",
        b["manifest"]["family"]["name"].as_str() == Some("Pierre-Leonard Family"),
        "family name missing");
    bundle
}

fn test_02_inspect(bundle: &str) {
    let env = parse(&inspect(bundle).to_json());
    check("T02 inspect returns ok", ok(&env), "status != ok");
    let d = data(&env);
    check("T02 stats.persons == 0",
        d["stats"]["persons"].as_u64() == Some(0),
        "persons count mismatch");
}

fn test_03_validate_empty(bundle: &str) {
    let env = parse(&validate(bundle).to_json());
    check("T03 validate empty bundle is ok", ok(&env), "status != ok");
    check("T03 no diagnostics on empty bundle",
        diag_codes(&env).is_empty(),
        "unexpected diagnostics");
}

fn test_04_add_person(bundle: &str) -> (String, String) {
    let person = json!({
        "type": "person",
        "axgf_version": "1.0",
        "identity": {
            "name": {
                "display": "Jean Pierre-Leonard",
                "components": [
                    {"type": "given_name",  "value": "Jean",           "order": 1},
                    {"type": "family_name", "value": "Pierre-Leonard", "order": 2}
                ]
            },
            "gender": {"value": "M"},
            "is_living": false,
            "visibility": "members"
        },
        "birth": {
            "date": {"value": "1923-04-12", "calendar": "gregorian",
                     "precision": "exact", "circa": false, "confidence": 0.98}
        },
        "death": {
            "date": {"value": "1987-03-03", "calendar": "gregorian",
                     "precision": "exact", "circa": false, "confidence": 0.99}
        },
        "bio": "Schoolteacher, village school founder."
    });
    let env = parse(&add_entity(bundle, EntityKind::Person,
                                &person.to_string()).to_json());
    check("T04 add person returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    let persons = b["persons"].as_object().unwrap();
    check("T04 bundle contains 1 person", persons.len() == 1, "person count mismatch");
    let person_id = persons.keys().next().unwrap().clone();
    (data(&env).to_string(), person_id)
}

fn test_05_add_uuid_generated(bundle: &str) -> String {
    let person = json!({
        "type": "person",
        "axgf_version": "1.0",
        "identity": {
            "name": {
                "display": "Elise Bernard",
                "components": [
                    {"type": "given_name",  "value": "Elise",  "order": 1},
                    {"type": "family_name", "value": "Bernard","order": 2}
                ]
            },
            "gender": {"value": "F"},
            "is_living": false,
            "visibility": "members"
        },
        "birth": {
            "date": {"value": "1925", "calendar": "gregorian",
                     "precision": "year", "circa": true, "confidence": 0.6}
        }
    });
    let env = parse(&add_entity(bundle, EntityKind::Person,
                                &person.to_string()).to_json());
    check("T05 add person without id returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    let persons = b["persons"].as_object().unwrap();
    check("T05 bundle now has 2 persons", persons.len() == 2, "person count mismatch");
    let all_uuid = persons.keys().all(|id| {
        let parts: Vec<&str> = id.split('-').collect();
        parts.len() == 5 && parts[2].starts_with('4')
    });
    check("T05 auto-generated id is UUID v4", all_uuid, "malformed UUID");
    data(&env).to_string()
}

fn test_06_add_family(bundle: &str, jean_id: &str, elise_id: &str) -> (String, String) {
    let family = json!({
        "type": "family",
        "axgf_version": "1.0",
        "name": "Family Jean x Elise",
        "union": {
            "type": "marriage",
            "status": "ended_by_death",
            "persons": [
                {"person_id": jean_id,  "role": "spouse"},
                {"person_id": elise_id, "role": "spouse"}
            ],
            "start": {
                "date": {"value": "1948-06-15", "calendar": "gregorian",
                         "precision": "exact", "circa": false, "confidence": 0.99}
            },
            "confidence": 0.99
        },
        "children": []
    });
    let env = parse(&add_entity(bundle, EntityKind::Family,
                                &family.to_string()).to_json());
    check("T06 add family returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    check("T06 bundle has 1 family",
        b["families"].as_object().map(|m| m.len()) == Some(1),
        "family count mismatch");
    let family_id = b["families"].as_object().unwrap()
        .keys().next().unwrap().clone();
    (data(&env).to_string(), family_id)
}

fn test_07_add_event(bundle: &str, jean_id: &str, elise_id: &str,
                     family_id: &str) -> (String, String) {
    let event = json!({
        "type": "event",
        "axgf_version": "1.0",
        "category": "marriage",
        "subcategory": "civil",
        "date": {"value": "1948-06-15", "calendar": "gregorian",
                 "precision": "exact", "circa": false, "confidence": 0.99},
        "participants": [
            {"entity_type": "person", "entity_id": jean_id,
             "role": "spouse_1", "confidence": 0.99},
            {"entity_type": "person", "entity_id": elise_id,
             "role": "spouse_2", "confidence": 0.99},
            {"entity_type": "family", "entity_id": family_id,
             "role": "created",   "confidence": 0.99}
        ],
        "description": "Civil marriage, Paris 14th"
    });
    let env = parse(&add_entity(bundle, EntityKind::Event,
                                &event.to_string()).to_json());
    check("T07 add marriage event returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    check("T07 bundle has 1 event",
        b["events"].as_object().map(|m| m.len()) == Some(1),
        "event count mismatch");
    let event_id = b["events"].as_object().unwrap()
        .keys().next().unwrap().clone();
    (data(&env).to_string(), event_id)
}

fn test_08_add_link(bundle: &str, jean_id: &str) -> String {
    let jules = json!({
        "type": "person",
        "axgf_version": "1.0",
        "identity": {
            "name": {"display": "Jules Martin",
                     "components": [
                         {"type": "given_name",  "value": "Jules",  "order": 1},
                         {"type": "family_name", "value": "Martin", "order": 2}
                     ]},
            "gender": {"value": "M"},
            "is_living": false,
            "visibility": "members"
        }
    });
    let env_j = parse(&add_entity(bundle, EntityKind::Person,
                                  &jules.to_string()).to_json());
    check("T08 add Jules returns ok", ok(&env_j), "status != ok");
    let b_j: Value = serde_json::from_str(&data(&env_j).to_string()).unwrap();
    let jules_id = b_j["persons"].as_object().unwrap()
        .iter()
        .find(|(_, v)| v["identity"]["name"]["display"].as_str() == Some("Jules Martin"))
        .map(|(k, _)| k.clone()).unwrap();

    let link = json!({
        "type": "link",
        "axgf_version": "1.0",
        "from": {"entity_type": "person", "entity_id": jean_id},
        "to":   {"entity_type": "person", "entity_id": jules_id},
        "label": "godfather",
        "label_reverse": "godchild",
        "category": "spiritual",
        "bidirectional": false,
        "confidence": 0.85
    });
    let bundle2 = data(&env_j).to_string();
    let env_l = parse(&add_entity(&bundle2, EntityKind::Link,
                                  &link.to_string()).to_json());
    check("T08 add link (godfather) returns ok", ok(&env_l), "status != ok");
    let b_l: Value = serde_json::from_str(&data(&env_l).to_string()).unwrap();
    check("T08 bundle has 1 link",
        b_l["links"].as_object().map(|m| m.len()) == Some(1),
        "link count mismatch");
    data(&env_l).to_string()
}

fn test_09_add_occupation(bundle: &str, jean_id: &str) -> String {
    let occ = json!({
        "type": "occupation",
        "axgf_version": "1.0",
        "person_id": jean_id,
        "title": "Schoolteacher",
        "title_latin": "Primary school teacher",
        "valid_from": {
            "date": {"value": "1948", "precision": "year", "circa": false}
        },
        "valid_until": {
            "date": {"value": "1978", "precision": "year", "circa": false}
        },
        "confidence": 0.90
    });
    let env = parse(&add_entity(bundle, EntityKind::Occupation,
                                &occ.to_string()).to_json());
    check("T09 add occupation returns ok", ok(&env), "status != ok");
    data(&env).to_string()
}

fn test_10_add_source_and_place(bundle: &str) -> String {
    let place = json!({
        "type": "place",
        "axgf_version": "1.0",
        "names": [{"lang": "en", "value": "Saint-Denis, Reunion",
                   "is_primary": true}],
        "place_type": "city",
        "country_current": "FR",
        "coordinates": {"lat": -20.8823, "lon": 55.4504,
                        "precision": "city_center"},
        "country_history": [
            {"country": "FR", "from": null, "until": null}
        ]
    });
    let env_p = parse(&add_entity(bundle, EntityKind::Place,
                                  &place.to_string()).to_json());
    check("T10 add place returns ok", ok(&env_p), "status != ok");

    let source = json!({
        "type": "source",
        "axgf_version": "1.0",
        "title": "Birth certificate no.47 - Jean Pierre-Leonard 1923",
        "source_type": "birth_certificate",
        "reliability": "primary",
        "confidence": 0.98,
        "status": "verified",
        "repository": {
            "name": "Departmental Archives of Reunion",
            "location": "Saint-Denis, Reunion"
        }
    });
    let b_p = data(&env_p).to_string();
    let env_s = parse(&add_entity(&b_p, EntityKind::Source,
                                  &source.to_string()).to_json());
    check("T10 add source returns ok", ok(&env_s), "status != ok");
    let b_s: Value = serde_json::from_str(&data(&env_s).to_string()).unwrap();
    check("T10 bundle has 1 place and 1 source",
        b_s["places"].as_object().map(|m| m.len()) == Some(1) &&
        b_s["sources"].as_object().map(|m| m.len()) == Some(1),
        "place or source count mismatch");
    data(&env_s).to_string()
}

fn test_11_update_entity(bundle: &str, jean_id: &str) -> String {
    let b: Value = serde_json::from_str(bundle).unwrap();
    let mut jean = b["persons"][jean_id].clone();
    jean["id"] = json!(jean_id);
    jean["bio"] = json!("Schoolteacher, village school founder. \
                         Decorated with the Order of Merit in 1972.");
    let env = parse(&update_entity(bundle, EntityKind::Person,
                                   &jean.to_string()).to_json());
    check("T11 update_entity returns ok", ok(&env), "status != ok");
    let b2: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    check("T11 bio was updated",
        b2["persons"][jean_id]["bio"].as_str()
            .map(|s| s.contains("Merit")) == Some(true),
        "bio not updated");
    data(&env).to_string()
}

fn test_12_update_entity_not_found(bundle: &str) {
    let ghost = json!({
        "id": "00000000-0000-4000-8000-000000000000",
        "type": "person",
        "axgf_version": "1.0",
        "identity": {
            "name": {"display": "Ghost", "components": []},
            "gender": {"value": "U"},
            "is_living": false,
            "visibility": "members"
        }
    });
    let env = parse(&update_entity(bundle, EntityKind::Person,
                                   &ghost.to_string()).to_json());
    check("T12 update non-existent entity returns error", !ok(&env),
        "expected error, got ok");
    check("T12 diagnostic code is ENTITY_NOT_FOUND",
        diag_codes(&env).iter().any(|c| c == "ENTITY_NOT_FOUND"),
        "wrong diagnostic code");
}

fn test_13_delete_reject_policy(bundle: &str, jean_id: &str) {
    let env = parse(&delete_entity(bundle, EntityKind::Person,
                                   jean_id, DeletePolicy::Reject).to_json());
    check("T13 delete referenced person with Reject blocks", !ok(&env),
        "expected error, got ok");
    check("T13 diagnostic is DELETE_BLOCKED_BY_REFERENCE",
        diag_codes(&env).iter().any(|c| c == "DELETE_BLOCKED_BY_REFERENCE"),
        "wrong diagnostic code");
}

fn test_14_delete_cascade_policy(bundle: &str, jean_id: &str) -> String {
    let env = parse(&delete_entity(bundle, EntityKind::Person,
                                   jean_id, DeletePolicy::Cascade).to_json());
    check("T14 delete with Cascade returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    check("T14 Jean removed from persons",
        b["persons"].get(jean_id).is_none(),
        "Jean still present");
    let families = b["families"].as_object().unwrap();
    let no_jean = families.values().all(|f| {
        f["union"]["persons"].as_array().unwrap_or(&vec![])
            .iter().all(|p| p["person_id"].as_str() != Some(jean_id))
    });
    check("T14 Jean removed from all family unions under Cascade", no_jean,
        "Jean still referenced in a family");
    data(&env).to_string()
}

fn test_15_validate_full_bundle(bundle: &str) {
    let env = parse(&validate(bundle).to_json());
    check("T15 validate populated bundle is ok", ok(&env), "status != ok");
    let empty = vec![];
    let errors: Vec<_> = env["diagnostics"].as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|d| d["severity"].as_str() == Some("error"))
        .collect();
    check("T15 no error-severity diagnostics", errors.is_empty(),
        "unexpected errors in populated bundle");
}

fn test_16_unsupported_spec_version() {
    let bad = json!({
        "manifest": {"axgf": "99.0"},
        "persons": {}, "families": {}, "events": {},
        "links": {}, "occupations": {}, "sources": {},
        "places": {}, "documents": {}
    });
    let env = parse(&validate(&bad.to_string()).to_json());
    check("T16 unknown spec version is rejected", !ok(&env),
        "expected error, got ok");
    check("T16 diagnostic is UNSUPPORTED_SPEC_VERSION",
        diag_codes(&env).iter().any(|c| c == "UNSUPPORTED_SPEC_VERSION"),
        "wrong diagnostic code");
}

fn test_17_export_and_reimport(bundle: &str) {
    let env_exp = parse(&export_bundle(bundle).to_json());
    check("T17 export_bundle returns ok", ok(&env_exp), "status != ok");
    let zip_b64 = env_exp["data"]["zip_base64"].as_str()
        .expect("zip_base64 missing from export data");
    let zip_bytes = base64::engine::general_purpose::STANDARD
        .decode(zip_b64)
        .expect("base64 decode failed");
    check("T17 ZIP bytes non-empty", !zip_bytes.is_empty(), "empty ZIP");
    let env_imp = parse(&import_bundle(&zip_bytes).to_json());
    check("T17 import_bundle of exported ZIP returns ok", ok(&env_imp),
        "status != ok");
    let orig: Value = serde_json::from_str(bundle).unwrap();
    let round: Value = serde_json::from_str(&data(&env_imp).to_string()).unwrap();
    check("T17 round-trip persons count matches",
        orig["persons"].as_object().map(|m| m.len()) ==
        round["persons"].as_object().map(|m| m.len()),
        "persons count mismatch after round-trip");
    check("T17 round-trip families count matches",
        orig["families"].as_object().map(|m| m.len()) ==
        round["families"].as_object().map(|m| m.len()),
        "families count mismatch after round-trip");
}

fn test_18_deduplicate() {
    let bundle0 = data(&parse(&create_bundle(None).to_json())).to_string();

    let alice = json!({
        "type": "person", "axgf_version": "1.0",
        "id": "aaaaaaaa-0000-4000-8000-000000000001",
        "identity": {
            "name": { "display": "Alice", "components": [
                { "type": "given_name", "value": "Alice", "order": 1 }
            ]},
            "gender": { "value": "F" },
            "is_living": false,
            "visibility": "members"
        }
    });
    let bob = json!({
        "type": "person", "axgf_version": "1.0",
        "id": "bbbbbbbb-0000-4000-8000-000000000002",
        "identity": {
            "name": { "display": "Bob", "components": [
                { "type": "given_name", "value": "Bob", "order": 1 }
            ]},
            "gender": { "value": "M" },
            "is_living": false,
            "visibility": "members"
        }
    });
    let b1 = data(&parse(&add_entity(&bundle0, EntityKind::Person,
        &alice.to_string()).to_json())).to_string();
    let b2 = data(&parse(&add_entity(&b1, EntityKind::Person,
        &bob.to_string()).to_json())).to_string();

    let fam_a = json!({
        "type": "family", "axgf_version": "1.0",
        "id": "faaaaaaa-0000-4000-8000-000000000001",
        "union": {
            "type": "marriage",
            "persons": [
                { "person_id": "aaaaaaaa-0000-4000-8000-000000000001", "role": "spouse" },
                { "person_id": "bbbbbbbb-0000-4000-8000-000000000002", "role": "spouse" }
            ],
            "confidence": 0.9
        },
        "children": []
    });
    let fam_b = json!({
        "type": "family", "axgf_version": "1.0",
        "id": "fbbbbbbb-0000-4000-8000-000000000002",
        "union": {
            "type": "marriage",
            "persons": [
                { "person_id": "aaaaaaaa-0000-4000-8000-000000000001", "role": "spouse" },
                { "person_id": "bbbbbbbb-0000-4000-8000-000000000002", "role": "spouse" }
            ],
            "confidence": 0.9
        },
        "children": []
    });
    let b3 = data(&parse(&add_entity(&b2, EntityKind::Family, &fam_a.to_string()).to_json())).to_string();
    let b4 = data(&parse(&add_entity(&b3, EntityKind::Family, &fam_b.to_string()).to_json())).to_string();

    let bv: Value = serde_json::from_str(&b4).unwrap();
    check("T18 setup: bundle has 2 duplicate families",
        bv["families"].as_object().map(|m| m.len()) == Some(2),
        "expected 2 families");
    let env = parse(&deduplicate(&b4).to_json());
    check("T18 deduplicate returns ok", ok(&env), "status != ok");
    let after: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    check("T18 duplicate family merged into 1",
        after["families"].as_object().map(|m| m.len()) == Some(1),
        "expected 1 family after dedup");
}

fn test_19_gedcom_conversion() {
    let ged = b"0 HEAD\n1 GEDC\n2 VERS 5.5.1\n1 CHAR UTF-8\n\
0 @I1@ INDI\n1 NAME Jean /Pierre-Leonard/\n2 GIVN Jean\n2 SURN Pierre-Leonard\n\
1 NAME Jean-Baptiste /Pierre-Leonard/\n\
1 SEX M\n1 BIRT\n2 DATE 12 APR 1923\n2 PLAC Saint-Denis, Reunion\n\
1 DEAT\n2 DATE ABT 1987\n1 OCCU Schoolteacher\n\
1 NOTE Village school founder.\n\
0 @I2@ INDI\n1 NAME Elise /Bernard/\n1 SEX F\n1 BIRT\n2 DATE ABT 1925\n\
0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 MARR\n2 DATE 15 JUN 1948\n\
0 TRLR\n";
    let env = parse(&convert_gedcom(ged, 0.8, "en").to_json());
    check("T19 convert_gedcom returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    check("T19 persons == 2",
        b["persons"].as_object().map(|m| m.len()) == Some(2),
        "person count mismatch");
    check("T19 families == 1",
        b["families"].as_object().map(|m| m.len()) == Some(1),
        "family count mismatch");
    check("T19 events >= 1 (marriage event created)",
        b["events"].as_object().map(|m| m.len()).unwrap_or(0) >= 1,
        "no marriage event");
    check("T19 occupations == 1",
        b["occupations"].as_object().map(|m| m.len()) == Some(1),
        "occupation count mismatch");
    check("T19 places >= 1",
        b["places"].as_object().map(|m| m.len()).unwrap_or(0) >= 1,
        "no places");
    let elise = b["persons"].as_object().unwrap().values()
        .find(|p| p["identity"]["name"]["display"].as_str() == Some("Elise Bernard"));
    check("T19 ABT date produces circa=true",
        elise.and_then(|e| e["birth"]["date"]["circa"].as_bool()) == Some(true),
        "circa not set for ABT date");
    let env_v = parse(&validate(&data(&env).to_string()).to_json());
    check("T19 converted bundle passes validation", ok(&env_v),
        "converted bundle invalid");
    let real_ged = "/home/cbrain/axgf-tools/tree2-fixed.ged";
    if std::path::Path::new(real_ged).exists() {
        let bytes = std::fs::read(real_ged).unwrap();
        let env_r = parse(&convert_gedcom(&bytes, 0.8, "pl").to_json());
        check("T19b real-world tree2-fixed.ged converts ok", ok(&env_r),
            "real-world conversion failed");
        let b_r: Value = serde_json::from_str(&data(&env_r).to_string()).unwrap();
        check("T19b real-world: persons >= 760",
            b_r["persons"].as_object().map(|m| m.len()).unwrap_or(0) >= 760,
            "too few persons");
    } else {
        println!("[SKIPPED] T19b real-world GEDCOM (file not found at {real_ged})");
    }
}

fn test_20_forward_compatibility() {
    let person = json!({
        "id": "cccccccc-0000-4000-8000-000000000001",
        "type": "person",
        "axgf_version": "1.0",
        "identity": {
            "name": {"display": "Future Person", "components": []},
            "gender": {"value": "U"},
            "is_living": true,
            "visibility": "public"
        },
        "x_axgf_future_field": "this field does not exist in V1 spec"
    });
    let bundle0 = data(&parse(&create_bundle(None).to_json())).to_string();
    let env = parse(&add_entity(&bundle0, EntityKind::Person,
                                &person.to_string()).to_json());
    check("T20 add entity with unknown field returns ok", ok(&env), "status != ok");
    let b: Value = serde_json::from_str(&data(&env).to_string()).unwrap();
    let id = "cccccccc-0000-4000-8000-000000000001";
    check("T20 unknown field preserved after round-trip",
        b["persons"][id]["x_axgf_future_field"].as_str()
            == Some("this field does not exist in V1 spec"),
        "unknown field lost");
}

// -- main ---------------------------------------------------------------------

fn main() {
    println!("axgf-rs end-to-end test suite\n");

    println!("---- Lifecycle ------------------------------------------------");
    let b01 = test_01_create_bundle();
    test_02_inspect(&b01);
    test_03_validate_empty(&b01);

    println!("\n---- CRUD - build a family from scratch -----------------------");
    let (b04, jean_id) = test_04_add_person(&b01);
    let b05 = test_05_add_uuid_generated(&b04);
    let bv05: Value = serde_json::from_str(&b05).unwrap();
    let elise_id = bv05["persons"].as_object().unwrap()
        .iter()
        .find(|(_, v)| v["identity"]["name"]["display"].as_str()
              == Some("Elise Bernard"))
        .map(|(k, _)| k.clone()).unwrap();
    let (b06, family_id) = test_06_add_family(&b05, &jean_id, &elise_id);
    let (b07, _) = test_07_add_event(&b06, &jean_id, &elise_id, &family_id);
    let b08 = test_08_add_link(&b07, &jean_id);
    let b09 = test_09_add_occupation(&b08, &jean_id);
    let b10 = test_10_add_source_and_place(&b09);

    println!("\n---- CRUD - update and delete ---------------------------------");
    let b11 = test_11_update_entity(&b10, &jean_id);
    test_12_update_entity_not_found(&b11);
    test_13_delete_reject_policy(&b11, &jean_id);
    let b14 = test_14_delete_cascade_policy(&b11, &jean_id);

    println!("\n---- Validation -----------------------------------------------");
    test_15_validate_full_bundle(&b14);
    test_16_unsupported_spec_version();

    println!("\n---- Export / Import round-trip --------------------------------");
    test_17_export_and_reimport(&b14);

    println!("\n---- Deduplication --------------------------------------------");
    test_18_deduplicate();

    println!("\n---- GEDCOM conversion ----------------------------------------");
    test_19_gedcom_conversion();

    println!("\n---- Forward compatibility ------------------------------------");
    test_20_forward_compatibility();

    println!("\n==============================================================");
    println!(" All tests passed - axgf-rs V1 fully operational");
    println!("==============================================================");
}
