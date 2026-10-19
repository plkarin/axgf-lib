// SPDX-License-Identifier: Apache-2.0
//! Merging two records a person says are one (0.6.0): `plan_merge`, then
//! `merge_entities`.
//!
//! Contract covered:
//! - A plan changes nothing, and finds every kind of field: agreements,
//!   joined lists, combined objects, conflicts with their choices and
//!   suggestions.
//! - Suggestions fail on the safe side: the more restrictive visibility,
//!   living, both texts, the better attested birth.
//! - A birth not chosen becomes an event, and the chosen source records the
//!   other's disagreement in `conflicts`; the result is valid.
//! - A display name not chosen joins the other names as an alias.
//! - Every reference to the absorbed record is gone, in all eight kinds,
//!   `derived_from` included; the duplicates that makes are tidied.
//! - The report carries the absorbed record verbatim.
//! - Refusals: an ancestor, a spouse and a child of one family, the two
//!   spouses of a union, two families of different couples; two siblings
//!   only with the name typed back.
//! - A stale plan, an unresolved conflict and an unsupported kind are
//!   refused, and nothing changes.
//! - Families, places and sources merge by their own rules.

use axgf_rs::boundary::envelope::{Envelope, Severity, Status};
use axgf_rs::{create_bundle, merge_entities, plan_merge, update_entity, validate, EntityKind};
use serde_json::{json, Value};

/// A lowercase UUID v4 that reads as a number.
fn u(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn person(id: &str, display: &str) -> Value {
    json!({
        "id": id, "type": "person", "axgf_version": "1.0", "version_num": 1,
        "created_at": "2026-01-01T00:00:00Z",
        "identity": {"name": {"display": display, "components": []},
                     "gender": {"value": "U"}, "is_living": false}
    })
}

fn family(id: &str, spouses: &[&str], children: &[&str]) -> Value {
    json!({
        "id": id, "type": "family", "axgf_version": "1.0",
        "union": {"type": "marriage",
                  "persons": spouses.iter().map(|s| json!({"person_id": s, "role": "spouse"})).collect::<Vec<_>>()},
        "children": children.iter().map(|c| json!({"person_id": c, "confidence": 0.9})).collect::<Vec<_>>()
    })
}

fn source(id: &str, title: &str) -> Value {
    json!({"id": id, "type": "source", "axgf_version": "1.0", "title": title,
           "source_type": "baptism_record", "reliability": "primary"})
}

fn place(id: &str, names: Value) -> Value {
    json!({"id": id, "type": "place", "axgf_version": "1.0", "names": names})
}

/// A bundle holding these records, each in its own collection.
fn bundle(records: &[Value]) -> String {
    let mut b = create_bundle(Some("Test")).data;
    for r in records {
        let coll = match r["type"].as_str().unwrap() {
            "person" => "persons",
            "family" => "families",
            "event" => "events",
            "link" => "links",
            "occupation" => "occupations",
            "source" => "sources",
            "place" => "places",
            "document" => "documents",
            other => panic!("{other}"),
        };
        b[coll][r["id"].as_str().unwrap()] = r.clone();
    }
    b.to_string()
}

fn ok(env: &Envelope) -> &Value {
    assert_eq!(env.status, Status::Ok, "{:?}", env.diagnostics);
    &env.data
}

fn codes(env: &Envelope) -> Vec<&'static str> {
    env.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// The plan's suggestion for every conflict: what a caller who accepts them
/// all sends back.
fn accept(plan: &Value) -> String {
    let mut choices = serde_json::Map::new();
    for c in plan["conflicts"].as_array().unwrap() {
        choices.insert(c["field"].as_str().unwrap().into(), c["suggestion"].clone());
    }
    json!({"fingerprint": plan["fingerprint"], "choices": choices}).to_string()
}

fn conflict<'a>(plan: &'a Value, field: &str) -> &'a Value {
    plan["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["field"] == field)
        .unwrap_or_else(|| panic!("no conflict on {field}: {}", plan["conflicts"]))
}

