// SPDX-License-Identifier: Apache-2.0
//! # flat — the working-form flat bundle representation
//!
//! The on-disk `.axgf` file is a ZIP with one JSON file per entity. Editing
//! that in place would require per-entity file I/O and is awkward across
//! language bindings, so every operation in this crate works on a **single
//! flat JSON object**:
//!
//! ```json
//! {
//!   "manifest":   { ... },
//!   "persons":    { "<uuid>": { ... }, ... },
//!   "families":   { "<uuid>": { ... }, ... },
//!   "events":     { "<uuid>": { ... }, ... },
//!   "links":      { "<uuid>": { ... }, ... },
//!   "occupations":{ "<uuid>": { ... }, ... },
//!   "sources":    { "<uuid>": { ... }, ... },
//!   "places":     { "<uuid>": { ... }, ... },
//!   "documents":  { "<uuid>": { ... }, ... }
//! }
//! ```
//!
//! [`FlatBundle`] uses [`BTreeMap`] throughout to keep serialization order
//! deterministic (bundles diff cleanly). Unknown top-level fields are
//! preserved via `#[serde(flatten)]` on [`FlatBundle::extra`] so a
//! newer-spec bundle passing through a V1 operation cannot lose data.
//!
//! Filled in during Phase 1.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A working-form AXGF bundle: manifest plus one map per entity kind.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlatBundle {
    /// Manifest as a raw JSON value; typed access lives in
    /// [`crate::model::manifest`].
    #[serde(default)]
    pub manifest: Value,

    /// Person entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub persons: BTreeMap<String, Value>,
    /// Family entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub families: BTreeMap<String, Value>,
    /// Event entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub events: BTreeMap<String, Value>,
    /// Link entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub links: BTreeMap<String, Value>,
    /// Occupation entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub occupations: BTreeMap<String, Value>,
    /// Source entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub sources: BTreeMap<String, Value>,
    /// Place entities keyed by lowercase UUID v4.
    #[serde(default)]
    pub places: BTreeMap<String, Value>,
    /// Document metadata entities keyed by lowercase UUID v4. Binary
    /// payloads live in [`FlatBundle::attachments`], indexed by ZIP path
    /// (e.g. `documents/files/{uuid}.pdf`).
    #[serde(default)]
    pub documents: BTreeMap<String, Value>,

    /// Auxiliary files that are part of the bundle but not modeled as
    /// entities: document binary payloads under `documents/files/…`,
    /// vault Markdown pages under `vault/…`, and any other files
    /// present in the source ZIP. Keys are ZIP paths; values are
    /// base64-encoded bytes.
    ///
    /// Populated by [`crate::import_bundle`] and written back verbatim
    /// by [`crate::export_bundle`]. `manifest.json` and everything under
    /// `schema/` and the eight entity directories are handled
    /// structurally and MUST NOT appear here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attachments: BTreeMap<String, String>,

    /// Forward-compatibility bucket: any top-level field the current
    /// implementation does not understand round-trips unchanged.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_bundle_serializes_with_all_eight_collections_present() {
        // Collections always appear (as `{}`) so bindings can index
        // `bundle.persons` unconditionally without a null-check.
        let b = FlatBundle::default();
        let v = serde_json::to_value(&b).unwrap();
        assert!(v["manifest"].is_null());
        for key in [
            "persons",
            "families",
            "events",
            "links",
            "occupations",
            "sources",
            "places",
            "documents",
        ] {
            assert!(
                v[key].as_object().map(|o| o.is_empty()).unwrap_or(false),
                "expected empty {key} object, got {:?}",
                v[key]
            );
        }
        // Attachments stays skipped when empty (bundles rarely carry any).
        assert!(v.get("attachments").is_none());
    }

    #[test]
    fn bundle_round_trip_preserves_ordered_persons() {
        let mut b = FlatBundle {
            manifest: json!({"axgf": "1.0", "created_at": "2026-06-15T10:00:00Z"}),
            ..Default::default()
        };
        b.persons.insert("aaa".into(), json!({"name": "A"}));
        b.persons.insert("bbb".into(), json!({"name": "B"}));

        let wire = serde_json::to_string(&b).unwrap();
        let parsed: FlatBundle = serde_json::from_str(&wire).unwrap();

        assert_eq!(parsed.persons.len(), 2);
        // BTreeMap orders keys — deterministic across runs.
        let keys: Vec<&String> = parsed.persons.keys().collect();
        assert_eq!(keys, vec!["aaa", "bbb"]);
        assert_eq!(parsed.manifest["axgf"], "1.0");
    }

    #[test]
    fn unknown_top_level_fields_survive_round_trip() {
        // A newer-spec bundle carrying fields V1 does not understand.
        // The forward-compat contract says these MUST NOT be dropped.
        let input = json!({
            "manifest": {"axgf": "2.0", "stats": {"persons": 0}, "future_field": 42},
            "persons": {},
            "future_collection": {"xyz": {"a": 1}},
            "another_top_level": "hello"
        });
        let parsed: FlatBundle = serde_json::from_value(input.clone()).unwrap();

        assert!(parsed.extra.contains_key("future_collection"));
        assert_eq!(parsed.extra["another_top_level"], json!("hello"));
        assert_eq!(parsed.manifest["future_field"], json!(42));

        let out = serde_json::to_value(&parsed).unwrap();
        assert_eq!(out["another_top_level"], json!("hello"));
        assert_eq!(out["future_collection"]["xyz"]["a"], json!(1));
        assert_eq!(out["manifest"]["future_field"], json!(42));
    }

    #[test]
    fn missing_collections_deserialize_as_empty() {
        // Per SPEC §2.2 partial bundles are valid; a bundle with only
        // persons should parse without failing.
        let input = json!({"manifest": {"axgf": "1.0"}, "persons": {"p1": {"x": 1}}});
        let parsed: FlatBundle = serde_json::from_value(input).unwrap();
        assert_eq!(parsed.persons.len(), 1);
        assert!(parsed.families.is_empty());
        assert!(parsed.events.is_empty());
        assert!(parsed.documents.is_empty());
    }
}
