// SPDX-License-Identifier: Apache-2.0
//! End-to-end tests for the typed model layer.
//!
//! Two shapes of test live here:
//!
//! 1. **Spec-example parsing** — every entity example in SPEC_1.0.md
//!    must deserialize into the corresponding typed struct without
//!    error, and the resulting typed fields must carry the expected
//!    values. Known-default fields (empty arrays, null options, zero
//!    counters) are legitimately omitted from serialized output; the
//!    check here focuses on payload fidelity, not string identity.
//!
//! 2. **Forward-compat round-trip** — unknown fields at every level of
//!    the entity tree MUST survive a parse-then-serialize cycle
//!    verbatim (spec principle P9). This is enforced with a strict
//!    JSON-subset check.

use axgf_rs::model::{
    document::Document, event::Event, family::Family, link::Link, manifest::Manifest,
    occupation::Occupation, person::Person, place::Place, source::Source,
};
use serde_json::{json, Value};

/// Parse `input` into `T`. Panics with a helpful message on failure.
fn parse<T: serde::de::DeserializeOwned>(input: &Value) -> T {
    serde_json::from_value(input.clone())
        .unwrap_or_else(|e| panic!("failed to parse: {e}\ninput={input}"))
}

/// Assert that every leaf key/value present in `expected` also appears
/// at the same JSON path in `actual`. Used for the strict
/// forward-compat check. Does NOT require full equality (so
/// legitimately-omitted default fields don't cause noise).
fn assert_contains(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, v) in e {
                let av = a.get(k).unwrap_or_else(|| {
                    panic!("missing key {path}/{k} in round-trip output.\n  expected sub={v}\n  actual object={a:?}")
                });
                assert_contains(v, av, &format!("{path}/{k}"));
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(e.len(), a.len(), "array length mismatch at {path}");
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                assert_contains(ev, av, &format!("{path}[{i}]"));
            }
        }
        (e, a) => assert_eq!(e, a, "value mismatch at {path}"),
    }
}

// ---------- Manifest ----------

#[test]
fn minimal_manifest_from_spec_13_2_parses() {
    let input = json!({
        "axgf": "1.0",
        "created_at": "2026-06-15T10:00:00Z",
        "updated_at": "2026-06-15T10:00:00Z",
        "stats": {"persons": 1, "families": 0, "events": 0, "links": 0}
    });
    let m: Manifest = parse(&input);
    assert_eq!(m.axgf, "1.0");
    assert_eq!(m.stats.persons, 1);
    assert_eq!(m.stats.families, 0);
    assert_eq!(m.stats.events, 0);
    assert_eq!(m.stats.links, 0);
}

#[test]
fn full_manifest_from_spec_section_3_parses() {
    let input = json!({
        "axgf": "1.0",
        "created_at": "2026-06-15T10:00:00Z",
        "updated_at": "2026-06-15T14:30:00Z",
        "generator": {"name": "ax-genealogy", "version": "1.0.0", "url": "https://ax-genealogy.example.com"},
        "family": {
            "name": "Famille Pierre-Léonard",
            "description": "Lignée Pierre-Léonard — La Réunion → France métropolitaine",
            "primary_culture": "fr",
            "primary_place": "La Réunion, France",
            "time_span": {"earliest": "1850", "latest": "2026"}
        },
        "stats": {"persons": 142, "families": 38, "events": 289, "links": 67,
                  "occupations": 54, "sources": 103, "places": 47, "documents": 215},
        "checksums": {"algorithm": "sha256", "manifest": "a3f8c2d1..."},
        "privacy": {"contains_living_persons": true, "living_persons_redacted": false, "gdpr_compliant": true},
        "license": {"type": "private", "note": "Family use only. Not for redistribution."},
        "compatibility": {"gedcom_source": "5.5.1", "gedcom_export": "7.0"}
    });
    let m: Manifest = parse(&input);
    assert_eq!(m.stats.persons, 142);
    assert_eq!(m.stats.documents, 215);
    assert_eq!(m.family.as_ref().unwrap().primary_culture.as_deref(), Some("fr"));
    assert_eq!(m.privacy.as_ref().unwrap().contains_living_persons, Some(true));
    // `compatibility` and `checksums` are typed as raw Value — payload preserved verbatim.
    assert_eq!(m.compatibility.as_ref().unwrap()["gedcom_source"], "5.5.1");
    assert_eq!(m.checksums.as_ref().unwrap()["algorithm"], "sha256");
}

