// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Event`] entity, mirroring
//! `#/$defs/event` in the schema and SPEC §4.3.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, BaseEntity, DocumentLink, Extra};

/// A participant in an [`Event`]. Mirrors `event.participants[]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParticipant {
    /// Referenced entity kind (`person`, `family`, `organization`).
    pub entity_type: String,
    /// UUID of the participant entity.
    pub entity_id: String,
    /// Role in the event (`spouse_1`, `witness`, …).
    pub role: String,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// AI metadata attached to an Event. Mirrors `event.ai`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventAi {
    /// Relative path to the Markdown vault page for this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_page: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An Event entity. See SPEC §4.3 for the field-by-field contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Event category (`birth`, `marriage`, `military`, …).
    pub category: String,
    /// Optional subcategory (e.g. `civil` for a marriage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    /// Date of the event.
    pub date: AxgfDate,
    /// Referenced [`crate::model::place::Place`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Participants in the event.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<EventParticipant>,
    /// Free-form description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Documents attached to this event.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<DocumentLink>,
    /// Overall confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// AI metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<EventAi>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
