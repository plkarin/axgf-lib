// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the Phase 6 deduplicate() surface.
//!
//! Contract covered:
//! - Two families sharing the same spouse set are merged; children
//!   are unioned; the surviving record keeps the lowest-UUID id.
//! - Two persons with identical normalized name + identical birth
//!   year + identical death year are merged; references across the
//!   bundle are rewritten from victim to keeper.
//! - After a person merge, two families that shared "the same
//!   couple" under different person IDs are caught by the family
//!   pass as a byproduct.
//! - Father/son homonyms (same name across generations) are NOT
//!   merged and yield a MANUAL_REVIEW_REQUIRED diagnostic.
//! - Same-name siblings/cousins are NOT merged (same diagnostic).
//! - Families with different union.type or start dates > 1 year
//!   apart are flagged as ambiguous and NOT merged.

use axgf_rs::boundary::envelope::Status;
use axgf_rs::{create_bundle, deduplicate};
use serde_json::{json, Value};

fn to_str(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}

fn person(id: &str, display: &str, birth: Option<&str>, death: Option<&str>) -> Value {
    let mut p = json!({
        "id": id, "type": "person", "axgf_version": "1.0",
        "identity": {"name": {"display": display, "components": []},
                     "gender": {"value": "U"}, "is_living": false}
    });
    if let Some(b) = birth {
        p["birth"] = json!({"date": {"value": b}});
    }
    if let Some(d) = death {
        p["death"] = json!({"date": {"value": d}});
    }
    p
}

fn family(id: &str, spouses: &[&str], children: &[&str]) -> Value {
    json!({
        "id": id, "type": "family", "axgf_version": "1.0",
        "union": {
            "type": "marriage",
            "persons": spouses.iter().map(|s| json!({"person_id": s, "role": "spouse"})).collect::<Vec<_>>()
        },
        "children": children.iter().map(|c| json!({"person_id": c, "birth_order": 1})).collect::<Vec<_>>()
    })
}

// ---------- Family pass ----------

#[test]
fn two_families_with_same_spouse_set_merge_children_are_unioned() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let c1 = "550e8400-e29b-41d4-a716-446655440010";
    let c2 = "550e8400-e29b-41d4-a716-446655440011";
    // Two families with identical spouse set but different children.
    let fa = "aaaa1234-e29b-41d4-a716-446655440001";
    let fb = "bbbb1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({
        p1: person(p1, "A", None, None),
        p2: person(p2, "B", None, None),
        c1: person(c1, "C1", None, None),
        c2: person(c2, "C2", None, None),
    });
    b["families"] = json!({
        fa: family(fa, &[p1, p2], &[c1]),
        fb: family(fb, &[p1, p2], &[c2]),
    });
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["merged_families"], 1);
    let out = &env.data["bundle"];
    // Keeper = lowest UUID.
    assert!(
        out["families"][fa].is_object(),
        "keeper family gone: {:?}",
        out["families"]
    );
    assert!(out["families"][fb].is_null() || out["families"].get(fb).is_none());
    let kids: Vec<&str> = out["families"][fa]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["person_id"].as_str().unwrap())
        .collect();
    assert_eq!(kids.len(), 2, "children should be unioned: {kids:?}");
    assert!(kids.contains(&c1));
    assert!(kids.contains(&c2));
}

#[test]
fn family_pass_leaves_bundle_untouched_when_no_duplicates() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let fa = "aaaa1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({ p1: person(p1, "A", None, None), p2: person(p2, "B", None, None) });
    b["families"] = json!({ fa: family(fa, &[p1, p2], &[]) });
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_families"], 0);
    assert_eq!(env.data["merged_persons"], 0);
    assert_eq!(env.data["manual_review"], 0);
    assert!(env.data["bundle"]["families"][fa].is_object());
}

#[test]
fn families_with_different_union_type_are_flagged_not_merged() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let fa = "aaaa1234-e29b-41d4-a716-446655440001";
    let fb = "bbbb1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({ p1: person(p1, "A", None, None), p2: person(p2, "B", None, None) });
    let mut fb_val = family(fb, &[p1, p2], &[]);
    fb_val["union"]["type"] = json!("cohabitation");
    b["families"] = json!({
        fa: family(fa, &[p1, p2], &[]),
        fb: fb_val,
    });
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_families"], 0);
    assert_eq!(env.data["manual_review"], 1);
    assert!(env
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "MANUAL_REVIEW_REQUIRED"));
    // Both families still present.
    assert!(env.data["bundle"]["families"][fa].is_object());
    assert!(env.data["bundle"]["families"][fb].is_object());
}

