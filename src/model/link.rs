// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Link`] entity, mirroring
//! `#/$defs/link` in the schema and SPEC §4.4.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, BaseEntity, Extra};

/// One endpoint of a Link (`from` or `to`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEndpoint {
    /// Referenced entity kind (`person`, `family`, `event`).
    pub entity_type: String,
    /// UUID of the referenced entity.
    pub entity_id: String,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Temporal validity of a link. Mirrors `link.valid_from` /
/// `link.valid_until`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkValidity {
    /// The date at which the link's validity boundary occurs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Referenced [`crate::model::event::Event`] UUID (e.g. a baptism
    /// creates a godparent link).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A typed, directed relationship between two entities. See SPEC §4.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Source endpoint.
    pub from: LinkEndpoint,
    /// Target endpoint.
    pub to: LinkEndpoint,
    /// Human-readable label of the relationship.
    pub label: String,
    /// Label in the reverse direction (e.g. `parrain` ↔ `filleul`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_reverse: Option<String>,
    /// Optional category (`spiritual`, `professional`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// `true` if the relationship holds in both directions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bidirectional: Option<bool>,
    /// When the relationship became valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<LinkValidity>,
    /// When the relationship ceased to be valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<LinkValidity>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Visibility (`public | members | contributors | private`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