// ---------- Person ----------

#[test]
fn minimal_person_from_spec_13_1_parses() {
    let input = json!({
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "type": "person",
        "axgf_version": "1.0",
        "created_at": "2026-06-15T10:00:00Z",
        "updated_at": "2026-06-15T10:00:00Z",
        "version_num": 1,
        "identity": {
            "name": {
                "display": "Jean Pierre-Léonard",
                "components": [
                    {"type": "given_name", "value": "Jean", "order": 1},
                    {"type": "family_name", "value": "Pierre-Léonard", "order": 2}
                ]
            },
            "gender": {"value": "M"},
            "is_living": false,
            "visibility": "members"
        }
    });
    let p: Person = parse(&input);
    assert_eq!(p.base.id, "550e8400-e29b-41d4-a716-446655440001");
    assert_eq!(p.base.kind, "person");
    assert_eq!(p.identity.name.display, "Jean Pierre-Léonard");
    assert_eq!(p.identity.name.components.len(), 2);
    assert_eq!(p.identity.name.components[1].value, "Pierre-Léonard");
    assert_eq!(p.identity.gender.value, "M");
    assert!(!p.identity.is_living);
    assert_eq!(p.identity.visibility.as_deref(), Some("members"));
    assert!(p.birth.is_none());
    assert!(p.death.is_none());
}

#[test]
fn rich_person_from_spec_4_1_parses_all_blocks() {
    let input = json!({
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "type": "person",
        "axgf_version": "1.0",
        "created_at": "2026-05-01T10:00:00Z",
        "updated_at": "2026-06-10T14:00:00Z",
        "version_num": 3,
        "created_by": "admin@ax-genealogy.local",
        "identity": {
            "name": {
                "display": "Jean Pierre-Léonard",
                "culture": "fr", "direction": "ltr", "display_order": "given_first",
                "components": [
                    {"type": "given_name", "value": "Jean", "value_latin": "Jean", "order": 1},
                    {"type": "family_name", "value": "Pierre-Léonard", "value_latin": "Pierre-Leonard", "order": 2}
                ]
            },
            "names": [{"type": "birth", "display": "Jean-Baptiste Pierre-Léonard",
                       "components": [{"type": "given_name", "value": "Jean-Baptiste", "order": 1}],
                       "source_id": "550e8400-e29b-41d4-a716-446655440002",
                       "confidence": 0.95, "valid_from": "1923-04-12"}],
            "gender": {"value": "M", "note": "Valeurs: M, F, NB, U"},
            "is_living": false, "visibility": "members"
        },
        "birth": {
            "date": {"value": "1923-04-12", "calendar": "gregorian",
                     "precision": "exact", "circa": false, "confidence": 0.98},
            "place_id": "550e8400-e29b-41d4-a716-446655440003",
            "confidence": 0.98
        },
        "death": {"date": {"value": "1987-03-03"}, "confidence": 0.99, "cause": null},
        "bio": "Instituteur retraité.",
        "notes": "Possible lien à investiguer.",
        "documents": [{"document_id": "aaaa1234-e29b-41d4-a716-446655440004",
                       "role": "birth_certificate", "note": "Acte de naissance original"}],
        "ai": {
            "vault_page": "vault/wiki/persons/550e8400.md",
            "embedding_model": "nomic-embed-text",
            "hypotheses": [{"id": "hyp-001", "claim": "Possible sibling",
                            "confidence": 0.87, "status": "pending",
                            "evidence": "same surname", "created_at": "2026-06-01T08:00:00Z"}]
        },
        "extensions": {
            "x-axgenealogy-dedup-score": 0.0,
            "x-axgenealogy-generation": 2
        }
    });
    let p: Person = parse(&input);
    assert_eq!(p.base.version_num, Some(3));
    assert_eq!(p.identity.names.len(), 1);
    // Name-list entry fields (type/source_id/confidence/valid_from) live in extras.
    let name_entry = &p.identity.names[0];
    assert_eq!(name_entry.extra["type"], "birth");
    assert_eq!(name_entry.extra["confidence"], 0.95);
    // Birth is a full Vital with date+confidence.
    let birth = p.birth.as_ref().unwrap();
    assert_eq!(birth.date.as_ref().unwrap().value.as_deref(), Some("1923-04-12"));
    assert_eq!(birth.date.as_ref().unwrap().confidence, Some(0.98));
    assert_eq!(birth.confidence, Some(0.98));
    // Death cause is null in input; parses as None.
    let death = p.death.as_ref().unwrap();
    assert!(death.cause.is_none());
    assert_eq!(death.confidence, Some(0.99));
    // Documents attached.
    assert_eq!(p.documents.len(), 1);
    assert_eq!(p.documents[0].role.as_deref(), Some("birth_certificate"));
    // AI block with one hypothesis.
    let ai = p.ai.as_ref().unwrap();
    assert_eq!(ai.hypotheses.len(), 1);
    assert_eq!(ai.hypotheses[0].status, "pending");
    // Extensions kept as raw Value.
    assert_eq!(p.base.extensions.as_ref().unwrap()["x-axgenealogy-generation"], 2);
}

