// SPDX-License-Identifier: Apache-2.0
//! AXGF 1.1 validation: every closed vocabulary, the semantic rules, and the
//! versions a bundle and its entities declare.
//!
//! The vocabulary tests are generated from the registry. For each place a
//! term can sit — an attribute's value, or a field inside it — a person is
//! built with the first term of that vocabulary, which must draw no
//! vocabulary finding, and then with a value that is not a term, which must
//! draw exactly one `OUT_OF_VOCABULARY` naming the vocabulary. A silently
//! accepted value fails the second half; a schema failure reported *instead*
//! fails it too, because the finding must say what it is.

#[path = "common/profile.rs"]
mod fixtures;

use std::collections::BTreeSet;

use axgf_rs::boundary::envelope::{Diagnostic, Envelope, Severity, Status};
use axgf_rs::model::profile::registry::{self, Attribute};
use axgf_rs::model::profile::vocab;
use axgf_rs::{
    add_entity, create_bundle, delete_entity, export_bundle, import_bundle, inspect, update_entity,
    validate, DeletePolicy, EntityKind,
};
use fixtures::{bundle_with, person_with, sample, vocabulary_locations, Fill, UUID_A};
use serde_json::{json, Value};

const NOT_A_TERM: &str = "not-a-term";

fn run(bundle: &Value) -> Envelope {
    validate(&bundle.to_string())
}

fn codes<'a>(env: &'a Envelope, code: &str) -> Vec<&'a Diagnostic> {
    env.diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .collect()
}

/// A person carrying `a`, with the vocabulary slot `field` set to `term`.
fn person_with_term(a: &Attribute, field: Option<&str>, term: &str) -> Value {
    let mut value = sample(&a.shape, Fill::Minimal);
    if a.path == "military.ranks" && matches!(field, Some("country" | "category")) {
        // A registered rank constrains both its country and its category, so
        // those two slots are tested on a rank given as text; the rank rules
        // have their own test below.
        value = json!({"rank_text": "Feldwebel"});
    }
    match field {
        None => value = json!(term),
        Some(f) => value[f] = json!(term),
    }
    person_with(&[(a, value)])
}

fn check_group(group: &str) {
    let mut checked = 0;
    for (a, field, vname) in vocabulary_locations()
        .into_iter()
        .filter(|(a, _, _)| a.group == group)
    {
        let voc = vocab::by_name(vname).unwrap();
        let slot = match field {
            Some(f) => format!("{}.value.{f}", a.path),
            None => format!("{}.value", a.path),
        };

        // A term of the vocabulary: no finding about vocabularies at all.
        let good = run(&bundle_with(&person_with_term(a, field, voc.terms[0])));
        assert_eq!(good.status, Status::Ok);
        let noise: Vec<_> = good
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.code.as_str(),
                    "OUT_OF_VOCABULARY" | "SCHEMA_VALIDATION_FAILED" | "CLAIM_INCONSISTENT"
                )
            })
            .collect();
        assert!(
            noise.is_empty(),
            "{slot} = {:?} is a term of {vname} and drew {noise:#?}",
            voc.terms[0]
        );

        // Not a term: exactly one OUT_OF_VOCABULARY, naming the vocabulary,
        // and no schema failure standing in for it.
        let bad = run(&bundle_with(&person_with_term(a, field, NOT_A_TERM)));
        let oov = codes(&bad, "OUT_OF_VOCABULARY");
        assert_eq!(
            oov.len(),
            1,
            "{slot} = {NOT_A_TERM:?} should draw one OUT_OF_VOCABULARY, drew {:#?}",
            bad.diagnostics
        );
        assert_eq!(oov[0].severity, Severity::Warning);
        assert!(
            oov[0].message.contains(vname) && oov[0].message.contains(NOT_A_TERM),
            "the finding names the vocabulary and the value: {}",
            oov[0].message
        );
        assert_eq!(
            oov[0].entity_ref.as_deref(),
            Some(&*format!("persons/{UUID_A}"))
        );
        assert!(
            codes(&bad, "SCHEMA_VALIDATION_FAILED").is_empty(),
            "{slot}: the schema's enum failure must not be reported a second time: {:#?}",
            bad.diagnostics
        );
        checked += 1;
    }
    assert!(checked > 0, "{group} has no vocabulary to check");
}

