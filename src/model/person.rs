// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Person`] entity, mirroring
//! `#/$defs/person` in the schema and SPEC §4.1.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, AxgfName, BaseEntity, DocumentLink, Extra, AiHypothesis};

/// A person's gender declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Gender {
    /// One of `M`, `F`, `NB`, `U` (unknown).
    pub value: String,
    /// Optional explanatory note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Identity block (name, alternate names, gender, is-living flag, visibility).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Identity {
    /// Primary display name.
    pub name: AxgfName,
    /// Alternate names (birth name, aliases, transliterations, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<AxgfName>,
    /// Gender information.
    pub gender: Gender,
    /// Whether the person is currently living (drives privacy handling).
    pub is_living: bool,
    /// Visibility level: `public | members | contributors | private`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Vital event block (birth or death), attached directly to a Person.
/// A first-class [`crate::model::event::Event`] MAY additionally exist and
/// be linked via `event_id`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Vital {
    /// Date of the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Referenced [`crate::model::place::Place`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Death-specific: cause of death (unused for birth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// Overall confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Referenced first-class [`crate::model::event::Event`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// AI metadata attached to a Person. Mirrors `person.ai`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonAi {
    /// Relative path to the Markdown vault page for this person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_page: Option<String>,
    /// Embedding model identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Last time the embedding was recomputed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_updated_at: Option<String>,
    /// Machine-generated hypotheses about this person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hypotheses: Vec<AiHypothesis>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A Person entity. See SPEC §4.1 for the full field-by-field contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    /// Base entity fields (`id`, `type`, `axgf_version`, timestamps, …).
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Identity block (name, gender, visibility).
    pub identity: Identity,
    /// Birth vitals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth: Option<Vital>,
    /// Death vitals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<Vital>,
    /// Free-form biography.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    /// Free-form notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Documents attached to this person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<DocumentLink>,
    /// AI metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<PersonAi>,
    /// Forward-compatible extras — anything else the spec adds later.
    #[serde(flatten)]
    pub extra: Extra,
}
