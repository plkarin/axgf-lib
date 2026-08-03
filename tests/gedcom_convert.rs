// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the Phase 7 GEDCOM 5.5.1 → AXGF converter.
//!
//! Contract covered:
//! - The small fixture converts into the expected entity counts.
//! - Polish qualifiers `PRZED` / `OK` / `PO` map to range / circa
//!   date shapes.
//! - Unparseable date values are preserved verbatim in the date
//!   object's `note` field.
//! - webtrees-style OBJE nesting (FILE then FORM+TITL under FILE)
//!   produces a Document with a mapped mime type.
//! - Multiple NAME entries populate `identity.names[]` as alias
//!   entries.
//! - OCCU creates a standalone Occupation entity linked to the
//!   right person.
//! - `NOTE @xref@` refs resolve into the person's `notes` field.
//! - `CHIL @X@` with `PEDI adopted` is preserved on the family.children entry.
//! - Encoding auto-detect picks up on the UTF-8 BOM.

use axgf_rs::boundary::envelope::Status;
use axgf_rs::{convert_gedcom, validate};
use serde_json::Value;

fn load_fixture() -> Vec<u8> {
    std::fs::read("tests/fixtures/small.ged").expect("fixture readable")
}

fn convert() -> axgf_rs::boundary::envelope::Envelope {
    let bytes = load_fixture();
    convert_gedcom(&bytes, 0.9, "fr")
}

fn bundle(env: &axgf_rs::boundary::envelope::Envelope) -> &Value {
    &env.data["bundle"]
}

// ---------- entity counts ----------

#[test]
fn converts_fixture_into_expected_entity_counts() {
    let env = convert();
    assert_eq!(env.status, Status::Ok, "diags: {:?}", env.diagnostics);
    let b = bundle(&env);
    // 3 INDI → 3 persons.
    assert_eq!(b["persons"].as_object().unwrap().len(), 3);
    // 1 FAM → 1 family; 1 MARR → 1 event.
    assert_eq!(b["families"].as_object().unwrap().len(), 1);
    assert_eq!(b["events"].as_object().unwrap().len(), 1);
    // 1 OCCU on I1 → 1 occupation.
    assert_eq!(b["occupations"].as_object().unwrap().len(), 1);
    // 1 SOUR + 1 OBJE @M1@ + 1 inline OBJE under I1 → 2 documents.
    assert_eq!(b["documents"].as_object().unwrap().len(), 2);
    assert_eq!(b["sources"].as_object().unwrap().len(), 1);
    // Place dedup: "Saint-Denis, La Réunion" appears twice, "Paris" once → 2 places.
    assert_eq!(b["places"].as_object().unwrap().len(), 2);
    // Stats mirror the counts.
    let s = &b["manifest"]["stats"];
    assert_eq!(s["persons"], 3);
    assert_eq!(s["families"], 1);
    assert_eq!(s["events"], 1);
    assert_eq!(s["occupations"], 1);
    assert_eq!(s["sources"], 1);
    assert_eq!(s["documents"], 2);
    assert_eq!(s["places"], 2);
}

// ---------- date parsing ----------

#[test]
fn polish_qualifiers_map_to_range_and_circa() {
    let env = convert();
    let b = bundle(&env);
    // Find Marie (I2) — birth "OK 1925" → circa year 1925.
    let marie = b["persons"]
        .as_object()
        .unwrap()
        .values()
        .find(|p| p["identity"]["name"]["display"] == "Marie Bernard")
        .expect("Marie present");
    let birth = &marie["birth"]["date"];
    assert_eq!(birth["value"], "1925");
    assert_eq!(birth["circa"], true);
    assert_eq!(birth["precision"], "year");
    // Marie death "PO 2000 R" → after year 2000 → range with earliest.
    let death = &marie["death"]["date"];
    assert_eq!(death["precision"], "unknown");
    assert!(
        death.get("value").is_none(),
        "ranged date must not carry a top-level value"
    );
    assert_eq!(death["range"]["earliest"]["value"], "2000");
    assert_eq!(death["range"]["earliest"]["precision"], "year");
    assert!(death["range"].get("latest").is_none());

    // Jean (I1) death "PRZED 1990" → before year 1990 → range with latest.
    let jean = b["persons"]
        .as_object()
        .unwrap()
        .values()
        .find(|p| p["identity"]["name"]["display"] == "Jean Pierre-Léonard")
        .expect("Jean present");
    let jd = &jean["death"]["date"];
    assert_eq!(jd["precision"], "unknown");
    assert!(
        jd.get("value").is_none(),
        "ranged date must not carry a top-level value"
    );
    assert_eq!(jd["range"]["latest"]["value"], "1990");
    assert_eq!(jd["range"]["latest"]["precision"], "year");
    assert!(jd["range"].get("earliest").is_none());
}

#[test]
fn unparseable_date_is_preserved_as_note_never_dropped() {
    let env = convert();
    let paul = bundle(&env)["persons"]
        .as_object()
        .unwrap()
        .values()
        .find(|p| p["identity"]["name"]["display"] == "Paul Pierre-Léonard")
        .expect("Paul present");
    let bd = &paul["birth"]["date"];
    assert_eq!(bd["precision"], "unknown");
    assert!(
        bd.get("value").is_none(),
        "unparseable dates must omit `value` (schema types it as string, not nullable)"
    );
    assert_eq!(
        bd["note"], "bogus-date-value",
        "unparseable dates must be preserved verbatim in the date.note field"
    );
}

