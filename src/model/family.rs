// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Family`] entity, mirroring
//! `#/$defs/family` in the schema and SPEC §4.2.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, BaseEntity, DocumentLink, Extra};

/// A person's role in a family union (typically `spouse`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionPerson {
    /// Referenced [`crate::model::person::Person`] UUID.
    pub person_id: String,
    /// Role of that person (e.g. `spouse`, `witness`).
    pub role: String,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Optional start date/place for a union. Mirrors `family.union.start`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnionStart {
    /// Date of the union.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Referenced [`crate::model::place::Place`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Referenced first-class [`crate::model::event::Event`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Optional end date/reason for a union. Mirrors `family.union.end`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnionEnd {
    /// Date the union ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Reason (`death_of_spouse`, `divorce`, `separation`, `annulment`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The union block on a family. Mirrors `family.union`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Union {
    /// Union kind (`marriage`, `civil_union`, `cohabitation`,
    /// `religious_only`, `polygamous`, `unknown`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Status (`active`, `ended_by_death`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Persons taking part in the union (at least one spouse).
    pub persons: Vec<UnionPerson>,
    /// Optional start date/place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<UnionStart>,
    /// Optional end date/reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<UnionEnd>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Forward-compatible extras — captures polygamous-only fields
    /// (`primary_person_id`, `unions[]`) as-is.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A child entry inside a family. Mirrors an item of `family.children[]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyChild {
    /// Referenced [`crate::model::person::Person`] UUID.
    pub person_id: String,
    /// Birth order within the family (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_order: Option<i32>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Free-form note (e.g. adoption pedigree).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// AI metadata attached to a Family. Mirrors `family.ai`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FamilyAi {
    /// Relative path to the Markdown vault page for this family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_page: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A Family entity. See SPEC §4.2 for the full field-by-field contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Family {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Human-readable family name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Short description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The union defining the family.
    pub union: Union,
    /// Children of the union.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FamilyChild>,
    /// Documents attached to this family.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<DocumentLink>,
    /// Free-form notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// AI metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<FamilyAi>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
