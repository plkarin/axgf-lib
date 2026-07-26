// SPDX-License-Identifier: Apache-2.0
//! # wasm adapter — thin `wasm-bindgen` wrappers
//!
//! Each function here mirrors one of the crate's boundary functions
//! but takes and returns JSON strings so the JavaScript side can call
//! them directly without any custom `serde_wasm_bindgen`
//! serialization. All logic still lives in [`crate::logic`] and
//! [`crate::boundary`]; this adapter exists solely to expose the
//! calling convention JS expects.
//!
//! Enabled with `--features wasm`. The Rust callable form is
//! unchanged; downstream WASM builds do
//! `wasm-pack build --features wasm`.

use wasm_bindgen::prelude::*;

use crate::logic::crud::{DeletePolicy, EntityKind};

fn parse_kind(s: &str) -> Result<EntityKind, JsValue> {
    match s {
        "persons" | "person" => Ok(EntityKind::Person),
        "families" | "family" => Ok(EntityKind::Family),
        "events" | "event" => Ok(EntityKind::Event),
        "links" | "link" => Ok(EntityKind::Link),
        "occupations" | "occupation" => Ok(EntityKind::Occupation),
        "sources" | "source" => Ok(EntityKind::Source),
        "places" | "place" => Ok(EntityKind::Place),
        "documents" | "document" => Ok(EntityKind::Document),
        other => Err(JsValue::from_str(&format!("unknown entity kind {other:?}"))),
    }
}

fn parse_policy(s: &str) -> Result<DeletePolicy, JsValue> {
    match s {
        "reject" => Ok(DeletePolicy::Reject),
        "cascade" => Ok(DeletePolicy::Cascade),
        "orphan" => Ok(DeletePolicy::Orphan),
        other => Err(JsValue::from_str(&format!("unknown delete policy {other:?}"))),
    }
}

/// See [`crate::create_bundle`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = createBundle)]
pub fn create_bundle_wasm(family_name: Option<String>) -> String {
    crate::create_bundle(family_name.as_deref()).to_json()
}

/// See [`crate::import_bundle`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = importBundle)]
pub fn import_bundle_wasm(zip_bytes: &[u8]) -> String {
    crate::import_bundle(zip_bytes).to_json()
}

/// See [`crate::export_bundle`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = exportBundle)]
pub fn export_bundle_wasm(flat_json: &str) -> String {
    crate::export_bundle(flat_json).to_json()
}

/// See [`crate::inspect`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = inspect)]
pub fn inspect_wasm(flat_json: &str) -> String {
    crate::inspect(flat_json).to_json()
}

/// See [`crate::validate`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = validate)]
pub fn validate_wasm(flat_json: &str) -> String {
    crate::validate(flat_json).to_json()
}

/// See [`crate::add_entity`]. `kind` is the plural collection name
/// (`"persons"`, `"families"`, …) or its singular. Returns a
/// JSON-serialized envelope.
#[wasm_bindgen(js_name = addEntity)]
pub fn add_entity_wasm(flat_json: &str, kind: &str, entity_json: &str) -> String {
    match parse_kind(kind) {
        Ok(k) => crate::add_entity(flat_json, k, entity_json).to_json(),
        Err(e) => e.as_string().unwrap_or_default(),
    }
}

/// See [`crate::update_entity`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = updateEntity)]
pub fn update_entity_wasm(flat_json: &str, kind: &str, entity_json: &str) -> String {
    match parse_kind(kind) {
        Ok(k) => crate::update_entity(flat_json, k, entity_json).to_json(),
        Err(e) => e.as_string().unwrap_or_default(),
    }
}

/// See [`crate::delete_entity`]. `policy` is one of `"reject"`,
/// `"cascade"`, `"orphan"`. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = deleteEntity)]
pub fn delete_entity_wasm(flat_json: &str, kind: &str, id: &str, policy: &str) -> String {
    let k = match parse_kind(kind) {
        Ok(k) => k,
        Err(e) => return e.as_string().unwrap_or_default(),
    };
    let p = match parse_policy(policy) {
        Ok(p) => p,
        Err(e) => return e.as_string().unwrap_or_default(),
    };
    crate::delete_entity(flat_json, k, id, p).to_json()
}

/// See [`crate::deduplicate`]. Returns a JSON-serialized envelope.
#[wasm_bindgen(js_name = deduplicate)]
pub fn deduplicate_wasm(flat_json: &str) -> String {
    crate::deduplicate(flat_json).to_json()
}

/// See [`crate::convert_gedcom`]. Returns a JSON-serialized envelope.
#[cfg(feature = "gedcom")]
#[wasm_bindgen(js_name = convertGedcom)]
pub fn convert_gedcom_wasm(
    gedcom_bytes: &[u8],
    default_confidence: f64,
    place_lang: &str,
) -> String {
    crate::convert_gedcom(gedcom_bytes, default_confidence, place_lang).to_json()
}