#[test]
fn french_month_names_parse() {
    let env = convert();
    let b = bundle(&env);
    // Jean birth "12 KWIETNIA 1923" — Polish month April → 1923-04-12.
    let jean = b["persons"]
        .as_object()
        .unwrap()
        .values()
        .find(|p| p["identity"]["name"]["display"] == "Jean Pierre-Léonard")
        .unwrap();
    assert_eq!(jean["birth"]["date"]["value"], "1923-04-12");
    assert_eq!(jean["birth"]["date"]["precision"], "exact");
    // Marriage "15 JUIN 1948" — French June → 1948-06-15.
    let evt = b["events"].as_object().unwrap().values().next().unwrap();
    assert_eq!(evt["date"]["value"], "1948-06-15");
    assert_eq!(evt["category"], "marriage");
}

// ---------- OBJE nesting ----------

#[test]
fn webtrees_obje_nesting_produces_correct_mime_type() {
    let env = convert();
    let docs = bundle(&env)["documents"].as_object().unwrap();
    // Find the inline OBJE (attached to I1, photo.jpg).
    let photo = docs
        .values()
        .find(|d| d["filename"] == "photo.jpg")
        .expect("photo document present");
    assert_eq!(photo["mime_type"], "image/jpeg");
    assert_eq!(photo["document_type"], "photo");
    assert_eq!(photo["caption"], "Family photo (webtrees nesting)");
    // Top-level @M1@ OBJE (pdf) — flat OBJE style.
    let cert = docs
        .values()
        .find(|d| d["filename"] == "certificate.pdf")
        .expect("certificate.pdf present");
    assert_eq!(cert["mime_type"], "application/pdf");
    assert_eq!(cert["document_type"], "other");
}

// ---------- multiple NAME ----------

#[test]
fn multiple_name_entries_become_alias_names() {
    let env = convert();
    let jean = bundle(&env)["persons"]
        .as_object()
        .unwrap()
        .values()
        .find(|p| p["identity"]["name"]["display"] == "Jean Pierre-Léonard")
        .unwrap();
    let names = jean["identity"]["names"].as_array().unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0]["display"], "Jean-Baptiste Pierre-Léonard");
    assert_eq!(names[0]["type"], "alias");
}

// ---------- OCCU + PEDI + NOTE ----------

#[test]
fn occu_becomes_standalone_occupation_linked_to_person() {
    let env = convert();
    let b = bundle(&env);
    let occ = b["occupations"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(occ["title"], "Instituteur");
    // person_id points to the person whose display is "Jean Pierre-Léonard".
    let pid = occ["person_id"].as_str().unwrap();
    let owner = &b["persons"][pid];
    assert_eq!(owner["identity"]["name"]["display"], "Jean Pierre-Léonard");
    // valid_from date 1948 parses to a year.
    assert_eq!(occ["valid_from"]["date"]["value"], "1948");
}

#[test]
fn note_xref_ref_resolves_into_person_notes() {
    let env = convert();
    let jean = bundle(&env)["persons"]
        .as_object()
        .unwrap()
        .values()
        .find(|p| p["identity"]["name"]["display"] == "Jean Pierre-Léonard")
        .unwrap();
    let notes = jean["notes"].as_str().unwrap();
    assert!(
        notes.contains("Referenced note body preserved."),
        "resolved @N1@ body should be inlined; got: {notes:?}"
    );
    assert!(
        notes.contains("Note inline about Jean."),
        "inline notes also included; got: {notes:?}"
    );
}

#[test]
fn family_chil_preserves_pedi_adopted() {
    let env = convert();
    let fam = bundle(&env)["families"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    let child = &fam["children"].as_array().unwrap()[0];
    assert!(
        child["note"].as_str().unwrap().contains("adopted"),
        "PEDI adopted should be preserved on the child entry"
    );
}

// ---------- encoding ----------

#[test]
fn utf8_bom_is_detected_and_stripped() {
    let mut bytes = b"\xef\xbb\xbf".to_vec();
    bytes.extend(load_fixture());
    let env = convert_gedcom(&bytes, 0.9, "fr");
    assert_eq!(env.status, Status::Ok);
    // Still parses the same number of INDIs.
    assert_eq!(bundle(&env)["persons"].as_object().unwrap().len(), 3);
}

// ---------- regression guard ----------

// A real webtrees export of ~767 persons with Polish date qualifiers,
// nameless INDIs, bare OCCU tags, parentless sibling groups, and stray
// FAM stubs. These are the exact conditions that hid bugs 1–7. The
// three-person fixture would not have caught any of them.
//
// The bundle produced from this fixture MUST validate with zero
// SCHEMA_VALIDATION_FAILED diagnostics. Any regression in the
// converter that emits schema-invalid output on real-world input
// will fail this test.
#[test]
fn converted_real_world_fixture_has_zero_schema_warnings() {
    let bytes = std::fs::read("tests/fixtures/tree.ged").expect("tree.ged fixture readable");
    let env = convert_gedcom(&bytes, 0.9, "fr");
    assert_eq!(
        env.status,
        Status::Ok,
        "convert diags: {:?}",
        env.diagnostics
    );

    let flat = bundle(&env);
    let flat_str = serde_json::to_string(flat).unwrap();
    let val = validate(&flat_str);

    let schema_warnings: Vec<_> = val
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "SCHEMA_VALIDATION_FAILED")
        .collect();
    assert_eq!(
        schema_warnings.len(),
        0,
        "expected 0 SCHEMA_VALIDATION_FAILED, got {}: {:#?}",
        schema_warnings.len(),
        schema_warnings
    );
}