/// The operator's own duplicate, verbatim: the same couple entered twice by a
/// GEDCOM conversion, once as a thin record whose `union.type` is the
/// `unknown` sentinel and once with the marriage, its date, place and event.
/// Only the people are stand-ins — their records are not what is under test.
///
/// Before 0.5.0 this pair was refused as ambiguous, because `unknown` counted
/// as a union type that disagreed with `marriage`. Fixing only that would have
/// merged it into the *thin* record — the keeper is the lowest UUID — and
/// discarded the victim's `union` whole, date and all. Both halves ship
/// together, and this is the pair that says so.
fn operators_duplicate_pair() -> (Value, Value) {
    let thin = json!({
        "id": "e823f9e2-8817-4d44-aabb-8b3999dcffe6",
        "type": "family", "axgf_version": "1.0",
        "created_at": "2026-08-09T13:15:54Z", "updated_at": "2026-08-09T13:15:54Z",
        "version_num": 1,
        "union": {
            "type": "unknown",
            "persons": [
                {"person_id": "df124fc1-f59a-46a9-b8e6-21481bcff2d4", "role": "spouse"},
                {"person_id": "c75f5032-038c-46b3-b818-d97f9089fd67", "role": "spouse"}
            ],
            "confidence": 0.8
        },
        "children": [{"person_id": "5eb76a82-bc24-409f-b3fa-d04f9e759a7a", "confidence": 0.8}]
    });
    let full = json!({
        "id": "f6c1c9ae-82b2-44b6-92fc-390c85566f65",
        "type": "family", "axgf_version": "1.0",
        "created_at": "2026-08-09T13:15:54Z", "updated_at": "2026-08-09T13:15:54Z",
        "version_num": 1,
        "union": {
            "type": "marriage",
            "persons": [
                {"person_id": "c75f5032-038c-46b3-b818-d97f9089fd67", "role": "spouse"},
                {"person_id": "df124fc1-f59a-46a9-b8e6-21481bcff2d4", "role": "spouse"}
            ],
            "confidence": 0.8,
            "start": {
                "date": {"value": "1991-08-24", "precision": "exact",
                         "calendar": "gregorian", "circa": false},
                "place_id": "e93709ad-3e83-4bb2-9063-0d0e5fc28d2b",
                "event_id": "d774f0ab-969e-4197-8ca6-32f85609c9eb"
            },
            "status": "unknown"
        },
        "children": [{"person_id": "5eb76a82-bc24-409f-b3fa-d04f9e759a7a", "confidence": 0.8}]
    });
    (thin, full)
}

#[test]
fn the_operators_duplicate_couple_merges_and_keeps_the_date_the_thin_record_lacked() {
    let (thin, full) = operators_duplicate_pair();
    let (h, w, kid) = (
        "df124fc1-f59a-46a9-b8e6-21481bcff2d4",
        "c75f5032-038c-46b3-b818-d97f9089fd67",
        "5eb76a82-bc24-409f-b3fa-d04f9e759a7a",
    );
    let thin_id = thin["id"].as_str().unwrap().to_string();
    let full_id = full["id"].as_str().unwrap().to_string();
    let mut b = create_bundle(None).data;
    b["persons"] = json!({
        h: person(h, "Husband", Some("1965"), None),
        w: person(w, "Wife", Some("1968"), None),
        kid: person(kid, "Child", Some("1993"), None),
    });
    b["families"] = json!({ &thin_id: thin, &full_id: full });

    let env = deduplicate(&to_str(&b));
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["merged_families"], 1, "{:?}", env.diagnostics);
    assert_eq!(env.data["manual_review"], 0, "{:?}", env.diagnostics);

    let fams = &env.data["bundle"]["families"];
    assert!(fams.get(&full_id).is_none(), "the victim is gone");
    let kept = &fams[&thin_id];
    assert!(kept.is_object(), "the lowest UUID is still the keeper");
    let u = &kept["union"];
    // What the thin record lacked, and the whole point of this release.
    assert_eq!(u["start"]["date"]["value"], "1991-08-24");
    assert_eq!(u["start"]["date"]["precision"], "exact");
    assert_eq!(
        u["start"]["place_id"],
        "e93709ad-3e83-4bb2-9063-0d0e5fc28d2b"
    );
    assert_eq!(
        u["start"]["event_id"],
        "d774f0ab-969e-4197-8ca6-32f85609c9eb"
    );
    // `unknown` gave way to the type the other record did record.
    assert_eq!(u["type"], "marriage");
    // The couple is still two people, not four.
    assert_eq!(u["persons"].as_array().unwrap().len(), 2);
    assert_eq!(kept["children"].as_array().unwrap().len(), 1);
}

