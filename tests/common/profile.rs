// SPDX-License-Identifier: Apache-2.0
//! Sample values built from the AXGF 1.1 registry, for the profile tests.
//!
//! Building fixtures from the registry rather than writing them out means a
//! test that iterates "every attribute" really does, including the ones added
//! after the test was written — and a typed struct that has lost a field
//! shows up as a failed round trip rather than as a fixture nobody updated.

#![allow(dead_code)]

use axgf_rs::model::profile::registry::{self, Attribute, Cardinality, Shape};
use serde_json::{json, Map, Value};

pub const UUID_A: &str = "7c9e6679-7425-40de-944b-e07fc1f90ae7";
pub const UUID_B: &str = "0f8fad5b-d9cb-469f-a165-70867728950e";

/// How much of an object to fill.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// Required fields, and the first group of every `one_of`.
    Minimal,
    /// Every field.
    Everything,
}

/// A valid sample for one shape.
pub fn sample(shape: &Shape, fill: Fill) -> Value {
    match shape {
        Shape::Text => json!("sample text"),
        Shape::Number { min, max, .. } => {
            let lo = min.unwrap_or(0.0);
            let hi = max.unwrap_or(lo + 10.0);
            number((lo + hi) / 2.0)
        }
        Shape::Integer { min, max, .. } => {
            let lo = min.unwrap_or(0);
            let hi = max.unwrap_or(lo + 10);
            json!((lo + hi) / 2)
        }
        Shape::Boolean => json!(true),
        Shape::Uuid => json!(UUID_B),
        Shape::TimeOfDay => json!("05:40"),
        Shape::Coordinates => json!({"lat": 50.06, "lon": 19.94, "precision": "grave"}),
        Shape::LanguageTag => json!("pl-PL"),
        Shape::CurrencyCode => json!("PLN"),
        Shape::Vocab(v) => json!(v.terms[0]),
        // The Y-DNA pattern is the one with ISOGG's `-SNP` shorthand in it.
        Shape::Pattern(p) if p.contains("-[A-Z]") => json!("R-M269"),
        Shape::Pattern(_) => json!("R0a"),
        Shape::Rank => json!("kapral"),
        Shape::Artefact(types) => json!({
            "document_id": UUID_B, "artefact_type": types[0], "format": "OBJ",
            "generator": "sample", "derived_from_id": UUID_A, "consent": "given"
        }),
        Shape::Object { fields, one_of } => {
            let mut m = Map::new();
            for f in fields.iter() {
                let wanted = f.required
                    || fill == Fill::Everything
                    || one_of.first().is_some_and(|g| g.contains(&f.key));
                if wanted {
                    m.insert(f.key.to_string(), sample(&f.shape, fill));
                }
            }
            // The fields whose valid value depends on a sibling: a rank needs
            // the country whose vocabulary it comes from, and its category
            // has to be that rank's; a subclade has to sit under its major.
            if m.contains_key("rank") {
                m.insert("country".into(), json!("PL"));
                if m.contains_key("category") {
                    m.insert("category".into(), json!("non_commissioned"));
                }
            }
            if m.contains_key("major") {
                m.insert("major".into(), json!("R"));
            }
            if let Some(Value::String(clock)) = m.get("clock") {
                if clock == "dunedinpace" {
                    m.insert("pace".into(), json!(0.95));
                }
            }
            if fill == Fill::Everything && m.contains_key("clock") {
                // Everything fills both results; horvath takes age_years.
                m.insert("clock".into(), json!("horvath"));
            }
            Value::Object(m)
        }
    }
}

fn number(x: f64) -> Value {
    if x.fract() == 0.0 {
        json!(x as i64)
    } else {
        json!(x)
    }
}

/// One claim holding `value`, with every provenance field filled.
pub fn claim(value: Value) -> Value {
    json!({
        "value": value,
        "date": {"value": "1950-06", "precision": "month"},
        "valid_from": {"date": {"value": "1949", "precision": "year"}},
        "valid_until": {"date": {"value": "1951", "precision": "year"}},
        "source_id": UUID_B,
        "event_id": UUID_B,
        "confidence": 0.85,
        "note": "sample note"
    })
}

/// The attribute as it sits in its block: a claim, or a series of one.
pub fn held(a: &Attribute, value: Value) -> Value {
    match a.cardinality {
        Cardinality::Single => claim(value),
        Cardinality::Series => json!([claim(value)]),
    }
}

/// A minimal 1.1 person carrying `attributes`, each with its sample value.
pub fn person_with(attrs: &[(&Attribute, Value)]) -> Value {
    let mut p = json!({
        "id": UUID_A, "type": "person", "axgf_version": "1.1",
        "identity": {"name": {"display": "Sample", "components": []},
                     "gender": {"value": "U"}, "is_living": false}
    });
    for (a, value) in attrs {
        let block = p
            .as_object_mut()
            .unwrap()
            .entry(a.block)
            .or_insert_with(|| json!({}));
        block[a.key] = held(a, value.clone());
    }
    p
}

/// A person carrying every attribute of one group, every field filled.
pub fn person_with_group(group: &str, fill: Fill) -> Value {
    let g = registry::group(group).expect("group");
    let attrs: Vec<(&Attribute, Value)> = g
        .attributes
        .iter()
        .map(|a| (a, sample(&a.shape, fill)))
        .collect();
    person_with(&attrs)
}

/// A flat 1.1 bundle holding one person.
pub fn bundle_with(person: &Value) -> Value {
    json!({
        "manifest": {"axgf": "1.1", "created_at": "2026-09-15T09:00:00Z",
                     "stats": {"persons": 1}},
        "persons": {UUID_A: person},
        "families": {}, "events": {}, "links": {}, "occupations": {},
        "sources": {}, "places": {}, "documents": {}
    })
}

/// Every place a vocabulary term can sit in a person: the attribute, the
/// field inside its value (`None` when the value itself is the term), and the
/// vocabulary's name.
pub fn vocabulary_locations() -> Vec<(&'static Attribute, Option<&'static str>, &'static str)> {
    let mut out = Vec::new();
    for a in registry::attributes() {
        match &a.shape {
            Shape::Vocab(v) => out.push((a, None, v.name)),
            Shape::Object { fields, .. } => {
                for f in fields.iter() {
                    if let Shape::Vocab(v) = &f.shape {
                        out.push((a, Some(f.key), v.name));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// JSON-subset check: every leaf of `expected` appears at the same path in
/// `actual`, with the same value. Numbers compare by their JSON spelling, so
/// `158` and `158.0` are different — which is the point.
pub fn assert_contains(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, v) in e {
                let av = a.get(k).unwrap_or_else(|| {
                    panic!("missing {path}/{k} after the round trip; had {a:?}")
                });
                assert_contains(v, av, &format!("{path}/{k}"));
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(e.len(), a.len(), "array length changed at {path}");
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                assert_contains(ev, av, &format!("{path}[{i}]"));
            }
        }
        (e, a) => assert_eq!(e.to_string(), a.to_string(), "value changed at {path}"),
    }
}