// ---------- Family ----------

#[test]
fn family_from_spec_4_2_parses() {
    let input = json!({
        "id": "aaaa1234-e29b-41d4-a716-446655440001",
        "type": "family",
        "axgf_version": "1.0",
        "created_at": "2026-05-01T10:00:00Z",
        "name": "Famille Pierre-Léonard — Branche Jean",
        "description": "Union de Jean Pierre-Léonard et Élise Bernard, Paris 1948",
        "union": {
            "type": "marriage",
            "status": "ended_by_death",
            "persons": [
                {"person_id": "550e8400-e29b-41d4-a716-446655440001", "role": "spouse"},
                {"person_id": "550e8400-e29b-41d4-a716-446655440011", "role": "spouse"}
            ],
            "start": {"date": {"value": "1948-06-15", "precision": "exact"},
                      "place_id": "aaaa1234-e29b-41d4-a716-446655440099",
                      "event_id": "aaaa1234-e29b-41d4-a716-446655440100"},
            "end": {"date": {"value": "1987-03-03"}, "reason": "death_of_spouse",
                    "note": "Fin par décès de Jean"},
            "confidence": 0.99,
            "source_id": "aaaa1234-e29b-41d4-a716-446655440077"
        },
        "children": [
            {"person_id": "aaaa1234-e29b-41d4-a716-446655440021", "birth_order": 1, "confidence": 0.99}
        ],
        "documents": [{"document_id": "aaaa1234-e29b-41d4-a716-446655440088", "role": "family_photo"}],
        "notes": "Famille installée à Paris."
    });
    let f: Family = parse(&input);
    assert_eq!(f.union.kind, "marriage");
    assert_eq!(f.union.status.as_deref(), Some("ended_by_death"));
    assert_eq!(f.union.persons.len(), 2);
    assert_eq!(f.union.end.as_ref().unwrap().reason.as_deref(), Some("death_of_spouse"));
    assert_eq!(f.children.len(), 1);
    assert_eq!(f.children[0].birth_order, Some(1));
    assert_eq!(f.documents[0].role.as_deref(), Some("family_photo"));
}