macro_rules! vocabulary_tests {
    ($($name:ident => $group:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                check_group($group);
            }
        )*
    };
}

vocabulary_tests! {
    identity_vocabularies_reject_what_is_not_a_term => "identity",
    morphology_vocabularies_reject_what_is_not_a_term => "morphology",
    biometrics_vocabularies_reject_what_is_not_a_term => "biometrics",
    health_vocabularies_reject_what_is_not_a_term => "health",
    genomics_vocabularies_reject_what_is_not_a_term => "genomics",
    death_vocabularies_reject_what_is_not_a_term => "death",
    residence_vocabularies_reject_what_is_not_a_term => "residence",
    education_vocabularies_reject_what_is_not_a_term => "education",
    military_vocabularies_reject_what_is_not_a_term => "military",
    legal_vocabularies_reject_what_is_not_a_term => "legal",
    belief_vocabularies_reject_what_is_not_a_term => "belief",
    personality_vocabularies_reject_what_is_not_a_term => "personality",
}

#[test]
fn every_vocabulary_is_exercised_somewhere() {
    // The group tests above reach every vocabulary an attribute uses. These
    // are the ones that sit elsewhere, and each has a test of its own below.
    let mut reached: BTreeSet<&str> = vocabulary_locations().iter().map(|(_, _, v)| *v).collect();
    for elsewhere in [
        "sensitive_class", // class_visibility keys, withheld_classes
        "artefact_type",   // artefact references
        "consent",         // artefact references
        "lineage",         // Family.children[]
        "link_relation",   // Link
    ] {
        reached.insert(elsewhere);
    }
    for rv in vocab::MILITARY_RANKS {
        reached.insert(rv.vocabulary.name); // the rank tests below
    }
    for v in vocab::ALL {
        assert!(
            reached.contains(v.name),
            "vocabulary {} is never validated",
            v.name
        );
    }
}

// -------------------------------------------------------- beyond attributes

fn person_json(extra: Value) -> Value {
    let mut p = json!({
        "id": UUID_A, "type": "person", "axgf_version": "1.1",
        "identity": {"name": {"display": "V", "components": []}, "gender": {"value": "U"}, "is_living": false}
    });
    for (k, v) in extra.as_object().unwrap() {
        if k == "identity" {
            for (ik, iv) in v.as_object().unwrap() {
                p["identity"][ik] = iv.clone();
            }
        } else {
            p[k] = v.clone();
        }
    }
    p
}

