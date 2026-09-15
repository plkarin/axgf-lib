// SPDX-License-Identifier: Apache-2.0
//! The registry and the schema say the same thing.
//!
//! `model::profile` restates `schema/axgf-1.1.schema.json` as Rust data, and a
//! restatement is a second source of truth waiting to disagree with the first.
//! These tests read the embedded schema and hold every attribute, cardinality,
//! class, vocabulary, artefact restriction and national rank list in the
//! registry against it — in both directions, so an attribute the schema gains
//! and the registry does not is a failure too.

use std::collections::BTreeSet;

use axgf_rs::boundary::lifecycle::{schema_for, EMBEDDED_SCHEMA, EMBEDDED_SCHEMA_1_1};
use axgf_rs::model::profile::registry::{self, Cardinality, SensitiveClass, Shape};
use axgf_rs::model::profile::vocab;
use serde_json::Value;

fn schema() -> Value {
    serde_json::from_str(EMBEDDED_SCHEMA_1_1).expect("the 1.1 schema is JSON")
}

/// Follow `{"$ref": "#/$defs/x"}` to the definition.
fn resolve<'a>(root: &'a Value, node: &'a Value) -> &'a Value {
    match node.get("$ref").and_then(Value::as_str) {
        Some(r) => {
            let name = r.trim_start_matches("#/$defs/");
            &root["$defs"][name]
        }
        None => node,
    }
}

/// The schema node for one attribute, starting from the person definition.
fn attribute_node<'a>(root: &'a Value, block: &str, key: &str) -> Option<&'a Value> {
    let person = &root["$defs"]["person"]["properties"];
    let block_node = resolve(root, person.get(block)?);
    block_node.get("properties")?.get(key)
}

#[test]
fn every_registry_attribute_is_in_the_schema_with_its_cardinality_and_class() {
    let root = schema();
    for a in registry::attributes() {
        let node = attribute_node(&root, a.block, a.key)
            .unwrap_or_else(|| panic!("{} is in the registry but not the schema", a.path));
        let is_series = node.get("type") == Some(&Value::from("array"));
        assert_eq!(
            is_series,
            a.cardinality == Cardinality::Series,
            "{} cardinality disagrees",
            a.path
        );
        let class = node.get("x-axgf-class").and_then(Value::as_str);
        assert_eq!(
            class,
            a.class.map(SensitiveClass::as_str),
            "{} class disagrees",
            a.path
        );
    }
}

#[test]
fn every_schema_attribute_of_a_1_1_block_is_in_the_registry() {
    let root = schema();
    let person = &root["$defs"]["person"]["properties"];
    for block in registry::PROFILE_BLOCKS {
        let node = resolve(&root, &person[*block]);
        let keys: BTreeSet<&str> = node["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{block} has no properties"))
            .keys()
            .map(String::as_str)
            .collect();
        for key in keys {
            let path = format!("{block}.{key}");
            assert!(
                registry::attribute(&path).is_some(),
                "the schema defines {path}, the registry does not"
            );
        }
    }
}

#[test]
fn every_vocabulary_is_the_schemas_enumeration_term_for_term() {
    let root = schema();
    let defs = root["$defs"].as_object().unwrap();
    let schema_vocabs: BTreeSet<String> = defs
        .keys()
        .filter_map(|k| k.strip_prefix("vocab_").map(str::to_string))
        .collect();
    let registry_vocabs: BTreeSet<String> = vocab::ALL.iter().map(|v| v.name.to_string()).collect();
    assert_eq!(schema_vocabs, registry_vocabs, "the vocabulary sets differ");
    for v in vocab::ALL {
        let terms: Vec<&str> = defs[&format!("vocab_{}", v.name)]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        assert_eq!(terms, v.terms, "vocabulary {} differs", v.name);
    }
}

/// The vocabulary a schema node names, if it names one.
fn vocab_ref(node: &Value) -> Option<&str> {
    node.get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.strip_prefix("#/$defs/vocab_"))
}