#[test]
fn polygamous_family_extras_survive() {
    // The polygamous shape in SPEC §4.2.2 uses fields not in our typed
    // Union struct (primary_person_id, unions[]) — they must round-trip
    // via the union.extra map.
    let input = json!({
        "id": "bbbb1234-e29b-41d4-a716-446655440001",
        "type": "family",
        "axgf_version": "1.0",
        "union": {
            "type": "polygamous",
            "persons": [{"person_id": "aaaa1234-e29b-41d4-a716-446655440020", "role": "spouse"}],
            "primary_person_id": "aaaa1234-e29b-41d4-a716-446655440020",
            "unions": [
                {"spouse_id": "aaaa1234-e29b-41d4-a716-446655440030", "start": {"date": {"value": "1890"}}},
                {"spouse_id": "aaaa1234-e29b-41d4-a716-446655440031", "start": {"date": {"value": "1895"}}}
            ]
        }
    });
    let f: Family = parse(&input);
    // Both unknown fields captured in extras.
    assert_eq!(f.union.extra["primary_person_id"], "aaaa1234-e29b-41d4-a716-446655440020");
    let unions = f.union.extra["unions"].as_array().unwrap();
    assert_eq!(unions.len(), 2);
    assert_eq!(unions[1]["spouse_id"], "aaaa1234-e29b-41d4-a716-446655440031");
    // Re-serialize: extras survive at their original path.
    let out = serde_json::to_value(&f).unwrap();
    assert_eq!(out["union"]["primary_person_id"], "aaaa1234-e29b-41d4-a716-446655440020");
    assert_eq!(out["union"]["unions"].as_array().unwrap().len(), 2);
}

// ---------- Event ----------

#[test]
fn event_from_spec_4_3_parses() {
    let input = json!({
        "id": "aaaa1234-e29b-41d4-a716-446655440100",
        "type": "event",
        "axgf_version": "1.0",
        "created_at": "2026-05-01T10:00:00Z",
        "category": "marriage",
        "subcategory": "civil",
        "date": {"value": "1948-06-15", "calendar": "gregorian",
                 "precision": "exact", "circa": false, "confidence": 0.99},
        "place_id": "aaaa1234-e29b-41d4-a716-446655440099",
        "participants": [
            {"entity_type": "person", "entity_id": "550e8400-e29b-41d4-a716-446655440001",
             "role": "spouse_1", "confidence": 0.99},
            {"entity_type": "family", "entity_id": "aaaa1234-e29b-41d4-a716-446655440001",
             "role": "created", "confidence": 0.99}
        ],
        "description": "Mariage civil.",
        "confidence": 0.99,
        "source_id": "aaaa1234-e29b-41d4-a716-446655440077",
        "ai": {"vault_page": "vault/wiki/events/evt-marriage.md"}
    });
    let e: Event = parse(&input);
    assert_eq!(e.category, "marriage");
    assert_eq!(e.subcategory.as_deref(), Some("civil"));
    assert_eq!(e.participants.len(), 2);
    assert_eq!(e.participants[0].role, "spouse_1");
    assert_eq!(e.participants[1].entity_type, "family");
    assert_eq!(e.date.value.as_deref(), Some("1948-06-15"));
    assert_eq!(e.date.calendar.as_deref(), Some("gregorian"));
}

// ---------- Link ----------

