// SPDX-License-Identifier: Apache-2.0
//! Shared primitive types used across multiple AXGF entities.
//!
//! Types here mirror the `$defs` primitives in `schema/axgf-1.0.schema.json`:
//! [`AxgfName`], [`AxgfDate`], [`NameComponent`], [`EntityRef`],
//! [`DocumentLink`], [`AiHypothesis`], and the shared [`BaseEntity`] header
//! fields.
//!
//! Every struct terminates in `#[serde(flatten)] pub extra: Extra` so
//! forward-compatible unknown fields survive round-trips.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Generic bucket for unknown fields at any level of the entity tree.
/// A [`BTreeMap`] gives deterministic serialization order.
pub type Extra = BTreeMap<String, Value>;

/// A single component of a person's name, e.g. a given name or family name.
///
/// Mirrors `#/$defs/name_component` in the schema.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NameComponent {
    /// Component kind (`given_name`, `family_name`, `patronymic`, …).
    /// Kept as a string so future component types round-trip cleanly.
    #[serde(rename = "type")]
    pub kind: String,
    /// The component text in its native script.
    pub value: String,
    /// Latin transliteration of `value`, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_latin: Option<String>,
    /// Phonetic reading (hiragana, pinyin, IPA, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading: Option<String>,
    /// The system used for `reading`, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_system: Option<String>,
    /// Ordinal position of the component within the name, starting at 1.
    #[serde(default)]
    pub order: u32,
    /// ID of the person `value` was derived from, when this component is
    /// a patronymic or matronymic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,
    /// Type of derivation (`patronymic` or `matronymic`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation_type: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A structured person or place name. Mirrors `#/$defs/axgf_name`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxgfName {
    /// Human-readable full-name display string.
    pub display: String,
    /// Latin-transliterated `display`, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_latin: Option<String>,
    /// BCP 47 culture tag of the name (e.g. `fr`, `ja`, `he`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culture: Option<String>,
    /// Text direction: `ltr` or `rtl`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// Display order: `given_first` or `family_first`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_order: Option<String>,
    /// Phonetic reading of the display form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading: Option<String>,
    /// Reading system for `reading`, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_system: Option<String>,
    /// Individual name components.
    #[serde(default)]
    pub components: Vec<NameComponent>,
    /// Forward-compatible extras. Also captures name-list entry fields
    /// (`type`, `source_id`, `confidence`, `valid_from`, `valid_until`,
    /// `note`) that only apply when this name appears inside `names[]`
    /// on a Person.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An earliest / latest bracket for uncertain dates. Mirrors the `range`
/// object inside `#/$defs/axgf_date`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxgfDateRange {
    /// Earliest possible date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub earliest: Option<Box<AxgfDate>>,
    /// Latest possible date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<Box<AxgfDate>>,
    /// Free-form explanatory note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// One alternative rendering of a date, typically in another calendar.
/// Mirrors `alternatives[]` inside `#/$defs/axgf_date`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxgfDateAlternative {
    /// The date as it appears in the alternative calendar.
    pub value: String,
    /// Calendar system for `value`.
    pub calendar: String,
    /// Era name (e.g. `Taisho`), when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era: Option<String>,
    /// Era year, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era_year: Option<i32>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A calendar-aware date with precision and confidence. Mirrors
/// `#/$defs/axgf_date`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxgfDate {
    /// The date value, format depending on `precision` and `calendar`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Calendar system. Defaults to `gregorian` per spec §5.2.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<String>,
    /// One of the spec's precision values (`exact`, `month`, `year`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<String>,
    /// `true` when the date is approximate rather than certain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circa: Option<bool>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Free-form note (used by the GEDCOM importer to preserve
    /// unparseable date strings verbatim).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Alternative renderings (other calendars, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<AxgfDateAlternative>,
    /// Uncertainty range (e.g. `BET 1920 AND 1925`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<AxgfDateRange>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A typed reference from one entity to another. Mirrors
/// `#/$defs/entity_ref`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    /// Referenced entity kind (`person`, `family`, `event`, `link`,
    /// `source`).
    pub entity_type: String,
    /// UUID of the referenced entity.
    pub entity_id: String,
    /// Role played by the referenced entity (contextual).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Confidence in the reference itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Attachment of a document to another entity. Mirrors
/// `#/$defs/document_link`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLink {
    /// UUID of the document.
    pub document_id: String,
    /// Role of the document w.r.t. the containing entity (e.g. `photo`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Effective date this document represents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A single AI-generated hypothesis about an entity. Mirrors
/// `#/$defs/ai_hypothesis`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiHypothesis {
    /// Local id (unique within the containing entity's hypotheses).
    pub id: String,
    /// Human-readable claim.
    pub claim: String,
    /// Confidence in \[0.0, 1.0\].
    pub confidence: f64,
    /// One of `pending | confirmed | rejected | investigating`.
    pub status: String,
    /// Optional evidence supporting the claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// When the hypothesis was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// When the hypothesis was last human-reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    /// Reviewer id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Fields that appear on every AXGF entity. Every entity struct embeds
/// this via `#[serde(flatten)]`. Mirrors `#/$defs/base_entity`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseEntity {
    /// Lowercase UUID v4 identifier.
    pub id: String,
    /// Entity kind marker: `person`, `family`, etc.
    #[serde(rename = "type")]
    pub kind: String,
    /// AXGF spec version this entity was written against.
    pub axgf_version: String,
    /// Creation timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last-modification timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Monotonically increasing revision counter, starting at 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_num: Option<u32>,
    /// Identifier of the actor that created the entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Free-form tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Namespaced extensions (`x-{vendor}-{field}`) per spec §10.4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
}