#[test]
fn a_union_field_both_records_hold_keeps_the_keepers_and_fills_only_the_gaps() {
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let fa = "aaaa1234-e29b-41d4-a716-446655440001";
    let fb = "bbbb1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({ p1: person(p1, "A", None, None), p2: person(p2, "B", None, None) });
    // The keeper knows where; the victim knows when. Neither may be lost.
    let mut keeper = family(fa, &[p1, p2], &[]);
    keeper["union"]["start"] = json!({"place_id": "11111111-1111-4111-8111-111111111111"});
    keeper["union"]["confidence"] = json!(0.9);
    let mut victim = family(fb, &[p1, p2], &[]);
    victim["union"]["start"] = json!({
        "date": {"value": "1950", "precision": "year"},
        "place_id": "22222222-2222-4222-8222-222222222222"
    });
    victim["union"]["confidence"] = json!(0.4);
    b["families"] = json!({ fa: keeper, fb: victim });

    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_families"], 1);
    let u = &env.data["bundle"]["families"][fa]["union"];
    assert_eq!(u["start"]["date"]["value"], "1950", "the gap is filled");
    assert_eq!(
        u["start"]["place_id"], "11111111-1111-4111-8111-111111111111",
        "a value the keeper recorded is not overwritten"
    );
    assert_eq!(u["confidence"], 0.9);
}

#[test]
fn two_recorded_types_that_disagree_are_still_refused_when_one_side_is_unknown_too() {
    // The sentinel is ignored, not the check: marriage against cohabitation
    // is a real disagreement whatever a third record leaves unrecorded.
    let mut b = create_bundle(None).data;
    let p1 = "550e8400-e29b-41d4-a716-446655440001";
    let p2 = "550e8400-e29b-41d4-a716-446655440002";
    let ids = [
        "aaaa1234-e29b-41d4-a716-446655440001",
        "bbbb1234-e29b-41d4-a716-446655440001",
        "cccc1234-e29b-41d4-a716-446655440001",
    ];
    b["persons"] = json!({ p1: person(p1, "A", None, None), p2: person(p2, "B", None, None) });
    let mut fams = serde_json::Map::new();
    for (id, t) in ids.iter().zip(["unknown", "marriage", "cohabitation"]) {
        let mut f = family(id, &[p1, p2], &[]);
        f["union"]["type"] = json!(t);
        fams.insert(id.to_string(), f);
    }
    b["families"] = Value::Object(fams);
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_families"], 0);
    assert_eq!(env.data["manual_review"], 1);
}

// ---------- Person pass ----------

#[test]
fn two_persons_with_identical_name_and_dates_merge_and_refs_rewrite() {
    let mut b = create_bundle(None).data;
    let a = "550e8400-e29b-41d4-a716-446655440001"; // keeper (lower)
    let b_id = "550e8400-e29b-41d4-a716-446655440002"; // victim
    let evt = "aaaa1234-e29b-41d4-a716-446655440100";
    b["persons"] = json!({
        a:    person(a,    "Jean Pierre-Léonard", Some("1900"), Some("1970")),
        b_id: person(b_id, "Jean Pierre-Léonard", Some("1900"), Some("1970")),
    });
    // Event participant references the victim by entity_id.
    b["events"] = json!({
        evt: {
            "id": evt, "type": "event", "axgf_version": "1.0",
            "category": "birth", "date": {"value": "1900"},
            "participants": [{"entity_type": "person", "entity_id": b_id, "role": "subject"}]
        }
    });

    let env = deduplicate(&to_str(&b));
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["merged_persons"], 1);
    let out = &env.data["bundle"];
    assert!(out["persons"][a].is_object());
    assert!(
        out["persons"].get(b_id).is_none() || out["persons"][b_id].is_null(),
        "victim should be gone"
    );
    // Reference rewritten on the event.
    assert_eq!(out["events"][evt]["participants"][0]["entity_id"], a);
}

