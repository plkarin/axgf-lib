// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Occupation`] entity, mirroring
//! `#/$defs/occupation` in the schema and SPEC §4.5.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, BaseEntity, Extra};

/// Employer information attached to an [`Occupation`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Employer {
    /// Employer name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Referenced [`crate::model::place::Place`] UUID for the employer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Boundary date for an occupation (`valid_from` / `valid_until`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OccupationBoundary {
    /// The date at the boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An Occupation entity — a professional state attached to a person for
/// a time period. See SPEC §4.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Occupation {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Referenced [`crate::model::person::Person`] UUID.
    pub person_id: String,
    /// Occupation title in its native language.
    pub title: String,
    /// Latin transliteration or English name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_latin: Option<String>,
    /// Normalized title for classification (e.g. `teacher`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_normalized: Option<String>,
    /// Optional employer block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employer: Option<Employer>,
    /// Referenced [`crate::model::place::Place`] UUID where the person
    /// worked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Start of the occupation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<OccupationBoundary>,
    /// End of the occupation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<OccupationBoundary>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
