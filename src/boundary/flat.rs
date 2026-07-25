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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub persons: BTreeMap<String, Value>,
    /// Family entities keyed by lowercase UUID v4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub families: BTreeMap<String, Value>,
    /// Event entities keyed by lowercase UUID v4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub events: BTreeMap<String, Value>,
    /// Link entities keyed by lowercase UUID v4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, Value>,
    /// Occupation entities keyed by lowercase UUID v4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub occupations: BTreeMap<String, Value>,
    /// Source entities keyed by lowercase UUID v4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, Value>,
    /// Place entities keyed by lowercase UUID v4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub places: BTreeMap<String, Value>,
    /// Document metadata entities keyed by lowercase UUID v4. Binary
    /// payloads are handled at ZIP boundary time, not here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub documents: BTreeMap<String, Value>,

    /// Forward-compatibility bucket: any top-level field the current
    /// implementation does not understand round-trips unchanged.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