#[test]
fn link_from_spec_4_4_parses() {
    let input = json!({
        "id": "cccc1234-e29b-41d4-a716-446655440001",
        "type": "link",
        "axgf_version": "1.0",
        "from": {"entity_type": "person", "entity_id": "550e8400-e29b-41d4-a716-446655440001"},
        "to":   {"entity_type": "person", "entity_id": "550e8400-e29b-41d4-a716-446655440042"},
        "label": "parrain", "label_reverse": "filleul", "category": "spiritual",
        "bidirectional": false,
        "valid_from": {"date": {"value": "1950-03-15", "precision": "exact"},
                       "event_id": "aaaa1234-e29b-41d4-a716-446655440101"},
        "valid_until": null,
        "confidence": 0.85,
        "source_id": "aaaa1234-e29b-41d4-a716-446655440078",
        "note": "Mentionné dans lettre familiale de 1952",
        "visibility": "members"
    });
    let l: Link = parse(&input);
    assert_eq!(l.label, "parrain");
    assert_eq!(l.label_reverse.as_deref(), Some("filleul"));
    assert_eq!(l.from.entity_id, "550e8400-e29b-41d4-a716-446655440001");
    assert_eq!(l.to.entity_id, "550e8400-e29b-41d4-a716-446655440042");
    assert!(l.valid_from.is_some());
    assert!(l.valid_until.is_none());
    assert_eq!(l.bidirectional, Some(false));
}

// ---------- Occupation ----------

#[test]
fn occupation_from_spec_4_5_parses() {
    let input = json!({
        "id": "dddd1234-e29b-41d4-a716-446655440001",
        "type": "occupation",
        "axgf_version": "1.0",
        "person_id": "550e8400-e29b-41d4-a716-446655440001",
        "title": "Instituteur",
        "title_latin": "Primary school teacher",
        "title_normalized": "teacher",
        "employer": {"name": "École publique de Saint-Denis",
                     "place_id": "aaaa1234-e29b-41d4-a716-446655440099"},
        "place_id": "aaaa1234-e29b-41d4-a716-446655440099",
        "valid_from": {"date": {"value": "1948", "precision": "year"}},
        "valid_until": {"date": {"value": "1978", "precision": "year"}},
        "confidence": 0.90,
        "source_id": "aaaa1234-e29b-41d4-a716-446655440079"
    });
    let o: Occupation = parse(&input);
    assert_eq!(o.title, "Instituteur");
    assert_eq!(o.title_normalized.as_deref(), Some("teacher"));
    assert_eq!(o.employer.as_ref().unwrap().name.as_deref(), Some("École publique de Saint-Denis"));
    assert_eq!(o.valid_from.as_ref().unwrap().date.as_ref().unwrap().precision.as_deref(), Some("year"));
    assert_eq!(o.confidence, Some(0.90));
}

// ---------- Source ----------

#[test]
fn source_from_spec_5_4_parses() {
    let input = json!({
        "id": "eeee1234-e29b-41d4-a716-446655440001",
        "type": "source",
        "axgf_version": "1.0",
        "title": "Acte de naissance n°47 — Jean Pierre-Léonard",
        "source_type": "birth_certificate",
        "reliability": "primary",
        "confidence": 0.98,
        "status": "verified",
        "repository": {"name": "Archives départementales", "location": "Saint-Denis",
                       "url": "https://archives.reunion.fr", "reference": "5MI/47/1923/0047"},
        "date": {"value": "1923-04-12", "precision": "exact"},
        "conflicts": [{"source_id": "eeee1234-e29b-41d4-a716-446655440002",
                       "field": "birthdate", "this_value": "1923-04-12",
                       "conflict_value": "1923-04-15", "resolution": "this_preferred",
                       "resolution_note": "Original preferred over family Bible"}],
        "transcription": "Le douze avril mil neuf cent vingt-trois...",
        "language": "fr", "script": "latin"
    });
    let s: Source = parse(&input);
    assert_eq!(s.source_type, "birth_certificate");
    assert_eq!(s.reliability, "primary");
    assert_eq!(s.repository.as_ref().unwrap().reference.as_deref(), Some("5MI/47/1923/0047"));
    assert_eq!(s.conflicts.len(), 1);
    assert_eq!(s.conflicts[0].field, "birthdate");
    assert_eq!(s.conflicts[0].resolution.as_deref(), Some("this_preferred"));
    assert_eq!(s.language.as_deref(), Some("fr"));
}