#[test]
fn duplicated_couples_merge_via_person_pass_then_family_pass() {
    let mut b = create_bundle(None).data;
    let a1 = "550e8400-e29b-41d4-a716-446655440001"; // Alice #1 (keeper)
    let a2 = "550e8400-e29b-41d4-a716-446655440002"; // Alice #2 (victim)
    let b1 = "550e8400-e29b-41d4-a716-446655440011"; // Bob   #1 (keeper)
    let b2 = "550e8400-e29b-41d4-a716-446655440012"; // Bob   #2 (victim)
    let fa = "aaaa1234-e29b-41d4-a716-446655440001";
    let fb = "bbbb1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({
        a1: person(a1, "Alice", Some("1900"), Some("1970")),
        a2: person(a2, "Alice", Some("1900"), Some("1970")),
        b1: person(b1, "Bob",   Some("1898"), Some("1972")),
        b2: person(b2, "Bob",   Some("1898"), Some("1972")),
    });
    b["families"] = json!({
        fa: family(fa, &[a1, b1], &[]),
        fb: family(fb, &[a2, b2], &[]),  // "same couple" under duplicate person ids
    });
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_persons"], 2);
    // After person merge, both families reference the same spouse set → family merge.
    assert_eq!(env.data["merged_families"], 1);
}

// ---------- Manual review ----------

#[test]
fn father_son_homonym_is_flagged_for_manual_review_not_merged() {
    // Two persons with the same name; one is a child of the other via a
    // family relationship — same-name across generations.
    let mut b = create_bundle(None).data;
    let dad = "550e8400-e29b-41d4-a716-446655440001";
    let son = "550e8400-e29b-41d4-a716-446655440002";
    let mom = "550e8400-e29b-41d4-a716-446655440003";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({
        dad: person(dad, "Jean Homonyme", Some("1900"), Some("1970")),
        son: person(son, "Jean Homonyme", Some("1900"), Some("1970")),
        mom: person(mom, "Marie", None, None),
    });
    b["families"] = json!({ fam: family(fam, &[dad, mom], &[son]) });
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_persons"], 0, "must NOT merge father/son");
    assert_eq!(env.data["manual_review"], 1);
    assert!(env
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "MANUAL_REVIEW_REQUIRED"));
    // Both persons still present.
    assert!(env.data["bundle"]["persons"][dad].is_object());
    assert!(env.data["bundle"]["persons"][son].is_object());
}

#[test]
fn same_name_siblings_are_flagged_for_manual_review_not_merged() {
    // Two persons who share a common parent → likely twins or a
    // pathological case; either way DO NOT merge without a human.
    let mut b = create_bundle(None).data;
    let sib1 = "550e8400-e29b-41d4-a716-446655440001";
    let sib2 = "550e8400-e29b-41d4-a716-446655440002";
    let dad = "550e8400-e29b-41d4-a716-446655440003";
    let mom = "550e8400-e29b-41d4-a716-446655440004";
    let fam = "aaaa1234-e29b-41d4-a716-446655440001";
    b["persons"] = json!({
        sib1: person(sib1, "Same Name", Some("1900"), Some("1970")),
        sib2: person(sib2, "Same Name", Some("1900"), Some("1970")),
        dad:  person(dad, "Dad", None, None),
        mom:  person(mom, "Mom", None, None),
    });
    b["families"] = json!({ fam: family(fam, &[dad, mom], &[sib1, sib2]) });
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.data["merged_persons"], 0);
    assert_eq!(env.data["manual_review"], 1);
}

// ---------- Version gating ----------

#[test]
fn deduplicate_rejects_unsupported_spec_version() {
    let mut b = create_bundle(None).data;
    b["manifest"]["axgf"] = json!("9.9");
    let env = deduplicate(&to_str(&b));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
}