fn oov_messages(env: &Envelope) -> Vec<String> {
    codes(env, "OUT_OF_VOCABULARY")
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn a_class_visibility_names_only_classes_and_visibilities() {
    let ok = person_json(json!({"identity": {"class_visibility": {"health": "members"}}}));
    assert!(oov_messages(&run(&bundle_with(&ok))).is_empty());
    let bad = person_json(json!({"identity": {"class_visibility": {"politics": "loud"}}}));
    let msgs = oov_messages(&run(&bundle_with(&bad)));
    assert_eq!(msgs.len(), 2, "{msgs:?}");
    assert!(msgs
        .iter()
        .any(|m| m.contains("sensitive_class") && m.contains("politics")));
    assert!(msgs
        .iter()
        .any(|m| m.contains("visibility") && m.contains("loud")));
    assert!(codes(&run(&bundle_with(&bad)), "SCHEMA_VALIDATION_FAILED").is_empty());
}

#[test]
fn an_artefact_takes_only_the_types_its_attribute_allows() {
    let voice = |t: &str, consent: &str| {
        person_json(json!({"digital_legacy": {"voice_corpora": [{"value": {
            "document_id": fixtures::UUID_B, "artefact_type": t, "consent": consent}}]}}))
    };
    assert!(oov_messages(&run(&bundle_with(&voice("voice_corpus", "given")))).is_empty());
    let msgs = oov_messages(&run(&bundle_with(&voice("mesh", "given"))));
    assert!(msgs.len() == 1 && msgs[0].contains("mesh"), "{msgs:?}");
    let msgs = oov_messages(&run(&bundle_with(&voice("voice_corpus", "maybe"))));
    assert!(msgs.len() == 1 && msgs[0].contains("consent"), "{msgs:?}");
}

#[test]
fn a_rank_comes_from_the_vocabulary_its_country_selects() {
    let rank = |v: Value| person_json(json!({"military": {"ranks": [{"value": v}]}}));
    for rv in vocab::MILITARY_RANKS {
        for r in rv.ranks {
            let env = run(&bundle_with(&rank(
                json!({"country": rv.country, "rank": r.term, "category": r.category}),
            )));
            assert!(
                oov_messages(&env).is_empty() && codes(&env, "CLAIM_INCONSISTENT").is_empty(),
                "{} {} drew {:#?}",
                rv.country,
                r.term,
                env.diagnostics
            );
        }
        let env = run(&bundle_with(&rank(
            json!({"country": rv.country, "rank": NOT_A_TERM}),
        )));
        let msgs = oov_messages(&env);
        assert!(
            msgs.len() == 1 && msgs[0].contains(rv.vocabulary.name),
            "{}: {msgs:?}",
            rv.country
        );
    }
    // A French rank is not a Polish one, even when it is spelt like one.
    let msgs = oov_messages(&run(&bundle_with(&rank(
        json!({"country": "PL", "rank": "caporal"}),
    ))));
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    // A rank from an army 1.1 registers no list for is rank_text, not rank.
    let msgs = oov_messages(&run(&bundle_with(&rank(
        json!({"country": "AT", "rank": "feldwebel"}),
    ))));
    assert!(msgs.len() == 1 && msgs[0].contains("rank_text"), "{msgs:?}");
    let env = run(&bundle_with(&rank(
        json!({"rank_text": "Zugsführer", "category": "non_commissioned"}),
    )));
    assert!(oov_messages(&env).is_empty(), "{:#?}", env.diagnostics);
    // The stated category has to be the rank's.
    let env = run(&bundle_with(&rank(
        json!({"country": "FR", "rank": "caporal", "category": "non_commissioned"}),
    )));
    let inc = codes(&env, "CLAIM_INCONSISTENT");
    assert!(
        inc.len() == 1 && inc[0].message.contains("enlisted"),
        "{:#?}",
        env.diagnostics
    );
}

#[test]
fn an_analyte_belongs_to_its_panel() {
    let lab = |panel: &str, analyte: &str| {
        person_json(json!({"health": {"lab_results": [{"value": {
            "panel": panel, "analyte": analyte, "result": 5.1, "unit": "mmol/L"}}]}}))
    };
    for (panel, analytes) in vocab::LAB_PANEL_ANALYTES {
        for a in *analytes {
            assert!(
                oov_messages(&run(&bundle_with(&lab(panel, a)))).is_empty(),
                "{panel} {a}"
            );
        }
    }
    let msgs = oov_messages(&run(&bundle_with(&lab("lipid", "creatinine"))));
    assert!(msgs.len() == 1 && msgs[0].contains("lipid"), "{msgs:?}");
}

#[test]
fn lineage_relation_and_withheld_classes_are_vocabularies_too() {
    let mut b = create_bundle(None).data;
    b["manifest"]["axgf"] = json!("1.1");
    b["families"] = json!({"aaaa1234-e29b-41d4-a716-446655440001": {
        "id": "aaaa1234-e29b-41d4-a716-446655440001", "type": "family", "axgf_version": "1.1",
        "children": [{"person_id": UUID_A, "lineage": "borrowed"}, {"person_id": fixtures::UUID_B, "lineage": "step"}]}});
    b["links"] = json!({"bbbb1234-e29b-41d4-a716-446655440002": {
        "id": "bbbb1234-e29b-41d4-a716-446655440002", "type": "link", "axgf_version": "1.1",
        "from": {"entity_type": "person", "entity_id": UUID_A},
        "to": {"entity_type": "person", "entity_id": fixtures::UUID_B},
        "label": "x", "relation": "nemesis"}});
    b["manifest"]["privacy"] = json!({"withheld_classes": ["health", "gossip"]});
    let env = run(&b);
    let msgs = oov_messages(&env);
    assert_eq!(msgs.len(), 3, "{msgs:#?}");
    for needle in ["lineage", "link_relation", "sensitive_class"] {
        assert!(
            msgs.iter().any(|m| m.contains(needle)),
            "{needle}: {msgs:?}"
        );
    }
    assert!(
        codes(&env, "SCHEMA_VALIDATION_FAILED").is_empty(),
        "{:#?}",
        env.diagnostics
    );
}

// ------------------------------------------------------------ semantic rules

#[test]
fn a_subclade_must_sit_under_its_major_clade() {
    let hg = |major: &str, sub: &str| {
        person_json(
            json!({"genomics": {"y_haplogroup": {"value": {"major": major, "subclade": sub}}}}),
        )
    };
    assert!(codes(&run(&bundle_with(&hg("R", "R-M269"))), "CLAIM_INCONSISTENT").is_empty());
    let env = run(&bundle_with(&hg("R", "I-M253")));
    assert_eq!(
        codes(&env, "CLAIM_INCONSISTENT").len(),
        1,
        "{:#?}",
        env.diagnostics
    );
}

#[test]
fn an_epigenetic_clock_gives_its_own_kind_of_result() {
    let age = |v: Value| person_json(json!({"genomics": {"epigenetic_age": [{"value": v}]}}));
    let ok = [
        json!({"clock": "horvath", "age_years": 61.4}),
        json!({"clock": "dunedinpace", "pace": 0.94}),
    ];
    for v in ok {
        assert!(
            codes(&run(&bundle_with(&age(v.clone()))), "CLAIM_INCONSISTENT").is_empty(),
            "{v}"
        );
    }
    for v in [
        json!({"clock": "dunedinpace", "age_years": 61.4}),
        json!({"clock": "grimage", "pace": 0.9}),
    ] {
        assert_eq!(
            codes(&run(&bundle_with(&age(v.clone()))), "CLAIM_INCONSISTENT").len(),
            1,
            "{v}"
        );
    }
}

#[test]
fn a_period_cannot_end_before_it_starts() {
    let period = |from: &str, until: &str| {
        person_json(json!({"residence": {"addresses": [{
            "value": {"lines": "x"},
            "valid_from": {"date": {"value": from}}, "valid_until": {"date": {"value": until}}}]}}))
    };
    assert!(codes(
        &run(&bundle_with(&period("1950", "1960-05"))),
        "CLAIM_INCONSISTENT"
    )
    .is_empty());
    assert!(
        codes(
            &run(&bundle_with(&period("1950", "1950-03"))),
            "CLAIM_INCONSISTENT"
        )
        .is_empty(),
        "a year and a month in that year do not disagree"
    );
    assert_eq!(
        codes(
            &run(&bundle_with(&period("1950-06", "1949"))),
            "CLAIM_INCONSISTENT"
        )
        .len(),
        1
    );
}

#[test]
fn two_causes_of_death_cannot_share_a_place_in_the_chain() {
    let causes = person_json(json!({"death": {"causes": [
        {"value": {"description": "a", "sequence": 1}},
        {"value": {"description": "b", "sequence": 1}}]}}));
    assert_eq!(
        codes(&run(&bundle_with(&causes)), "CLAIM_INCONSISTENT").len(),
        1
    );
}

#[test]
fn an_attribute_1_1_does_not_define_is_reported_and_kept() {
    let p = person_json(json!({"health": {"blod_group": {"value": "A"}}}));
    let env = run(&bundle_with(&p));
    let info = codes(&env, "UNKNOWN_ATTRIBUTE");
    assert!(
        info.len() == 1
            && info[0].severity == Severity::Info
            && info[0].message.contains("blod_group"),
        "{:#?}",
        env.diagnostics
    );
    assert_eq!(env.data["errors"], 0);
}

#[test]
fn schema_shape_errors_in_1_1_content_are_still_schema_errors() {
    // Only enumeration failures move to OUT_OF_VOCABULARY. A series written as
    // a single claim is a structural fault and stays one.
    let p = person_json(json!({"morphology": {"height": {"value": 158}}}));
    let env = run(&bundle_with(&p));
    assert!(
        !codes(&env, "SCHEMA_VALIDATION_FAILED").is_empty(),
        "{:#?}",
        env.diagnostics
    );
    assert!(codes(&env, "OUT_OF_VOCABULARY").is_empty());
}

#[test]
fn a_1_0_enumeration_keeps_its_schema_diagnostic() {
    let p = person_json(json!({"identity": {"gender": {"value": "Q"}}}));
    let env = run(&bundle_with(&p));
    assert!(!codes(&env, "SCHEMA_VALIDATION_FAILED").is_empty());
    assert!(codes(&env, "OUT_OF_VOCABULARY").is_empty());
}

// ------------------------------------------------------------------ versions

fn with_version(v: &str) -> String {
    let mut b = create_bundle(None).data;
    b["manifest"]["axgf"] = json!(v);
    b.to_string()
}

#[test]
fn both_supported_versions_are_accepted_and_the_next_is_not() {
    for v in ["1.0", "1.1"] {
        let b = with_version(v);
        assert_eq!(validate(&b).status, Status::Ok, "validate {v}");
        assert_eq!(inspect(&b).status, Status::Ok, "inspect {v}");
        assert_eq!(export_bundle(&b).status, Status::Ok, "export {v}");
        let person = json!({"identity": {"name": {"display": "X", "components": []},
                                         "gender": {"value": "U"}, "is_living": false}});
        assert_eq!(
            add_entity(&b, EntityKind::Person, &person.to_string()).status,
            Status::Ok,
            "add {v}"
        );
    }
    let env = validate(&with_version("1.2"));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
}

#[test]
fn adding_1_1_content_to_a_1_0_bundle_raises_it_and_stamps_the_entity() {
    let b = create_bundle(None).data.to_string();
    assert_eq!(
        serde_json::from_str::<Value>(&b).unwrap()["manifest"]["axgf"],
        "1.0"
    );

    // A person with nothing 1.1 about it stays 1.0, and so does the bundle.
    let plain = json!({"identity": {"name": {"display": "Plain", "components": []},
                                    "gender": {"value": "U"}, "is_living": false}});
    let env = add_entity(&b, EntityKind::Person, &plain.to_string());
    let id = env.data["id"].as_str().unwrap().to_string();
    assert_eq!(env.data["bundle"]["persons"][&id]["axgf_version"], "1.0");
    assert_eq!(env.data["bundle"]["manifest"]["axgf"], "1.0");

    // One blood group later, both are 1.1.
    let mut rich = env.data["bundle"]["persons"][&id].clone();
    rich["health"] = json!({"blood_group": {"value": "O", "confidence": 0.9}});
    rich["axgf_version"] = json!("1.1");
    let env = update_entity(
        &env.data["bundle"].to_string(),
        EntityKind::Person,
        &rich.to_string(),
    );
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["bundle"]["manifest"]["axgf"], "1.1");
    assert!(
        env.diagnostics.is_empty(),
        "the write validated against the 1.1 schema and found nothing: {:#?}",
        env.diagnostics
    );

    // Deleting it does not lower the bundle again.
    let env = delete_entity(
        &env.data["bundle"].to_string(),
        EntityKind::Person,
        &id,
        DeletePolicy::Reject,
    );
    assert_eq!(env.data["bundle"]["manifest"]["axgf"], "1.1");
}