#[test]
fn every_value_shape_names_the_vocabulary_the_schema_does() {
    let root = schema();
    for a in registry::attributes() {
        let node = attribute_node(&root, a.block, a.key).unwrap();
        let claim = if a.cardinality == Cardinality::Series {
            &node["items"]
        } else {
            node
        };
        let value = &claim["properties"]["value"];
        match &a.shape {
            Shape::Vocab(v) => assert_eq!(vocab_ref(value), Some(v.name), "{}", a.path),
            Shape::Object { fields, .. } => {
                for f in fields.iter() {
                    let fnode = &value["properties"][f.key];
                    assert!(!fnode.is_null(), "{}.{} missing from schema", a.path, f.key);
                    if let Shape::Vocab(v) = &f.shape {
                        assert_eq!(vocab_ref(fnode), Some(v.name), "{}.{}", a.path, f.key);
                    }
                    let required = value["required"]
                        .as_array()
                        .is_some_and(|r| r.iter().any(|k| k == f.key));
                    assert_eq!(required, f.required, "{}.{} required", a.path, f.key);
                }
            }
            Shape::Artefact(types) => {
                let allowed: Vec<&str> = value["properties"]["artefact_type"]["enum"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t.as_str().unwrap())
                    .collect();
                assert_eq!(&allowed, types, "{} artefact types", a.path);
            }
            _ => {}
        }
    }
}

#[test]
fn every_national_rank_list_is_the_one_the_schema_selects_by_country() {
    let root = schema();
    let ranks = attribute_node(&root, "military", "ranks").unwrap();
    let rules = ranks["items"]["properties"]["value"]["allOf"]
        .as_array()
        .expect("the rank rules");
    for rv in vocab::MILITARY_RANKS {
        let rule = rules
            .iter()
            .find(|r| r["if"]["properties"]["country"]["const"] == rv.country)
            .unwrap_or_else(|| panic!("no schema rule selects {}", rv.country));
        assert_eq!(
            vocab_ref(&rule["then"]["properties"]["rank"]),
            Some(rv.vocabulary.name)
        );
        let terms: Vec<&str> = rv.ranks.iter().map(|r| r.term).collect();
        assert_eq!(terms, rv.vocabulary.terms, "{} rank terms", rv.country);
        for r in rv.ranks {
            assert!(
                vocab::RANK_CATEGORY.contains(r.category),
                "{} {} has category {}",
                rv.country,
                r.term,
                r.category
            );
        }
    }
}

#[test]
fn every_panel_lists_only_analytes_of_the_analyte_vocabulary() {
    let mut seen = BTreeSet::new();
    for (panel, analytes) in vocab::LAB_PANEL_ANALYTES {
        assert!(vocab::LAB_PANEL.contains(panel), "{panel}");
        for a in *analytes {
            assert!(vocab::LAB_ANALYTE.contains(a), "{panel}: {a}");
            seen.insert(*a);
        }
    }
    assert_eq!(
        seen.len(),
        vocab::LAB_ANALYTE.terms.len(),
        "an analyte belongs to no panel"
    );
}

#[test]
fn the_classes_are_the_four_the_specification_names() {
    let names: Vec<&str> = SensitiveClass::ALL.iter().map(|c| c.as_str()).collect();
    assert_eq!(names, vocab::SENSITIVE_CLASS.terms);
    for c in SensitiveClass::ALL {
        assert_eq!(SensitiveClass::parse(c.as_str()), Some(c));
        assert!(
            registry::attributes_of_class(c).next().is_some(),
            "no attribute is {}",
            c.as_str()
        );
    }
}

#[test]
fn both_schemas_compile_for_every_kind() {
    use jsonschema::JSONSchema;
    for (version, text) in [("1.0", EMBEDDED_SCHEMA), ("1.1", EMBEDDED_SCHEMA_1_1)] {
        assert_eq!(schema_for(version).map(|(_, t)| t), Some(text));
        let root: Value = serde_json::from_str(text).unwrap();
        for kind in [
            "manifest",
            "person",
            "family",
            "event",
            "link",
            "occupation",
            "source",
            "place",
            "document",
        ] {
            let wrapper =
                serde_json::json!({"$defs": root["$defs"], "$ref": format!("#/$defs/{kind}")});
            assert!(
                JSONSchema::compile(&wrapper).is_ok(),
                "{version} {kind} does not compile"
            );
        }
    }
    assert!(schema_for("1.2").is_none());
}

#[test]
fn the_registry_has_the_fourteen_groups_in_order() {
    let keys: Vec<&str> = registry::GROUPS.iter().map(|g| g.key).collect();
    assert_eq!(
        keys,
        [
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
            "relationships",
            "digital_legacy"
        ]
    );
    assert_eq!(registry::attributes().count(), 132);
    assert_eq!(vocab::ALL.len(), 102);
    let relationships = registry::group("relationships").unwrap();
    assert!(
        relationships.attributes.is_empty() && !relationships.mapped.is_empty(),
        "relationships live on Family and Link, never on the person"
    );
}