#[test]
fn dna_source_from_spec_5_4_3_parses() {
    let input = json!({
        "id": "eeee1234-e29b-41d4-a716-446655440005",
        "type": "source",
        "axgf_version": "1.0",
        "title": "DNA test",
        "source_type": "dna",
        "reliability": "primary",
        "dna": {
            "test_provider": "23andMe",
            "test_type": "autosomal",
            "test_date": "2024-03-15",
            "kit_id": "anonymized",
            "match": {"person_id": "550e8400-e29b-41d4-a716-446655440042",
                      "shared_cm": 847, "shared_percent": 12.5,
                      "predicted_relationship": "first_cousin", "confidence": 0.92}
        }
    });
    let s: Source = parse(&input);
    let dna = s.dna.as_ref().unwrap();
    assert_eq!(dna.test_provider.as_deref(), Some("23andMe"));
    assert_eq!(dna.test_type.as_deref(), Some("autosomal"));
    let m = dna.match_.as_ref().unwrap();
    assert_eq!(m.shared_cm, Some(847.0));
    assert_eq!(m.predicted_relationship.as_deref(), Some("first_cousin"));
}

// ---------- Place ----------

#[test]
fn place_from_spec_5_3_parses() {
    let input = json!({
        "id": "aaaa1234-e29b-41d4-a716-446655440099",
        "type": "place",
        "axgf_version": "1.0",
        "names": [
            {"lang": "fr", "value": "Saint-Denis", "is_primary": true},
            {"lang": "en", "value": "Saint-Denis, Réunion", "is_primary": false}
        ],
        "place_type": "city",
        "region": "La Réunion",
        "country_current": "FR",
        "coordinates": {"lat": -20.8823, "lon": 55.4504, "precision": "city_center"},
        "country_history": [{"country": "FR", "note": "French territory since 1638"}],
        "identifiers": {"wikidata": "Q47045", "geonames": "935264", "insee": "97411"}
    });
    let p: Place = parse(&input);
    assert_eq!(p.names.len(), 2);
    assert_eq!(p.names[0].lang, "fr");
    assert!(p.names[0].is_primary);
    assert!(!p.names[1].is_primary);
    assert_eq!(p.place_type.as_deref(), Some("city"));
    assert_eq!(p.country_current.as_deref(), Some("FR"));
    assert_eq!(p.coordinates.as_ref().unwrap().lat, Some(-20.8823));
    assert_eq!(p.identifiers.as_ref().unwrap().insee.as_deref(), Some("97411"));
}

// ---------- Document ----------

#[test]
fn document_from_spec_5_5_parses() {
    let input = json!({
        "id": "ffff1234-e29b-41d4-a716-446655440001",
        "type": "document",
        "axgf_version": "1.0",
        "filename": "acte-naissance-jean-1923.pdf",
        "mime_type": "application/pdf",
        "document_type": "birth_certificate",
        "status": "present",
        "file": {"path": "documents/files/doc-001.pdf", "size_bytes": 1048576,
                 "sha256": "a3f8c2d1e4b5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1"},
        "date": {"value": "1923-04-12", "precision": "exact"},
        "language": "fr",
        "linked_to": [
            {"entity_type": "person", "entity_id": "550e8400-e29b-41d4-a716-446655440001", "role": "subject"},
            {"entity_type": "source", "entity_id": "eeee1234-e29b-41d4-a716-446655440001", "role": "evidence"}
        ],
        "ocr": {"text": "Le douze avril...", "confidence": 0.94, "language": "fr", "engine": "tesseract-5"},
        "ai": {"summary": "Birth certificate.",
               "suggested_links": [{"entity_type": "person",
                                    "entity_id": "550e8400-e29b-41d4-a716-446655440001",
                                    "confidence": 0.97, "reason": "Name match"}]},
        "caption": "Acte de naissance — 1923"
    });
    let d: Document = parse(&input);
    assert_eq!(d.filename, "acte-naissance-jean-1923.pdf");
    assert_eq!(d.status, "present");
    assert_eq!(d.file.as_ref().unwrap().size_bytes, Some(1_048_576));
    assert_eq!(d.linked_to.len(), 2);
    assert_eq!(d.ocr.as_ref().unwrap().engine.as_deref(), Some("tesseract-5"));
    let ai = d.ai.as_ref().unwrap();
    assert_eq!(ai.suggested_links.len(), 1);
    assert_eq!(ai.suggested_links[0].confidence, Some(0.97));
}