#[test]
fn add_stamps_the_oldest_version_the_content_fits() {
    let b = create_bundle(None).data.to_string();
    let rich = json!({"identity": {"name": {"display": "R", "components": []},
                                   "gender": {"value": "U"}, "is_living": false},
                      "morphology": {"height": [{"value": 170}]}});
    let env = add_entity(&b, EntityKind::Person, &rich.to_string());
    let id = env.data["id"].as_str().unwrap();
    assert_eq!(env.data["bundle"]["persons"][id]["axgf_version"], "1.1");
    assert_eq!(env.data["bundle"]["manifest"]["axgf"], "1.1");
}

#[test]
fn a_write_reports_out_of_vocabulary_values_as_it_saves_them() {
    // The CRUD path reports what validate would, so an editor learns at save
    // time rather than on the next validation run.
    let b = create_bundle(None).data.to_string();
    let bad = json!({"identity": {"name": {"display": "B", "components": []},
                                  "gender": {"value": "U"}, "is_living": false},
                     "health": {"blood_group": {"value": "Z"}}});
    let env = add_entity(&b, EntityKind::Person, &bad.to_string());
    assert_eq!(env.status, Status::Ok, "non-blocking");
    let oov = codes(&env, "OUT_OF_VOCABULARY");
    assert!(
        oov.len() == 1 && oov[0].message.contains("blood_group"),
        "{:#?}",
        env.diagnostics
    );
    assert!(
        codes(&env, "SCHEMA_VALIDATION_FAILED").is_empty(),
        "{:#?}",
        env.diagnostics
    );
}