/// The grandmother entered twice: one record from a baptism, one from a
/// burial, each with its own birth date and source.
fn grandmother() -> (String, String, String, String, String) {
    let (keep, absorb, s1, s2, village) = (u(1), u(2), u(11), u(12), u(21));
    let mut k = person(&keep, "Zofia Mazur");
    k["identity"]["visibility"] = json!("members");
    k["identity"]["names"] = json!([{"display": "Zosia", "type": "nickname", "components": []}]);
    k["birth"] = json!({"date": {"value": "1834-03-02", "precision": "exact"}, "source_id": s1, "confidence": 0.9});
    k["notes"] = json!("Baptised at Wąwolnica.");
    let mut a = person(&absorb, "Zofia Mazurowa");
    a["identity"]["visibility"] = json!("private");
    a["identity"]["is_living"] = json!(true);
    a["identity"]["gender"] = json!({"value": "F"});
    a["birth"] = json!({"date": {"value": "1835", "precision": "year"}, "place_id": village, "source_id": s2, "confidence": 0.6});
    a["death"] = json!({"date": {"value": "1901", "precision": "year"}});
    a["notes"] = json!("Buried at Kazimierz.");
    let b = bundle(&[
        k,
        a,
        source(&s1, "Baptisms, Wąwolnica"),
        source(&s2, "Burials, Kazimierz"),
        place(
            &village,
            json!([{"lang": "pl", "value": "Wąwolnica", "is_primary": true}]),
        ),
    ]);
    (b, keep, absorb, s1, s2)
}