// ---------- Forward-compat: unknown fields survive at every level ----------

#[test]
fn unknown_fields_at_every_level_survive_person_round_trip() {
    // Fields prefixed `future_` do not exist in AXGF 1.0. Every one must
    // round-trip back out at exactly the same path — spec principle P9.
    let input = json!({
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "type": "person",
        "axgf_version": "1.0",
        "future_top": "keep me",
        "identity": {
            "name": {
                "display": "Jean",
                "components": [{"type": "given_name", "value": "Jean", "order": 1,
                                "future_component_field": 42}],
                "future_name_field": {"nested": true}
            },
            "gender": {"value": "M", "future_gender_field": "ok"},
            "is_living": false,
            "future_identity_field": [1, 2, 3]
        },
        "birth": {
            "date": {"value": "1923-04-12", "future_date_field": "keep"},
            "future_birth_field": "keep"
        }
    });
    let p: Person = parse(&input);
    let out = serde_json::to_value(&p).unwrap();
    // Every "future_*" path from input must reappear in output.
    assert_contains(&input, &out, "$");
}

#[test]
fn extensions_field_preserves_arbitrary_vendor_data() {
    let input = json!({
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "type": "person",
        "axgf_version": "1.0",
        "identity": {"name": {"display": "X", "components": []},
                     "gender": {"value": "U"}, "is_living": false},
        "extensions": {
            "x-vendorA-field1": "value1",
            "x-vendorB-field2": [1, 2, {"deep": "map"}]
        }
    });
    let p: Person = parse(&input);
    let out = serde_json::to_value(&p).unwrap();
    assert_eq!(out["extensions"]["x-vendorA-field1"], "value1");
    assert_eq!(out["extensions"]["x-vendorB-field2"][2]["deep"], "map");
}

#[test]
fn unknown_fields_on_family_and_event_survive() {
    // Cross-check on two more entities to be sure `#[serde(flatten)] extra`
    // is wired at every entity root and at every embedded sub-struct.
    let f_input = json!({
        "id": "aaaa1234-e29b-41d4-a716-446655440001",
        "type": "family",
        "axgf_version": "1.0",
        "future_family_field": {"a": 1},
        "union": {"type": "marriage",
                  "persons": [{"person_id": "550e8400-e29b-41d4-a716-446655440001", "role": "spouse",
                               "future_union_person_field": true}],
                  "future_union_field": [42]}
    });
    let f: Family = parse(&f_input);
    let f_out = serde_json::to_value(&f).unwrap();
    assert_contains(&f_input, &f_out, "$");

    let e_input = json!({
        "id": "aaaa1234-e29b-41d4-a716-446655440100",
        "type": "event",
        "axgf_version": "1.0",
        "category": "birth",
        "date": {"value": "1923-04-12", "future_date_field": "keep"},
        "future_event_field": "keep",
        "participants": [{"entity_type": "person",
                          "entity_id": "550e8400-e29b-41d4-a716-446655440001",
                          "role": "subject", "future_participant_field": 99}]
    });
    let e: Event = parse(&e_input);
    let e_out = serde_json::to_value(&e).unwrap();
    assert_contains(&e_input, &e_out, "$");
}