#[test]
fn a_declaration_that_undersells_or_oversells_is_reported() {
    let mut b = create_bundle(None).data;
    let mut p = person_json(json!({"health": {"rhesus": {"value": "negative"}}}));
    p["axgf_version"] = json!("1.0");
    b["manifest"]["axgf"] = json!("1.1");
    b["persons"] = json!({UUID_A: p.clone()});
    let env = validate(&b.to_string());
    let m = codes(&env, "SPEC_VERSION_MISMATCH");
    assert!(
        m.len() == 1 && m[0].message.contains("declares axgf_version 1.0"),
        "{:#?}",
        env.diagnostics
    );

    // An entity newer than its bundle.
    p["axgf_version"] = json!("1.1");
    b["manifest"]["axgf"] = json!("1.0");
    b["persons"] = json!({UUID_A: p});
    let env = validate(&b.to_string());
    let m = codes(&env, "SPEC_VERSION_MISMATCH");
    assert!(
        m.len() == 1 && m[0].message.contains("newer than the bundle's manifest"),
        "{:#?}",
        env.diagnostics
    );
    // Said once: the 1.0 schema's own refusal of "1.1" is the same finding.
    assert!(
        codes(&env, "SCHEMA_VALIDATION_FAILED").is_empty(),
        "{:#?}",
        env.diagnostics
    );
    // …and its 1.1 content is still checked, because the checks run on
    // content, not on what the manifest claims.
    let mut q = person_json(json!({"health": {"rhesus": {"value": "sideways"}}}));
    q["axgf_version"] = json!("1.1");
    b["persons"] = json!({UUID_A: q});
    assert_eq!(
        codes(&validate(&b.to_string()), "OUT_OF_VOCABULARY").len(),
        1
    );
}