#[test]
fn a_plan_changes_nothing_and_names_every_disagreement() {
    let (b, keep, absorb, _, _) = grandmother();
    let env = plan_merge(&b, EntityKind::Person, &keep, &absorb);
    let plan = ok(&env);
    assert_eq!(plan["mergeable"], true);
    assert!(plan["fingerprint"].as_str().unwrap().len() >= 16);
    // A plan returns no bundle: there is nothing it could have changed.
    assert!(plan.get("bundle").is_none());

    let fields: Vec<&str> = plan["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["field"].as_str().unwrap())
        .collect();
    for f in [
        "identity.name",
        "identity.visibility",
        "identity.is_living",
        "birth",
        "notes",
    ] {
        assert!(fields.contains(&f), "{f} in {fields:?}");
    }
    // What only one holds is taken, not asked about.
    let agreed = |f: &str, from: &str| {
        plan["agreements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["field"] == f && a["from"] == from)
    };
    assert!(agreed("death", "absorb"), "{}", plan["agreements"]);
    assert!(
        agreed("identity.gender", "absorb"),
        "an unknown gender says nothing"
    );
    assert!(agreed("identity.names", "keep"));
    assert!(!fields.contains(&"identity.gender") && !fields.contains(&"death"));
}

#[test]
fn suggestions_fail_on_the_safe_side() {
    let (b, keep, absorb, _, _) = grandmother();
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let vis = conflict(&plan, "identity.visibility");
    assert_eq!(
        (vis["suggestion"].as_str(), vis["because"].as_str()),
        (Some("absorb"), Some("more_restrictive"))
    );
    let living = conflict(&plan, "identity.is_living");
    assert_eq!(
        (living["suggestion"].as_str(), living["because"].as_str()),
        (Some("absorb"), Some("protective"))
    );
    let notes = conflict(&plan, "notes");
    assert_eq!(notes["choices"], json!(["keep", "absorb", "both"]));
    assert_eq!(notes["suggestion"], "both");
    let birth = conflict(&plan, "birth");
    assert_eq!(
        (birth["suggestion"].as_str(), birth["because"].as_str()),
        (Some("keep"), Some("higher_confidence"))
    );
    assert_eq!(birth["not_chosen_becomes"], "event");
    assert_eq!(
        conflict(&plan, "identity.name")["not_chosen_becomes"],
        "alias"
    );
}

#[test]
fn a_birth_not_chosen_becomes_an_event_and_its_source_records_the_disagreement() {
    let (b, keep, absorb, s1, s2) = grandmother();
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let data = ok(&env);
    let out = &data["bundle"];
    let p = &out["persons"][&keep];
    assert_eq!(
        p["birth"]["date"]["value"], "1834-03-02",
        "the better attested birth stays"
    );
    assert_eq!(
        p["death"]["date"]["value"], "1901",
        "the death only the other had is taken"
    );
    assert_eq!(p["identity"]["visibility"], "private");
    assert_eq!(p["identity"]["is_living"], true);
    assert!(out["persons"].get(&absorb).is_none());

    // The 1835 birth is an event now, the kept person its subject.
    let made = data["report"]["events_created"].as_array().unwrap();
    assert_eq!(made.len(), 1);
    let e = &out["events"][made[0]["id"].as_str().unwrap()];
    assert_eq!(e["category"], "birth");
    assert_eq!(e["date"]["value"], "1835");
    assert_eq!(e["source_id"], s2.as_str());
    assert_eq!(
        e["participants"],
        json!([{"entity_type": "person", "entity_id": keep, "role": "subject"}])
    );

    // The baptism register records that the burial register disagrees.
    let c = &out["sources"][&s1]["conflicts"];
    assert!(
        c.as_array().unwrap().contains(&json!({
            "source_id": s2, "field": "birth.date", "this_value": "1834-03-02",
            "conflict_value": "1835", "resolution": "this_preferred"
        })),
        "{c}"
    );

    // And the whole bundle is valid.
    let checked = validate(&out.to_string());
    assert_eq!(checked.status, Status::Ok);
    let errors: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn the_display_name_not_chosen_joins_the_other_names_and_both_texts_are_kept() {
    let (b, keep, absorb, _, _) = grandmother();
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let p = &ok(&env)["bundle"]["persons"][&keep];
    assert_eq!(p["identity"]["name"]["display"], "Zofia Mazur");
    let names = p["identity"]["names"].as_array().unwrap();
    assert!(
        names.iter().any(|n| n["display"] == "Zosia"),
        "the kept record's own names stay"
    );
    assert!(
        names
            .iter()
            .any(|n| n["display"] == "Zofia Mazurowa" && n["type"] == "alias"),
        "{names:?}"
    );
    let notes = p["notes"].as_str().unwrap();
    assert!(notes.starts_with("Baptised at Wąwolnica."), "{notes}");
    assert!(
        notes.contains("— Zofia Mazurowa · 00000000 —\nBuried at Kazimierz."),
        "{notes}"
    );
}

#[test]
fn the_report_carries_the_absorbed_record_verbatim() {
    let (b, keep, absorb, _, _) = grandmother();
    let before: Value = serde_json::from_str(&b).unwrap();
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let report = &ok(&env)["report"];
    assert_eq!(report["absorbed_record"], before["persons"][&absorb]);
    assert_eq!(report["kept_id"], keep.as_str());
    assert_eq!(report["absorbed_id"], absorb.as_str());
    assert_eq!(
        report["resolutions"].as_array().unwrap().len(),
        plan["conflicts"].as_array().unwrap().len()
    );
}

#[test]
fn every_reference_to_the_absorbed_record_is_gone_in_all_eight_kinds() {
    let (keep, absorb, child, other, wife) = (u(1), u(2), u(3), u(4), u(5));
    let (f1, f2, ev, ln, oc, src, pl, doc) =
        (u(31), u(32), u(33), u(34), u(35), u(36), u(37), u(38));
    let mut kid = person(&child, "Jan Kowalski");
    // The patronymic derives from the father: a reference not named *_id.
    kid["identity"]["name"]["components"] = json!([
        {"type": "patronymic", "value": "Janowicz", "order": 1, "derived_from": absorb, "derivation_type": "patronymic"}
    ]);
    let mut a = person(&absorb, "Jan Kowalski");
    a["identity"]["names"] =
        json!([{"display": "Johann Kowalski", "type": "transliteration", "components": []}]);
    let b = bundle(&[
        person(&keep, "Jan Kowalski"),
        a,
        kid,
        person(&other, "Anna Nowak"),
        person(&wife, "Maria Wiśniewska"),
        family(&f1, &[&absorb, &wife], &[&child]),
        family(&f2, &[&other], &[]),
        json!({"id": ev, "type": "event", "axgf_version": "1.0", "category": "census", "date": {"value": "1880"},
               "participants": [{"entity_type": "person", "entity_id": keep, "role": "head", "confidence": 0.7},
                                {"entity_type": "person", "entity_id": absorb, "role": "head", "confidence": 0.9}]}),
        json!({"id": ln, "type": "link", "axgf_version": "1.0", "label": "godfather of",
               "from": {"entity_type": "person", "entity_id": absorb}, "to": {"entity_type": "person", "entity_id": other}}),
        json!({"id": oc, "type": "occupation", "axgf_version": "1.0", "person_id": absorb, "title": "blacksmith"}),
        json!({"id": src, "type": "source", "axgf_version": "1.0", "title": "DNA", "source_type": "dna", "reliability": "primary",
               "dna": {"test_provider": "x", "test_type": "autosomal", "match": {"person_id": absorb, "shared_cm": 3400.0}}}),
        place(&pl, json!([{"lang": "pl", "value": "Lublin"}])),
        json!({"id": doc, "type": "document", "axgf_version": "1.0", "filename": "p.jpg", "mime_type": "image/jpeg",
               "document_type": "photo", "status": "referenced", "linked_to": [{"entity_type": "person", "entity_id": absorb}]}),
    ]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let refs = &plan["effects"]["references"];
    for coll in [
        "persons",
        "families",
        "events",
        "links",
        "occupations",
        "sources",
        "documents",
    ] {
        assert!(refs[coll].as_u64().unwrap_or(0) >= 1, "{coll}: {refs}");
    }
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let out = &ok(&env)["bundle"];
    assert!(
        !out.to_string().contains(&absorb),
        "the absorbed id survives somewhere"
    );
    assert_eq!(
        out["persons"][&child]["identity"]["name"]["components"][0]["derived_from"],
        keep.as_str()
    );
    assert_eq!(
        out["families"][&f1]["union"]["persons"][0]["person_id"],
        keep.as_str()
    );
    // The census named him twice: one participant is left, the surer one.
    let parts = out["events"][&ev]["participants"].as_array().unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["confidence"], 0.9);
    // The other names are joined.
    let names = out["persons"][&keep]["identity"]["names"]
        .as_array()
        .unwrap();
    assert!(names.iter().any(|n| n["display"] == "Johann Kowalski"));
    let checked = validate(&out.to_string());
    assert!(
        checked
            .diagnostics
            .iter()
            .all(|d| d.code.as_str() != "DANGLING_REFERENCE"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn a_link_from_a_person_to_themselves_is_removed() {
    let (keep, absorb, ln) = (u(1), u(2), u(34));
    let b = bundle(&[
        person(&keep, "Piotr Lis"),
        person(&absorb, "Piotr Lis"),
        json!({"id": ln, "type": "link", "axgf_version": "1.0", "label": "witness for",
               "from": {"entity_type": "person", "entity_id": keep}, "to": {"entity_type": "person", "entity_id": absorb}}),
    ]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    assert_eq!(plan["effects"]["self_links_removed"], json!([ln]));
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let data = ok(&env);
    assert!(data["bundle"]["links"].get(&ln).is_none());
    assert_eq!(data["report"]["links_removed"], json!([ln]));
}

#[test]
fn an_ancestor_is_never_merged_with_a_descendant() {
    // Father and son of one name: the commonest homonym.
    let (father, son, mother, f) = (u(1), u(2), u(3), u(31));
    let b = bundle(&[
        person(&father, "Józef Nowak"),
        person(&son, "Józef Nowak"),
        person(&mother, "Ewa"),
        family(&f, &[&father, &mother], &[&son]),
    ]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &father, &son)).clone();
    assert_eq!(plan["mergeable"], false);
    let reasons: Vec<&str> = plan["refusals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["reason"].as_str().unwrap())
        .collect();
    assert!(
        reasons.contains(&"ancestor") && reasons.contains(&"spouse_and_child"),
        "{reasons:?}"
    );
    let env = merge_entities(&b, EntityKind::Person, &father, &son, &accept(&plan));
    assert_eq!(env.status, Status::Error);
    assert!(env.data.is_null());
    assert!(codes(&env).iter().all(|c| *c == "MERGE_REFUSED"));
}

#[test]
fn the_two_spouses_of_a_union_are_never_merged() {
    let (a, b2, f) = (u(1), u(2), u(31));
    let b = bundle(&[
        person(&a, "Alex Nowak"),
        person(&b2, "Alex Nowak"),
        family(&f, &[&a, &b2], &[]),
    ]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &a, &b2)).clone();
    assert!(plan["refusals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["reason"] == "spouses" && r["confirm"].is_null()));
    let env = merge_entities(&b, EntityKind::Person, &a, &b2, &accept(&plan));
    assert_eq!(codes(&env), ["MERGE_REFUSED"]);
}

#[test]
fn two_siblings_are_merged_only_when_the_name_is_typed_back() {
    // A child who died, and the next one given the name.
    let (dad, mum, first, second, f) = (u(1), u(2), u(3), u(4), u(31));
    let b = bundle(&[
        person(&dad, "Jan Lis"),
        person(&mum, "Anna Lis"),
        person(&first, "Stanisław Lis"),
        person(&second, "Stanisław Lis"),
        family(&f, &[&dad, &mum], &[&first, &second]),
    ]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &first, &second)).clone();
    let r = &plan["refusals"].as_array().unwrap()[0];
    assert_eq!(r["reason"], "siblings");
    assert_eq!(r["confirm"], "Stanisław Lis");
    assert_eq!(plan["mergeable"], true, "lifted by a typed confirmation");

    let refused = merge_entities(&b, EntityKind::Person, &first, &second, &accept(&plan));
    assert_eq!(codes(&refused), ["MERGE_REFUSED"]);
    let mut wrong: Value = serde_json::from_str(&accept(&plan)).unwrap();
    wrong["confirm"] = json!("stanislaw lis");
    assert_eq!(
        codes(&merge_entities(
            &b,
            EntityKind::Person,
            &first,
            &second,
            &wrong.to_string()
        )),
        ["MERGE_REFUSED"]
    );

    let mut typed: Value = serde_json::from_str(&accept(&plan)).unwrap();
    typed["confirm"] = json!("Stanisław Lis");
    let env = merge_entities(&b, EntityKind::Person, &first, &second, &typed.to_string());
    let fam = &ok(&env)["bundle"]["families"][&f];
    assert_eq!(fam["children"].as_array().unwrap().len(), 1, "listed once");
}

#[test]
fn a_stale_plan_is_refused_and_nothing_changes() {
    let (b, keep, absorb, _, _) = grandmother();
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    // Someone edits the kept record in between.
    let mut edited: Value = serde_json::from_str(&b).unwrap();
    let mut k = edited["persons"][&keep].clone();
    k["notes"] = json!("Baptised at Wąwolnica, godmother Agnieszka.");
    edited = ok(&update_entity(
        &edited.to_string(),
        EntityKind::Person,
        &k.to_string(),
    ))["bundle"]
        .clone();
    let env = merge_entities(
        &edited.to_string(),
        EntityKind::Person,
        &keep,
        &absorb,
        &accept(&plan),
    );
    assert_eq!(env.status, Status::Error);
    assert_eq!(codes(&env), ["STALE_PLAN"]);
    assert!(env.data.is_null());
}

#[test]
fn the_fingerprint_does_not_depend_on_the_order_of_keys() {
    let (b, keep, absorb, _, _) = grandmother();
    let one = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb))["fingerprint"].clone();
    // The same records, every object's keys reversed.
    fn reversed(v: &Value) -> Value {
        match v {
            Value::Object(m) => Value::Object(
                m.iter()
                    .rev()
                    .map(|(k, v)| (k.clone(), reversed(v)))
                    .collect(),
            ),
            Value::Array(a) => Value::Array(a.iter().map(reversed).collect()),
            other => other.clone(),
        }
    }
    let shuffled = reversed(&serde_json::from_str::<Value>(&b).unwrap()).to_string();
    assert_ne!(shuffled, b);
    let two = ok(&plan_merge(&shuffled, EntityKind::Person, &keep, &absorb))["fingerprint"].clone();
    assert_eq!(one, two);
}

#[test]
fn a_conflict_without_a_choice_it_allows_is_refused() {
    let (b, keep, absorb, _, _) = grandmother();
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let none = json!({"fingerprint": plan["fingerprint"], "choices": {}}).to_string();
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &none);
    assert_eq!(env.status, Status::Error);
    assert_eq!(
        env.diagnostics.len(),
        plan["conflicts"].as_array().unwrap().len()
    );
    assert!(codes(&env).iter().all(|c| *c == "MERGE_UNRESOLVED"));
    // "both" is for free text, not for a birth.
    let mut wrong: Value = serde_json::from_str(&accept(&plan)).unwrap();
    wrong["choices"]["birth"] = json!("both");
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &wrong.to_string());
    assert_eq!(codes(&env), ["MERGE_UNRESOLVED"]);
}

#[test]
fn events_links_occupations_and_documents_are_not_merged() {
    let (e1, e2) = (u(1), u(2));
    let ev = |id: &str| json!({"id": id, "type": "event", "axgf_version": "1.0", "category": "census", "date": {"value": "1880"}});
    let b = bundle(&[ev(&e1), ev(&e2)]);
    for kind in [
        EntityKind::Event,
        EntityKind::Link,
        EntityKind::Occupation,
        EntityKind::Document,
    ] {
        let env = plan_merge(&b, kind, &e1, &e2);
        assert_eq!(codes(&env), ["MERGE_UNSUPPORTED"], "{kind:?}");
    }
    assert_eq!(
        codes(&plan_merge(&b, EntityKind::Person, &u(8), &u(9))),
        ["ENTITY_NOT_FOUND"]
    );
}

#[test]
fn a_person_merge_lists_the_families_that_now_describe_one_couple() {
    // The grandfather entered twice, each record married to the same wife.
    let (keep, absorb, wife, f1, f2, c1, c2) = (u(1), u(2), u(3), u(31), u(32), u(4), u(5));
    let b = bundle(&[
        person(&keep, "Feliks Brzoza"),
        person(&absorb, "Feliks Brzoza"),
        person(&wife, "Rozalia"),
        person(&c1, "Ignacy"),
        person(&c2, "Wiktoria"),
        family(&f1, &[&keep, &wife], &[&c1]),
        family(&f2, &[&absorb, &wife], &[&c2]),
    ]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    assert_eq!(plan["effects"]["same_couple_families"], json!([[f1, f2]]));
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let data = ok(&env);
    assert_eq!(data["report"]["same_couple_families"], json!([[f1, f2]]));
    // Not merged on the side: one merge is one decision, and the family
    // merge is the next one, through the same two calls.
    let out = data["bundle"].to_string();
    let plan2 = ok(&plan_merge(&out, EntityKind::Family, &f1, &f2)).clone();
    assert_eq!(plan2["mergeable"], true, "{}", plan2["refusals"]);
    let env2 = merge_entities(&out, EntityKind::Family, &f1, &f2, &accept(&plan2));
    let fam = &ok(&env2)["bundle"]["families"][&f1];
    let kids: Vec<&str> = fam["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["person_id"].as_str().unwrap())
        .collect();
    assert_eq!(kids, [c1.as_str(), c2.as_str()]);
    assert_eq!(fam["union"]["persons"].as_array().unwrap().len(), 2);
}

#[test]
fn families_of_different_couples_are_refused_but_halves_of_one_couple_merge() {
    let (a, b2, c, f1, f2, f3) = (u(1), u(2), u(3), u(31), u(32), u(33));
    let mut half = family(&f3, &[&b2], &[]);
    half["union"]["start"] = json!({"date": {"value": "1870"}});
    let bd = bundle(&[
        person(&a, "A"),
        person(&b2, "B"),
        person(&c, "C"),
        family(&f1, &[&a, &b2], &[]),
        family(&f2, &[&a, &c], &[]),
        half,
    ]);
    let refused = ok(&plan_merge(&bd, EntityKind::Family, &f1, &f2)).clone();
    assert_eq!(refused["refusals"][0]["reason"], "different_couples");
    // One record knows the couple, the other only the wife and the date.
    let plan = ok(&plan_merge(&bd, EntityKind::Family, &f1, &f3)).clone();
    assert_eq!(plan["mergeable"], true);
    let env = merge_entities(&bd, EntityKind::Family, &f1, &f3, &accept(&plan));
    let fam = &ok(&env)["bundle"]["families"][&f1];
    assert_eq!(fam["union"]["start"]["date"]["value"], "1870");
    assert_eq!(fam["union"]["persons"].as_array().unwrap().len(), 2);
}

#[test]
fn places_join_their_names_and_ask_about_coordinates() {
    let (keep, absorb, who) = (u(1), u(2), u(3));
    let mut k = place(
        &keep,
        json!([{"lang": "pl", "value": "Wąwolnica", "is_primary": true}]),
    );
    k["coordinates"] = json!({"lat": 51.29, "lon": 22.14});
    let mut a = place(
        &absorb,
        json!([{"lang": "pl", "value": "Wawolnica", "is_primary": true}, {"lang": "de", "value": "Wonwolnitza"}]),
    );
    a["coordinates"] = json!({"lat": 51.3, "lon": 22.1});
    let mut p = person(&who, "Jan");
    p["birth"] = json!({"date": {"value": "1830"}, "place_id": absorb});
    let b = bundle(&[k, a, p]);
    let plan = ok(&plan_merge(&b, EntityKind::Place, &keep, &absorb)).clone();
    assert_eq!(plan["conflicts"].as_array().unwrap().len(), 1);
    assert_eq!(
        conflict(&plan, "coordinates")["choices"],
        json!(["keep", "absorb"])
    );
    let env = merge_entities(&b, EntityKind::Place, &keep, &absorb, &accept(&plan));
    let out = &ok(&env)["bundle"];
    let names = out["places"][&keep]["names"].as_array().unwrap();
    assert_eq!(names.len(), 3);
    assert_eq!(
        names
            .iter()
            .filter(|n| n["lang"] == "pl" && n["is_primary"] == true)
            .count(),
        1,
        "one primary per language"
    );
    assert_eq!(out["persons"][&who]["birth"]["place_id"], keep.as_str());
}

#[test]
fn sources_join_their_conflicts_and_their_citations_follow() {
    let (keep, absorb, other, who) = (u(1), u(2), u(3), u(4));
    let mut k = source(&keep, "Parish register, Wąwolnica");
    k["conflicts"] =
        json!([{"source_id": other, "field": "birth.date", "resolution": "unresolved"}]);
    let mut a = source(&absorb, "Parish register, Wąwolnica");
    a["reliability"] = json!("unknown");
    a["conflicts"] =
        json!([{"source_id": keep, "field": "death.date", "resolution": "unresolved"}]);
    a["note"] = json!("Microfilm 2041.");
    let mut p = person(&who, "Jan");
    p["birth"] = json!({"date": {"value": "1830"}, "source_id": absorb});
    let b = bundle(&[k, a, source(&other, "Census 1880"), p]);
    let plan = ok(&plan_merge(&b, EntityKind::Source, &keep, &absorb)).clone();
    assert!(
        plan["conflicts"].as_array().unwrap().is_empty(),
        "{}",
        plan["conflicts"]
    );
    let env = merge_entities(&b, EntityKind::Source, &keep, &absorb, &accept(&plan));
    let out = &ok(&env)["bundle"];
    let s = &out["sources"][&keep];
    assert_eq!(
        s["reliability"], "primary",
        "an unknown reliability says nothing"
    );
    assert_eq!(s["note"], "Microfilm 2041.");
    // The absorbed source's disagreement with the kept one is no longer a
    // disagreement: a source does not conflict with itself.
    let c = s["conflicts"].as_array().unwrap();
    assert_eq!(c.len(), 1, "{c:?}");
    assert_eq!(c[0]["source_id"], other.as_str());
    assert_eq!(out["persons"][&who]["birth"]["source_id"], keep.as_str());
}

#[test]
fn a_profile_series_is_joined_and_the_record_rises_to_1_1() {
    let (keep, absorb) = (u(1), u(2));
    let mut a = person(&absorb, "Ewa Lis");
    a["axgf_version"] = json!("1.1");
    a["residence"] =
        json!({"acquired_nationalities": [{"value": {"country": "PL"}, "confidence": 0.9}]});
    a["health"] = json!({"blood_group": {"value": "A", "confidence": 0.9}});
    let mut k = person(&keep, "Ewa Lis");
    k["axgf_version"] = json!("1.1");
    k["residence"] =
        json!({"acquired_nationalities": [{"value": {"country": "FR"}, "confidence": 0.8}]});
    k["health"] = json!({"blood_group": {"value": "B", "confidence": 0.6}});
    let b = bundle(&[k, a]);
    let plan = ok(&plan_merge(&b, EntityKind::Person, &keep, &absorb)).clone();
    let blood = conflict(&plan, "health.blood_group");
    assert_eq!(blood["suggestion"], "absorb", "the higher confidence");
    let env = merge_entities(&b, EntityKind::Person, &keep, &absorb, &accept(&plan));
    let out = &ok(&env)["bundle"];
    let p = &out["persons"][&keep];
    assert_eq!(
        p["residence"]["acquired_nationalities"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "a series is joined"
    );
    assert_eq!(p["health"]["blood_group"]["value"], "A");
    assert_eq!(p["axgf_version"], "1.1");
    assert_eq!(p["version_num"], 2, "one version more than either");
    assert_eq!(out["manifest"]["axgf"], "1.1");
}