#[test]
fn an_exported_1_1_bundle_carries_the_1_1_schema_and_a_1_0_bundle_the_1_0_one() {
    use std::io::{Cursor, Read};
    let names = |b64: &str| -> Vec<String> {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        let mut ar = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..ar.len())
            .map(|i| {
                let mut f = ar.by_index(i).unwrap();
                let mut s = String::new();
                if f.name().starts_with("schema/") {
                    f.read_to_string(&mut s).unwrap();
                    assert!(s.contains("\"$defs\""), "the schema is written whole");
                }
                f.name().to_string()
            })
            .collect()
    };
    let v10 = export_bundle(&with_version("1.0"));
    let n = names(v10.data["zip_base64"].as_str().unwrap());
    assert!(
        n.contains(&"schema/axgf-1.0.schema.json".to_string())
            && !n.iter().any(|x| x.contains("1.1"))
    );

    let p = person_with(&[(
        registry::attribute("health.blood_group").unwrap(),
        json!("A"),
    )]);
    let v11 = export_bundle(&bundle_with(&p).to_string());
    let n = names(v11.data["zip_base64"].as_str().unwrap());
    assert!(
        n.contains(&"schema/axgf-1.1.schema.json".to_string()),
        "{n:?}"
    );
    assert!(
        !n.contains(&"schema/axgf-1.0.schema.json".to_string()),
        "{n:?}"
    );

    // Export raises a hand-built bundle that forgot to.
    let mut forgot = bundle_with(&p);
    forgot["manifest"]["axgf"] = json!("1.0");
    let env = export_bundle(&forgot.to_string());
    let n = names(env.data["zip_base64"].as_str().unwrap());
    assert!(
        n.contains(&"schema/axgf-1.1.schema.json".to_string()),
        "{n:?}"
    );
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(env.data["zip_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(import_bundle(&bytes).data["manifest"]["axgf"], "1.1");
}

#[test]
fn a_streaming_export_writes_the_schema_the_manifest_declares() {
    use std::io::Cursor;
    let p = person_with(&[(
        registry::attribute("genomics.y_haplogroup").unwrap(),
        json!({"major": "I", "subclade": "I-M253"}),
    )]);
    let mut out = Cursor::new(Vec::new());
    let env = axgf_rs::export_bundle_streaming(&bundle_with(&p).to_string(), &mut out, |_| Ok(()));
    assert_eq!(env.status, Status::Ok, "{:#?}", env.diagnostics);
    let ar = zip::ZipArchive::new(Cursor::new(out.into_inner())).unwrap();
    let names: Vec<&str> = ar.file_names().collect();
    assert!(names.contains(&"schema/axgf-1.1.schema.json"), "{names:?}");
}

#[test]
fn every_attribute_of_every_group_validates_clean_when_filled_correctly() {
    // The positive control for the whole registry at once: a person carrying
    // every attribute of a group, every field, draws no finding at all beyond
    // the dangling references its sample UUIDs make.
    for g in registry::GROUPS {
        if g.attributes.is_empty() {
            continue;
        }
        let p = fixtures::person_with_group(g.key, Fill::Everything);
        let env = run(&bundle_with(&p));
        let findings: Vec<_> = env
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() != "DANGLING_REFERENCE")
            .collect();
        assert!(findings.is_empty(), "{} drew {findings:#?}", g.key);
    }
}

#[test]
fn the_specifications_example_validates_clean() {
    let example: Value =
        serde_json::from_str(include_str!("fixtures/person-profile-1.1.json")).unwrap();
    let env = run(&bundle_with(&example));
    let findings: Vec<_> = env
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() != "DANGLING_REFERENCE")
        .collect();
    assert!(findings.is_empty(), "{findings:#?}");
    // Its uuids point at sources and documents the example does not include.
    assert!(!codes(&env, "DANGLING_REFERENCE").is_empty());
}
