// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Source`] entity, mirroring
//! `#/$defs/source` in the schema and SPEC §5.4.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, BaseEntity, Extra};

/// The archive or place holding a source. Mirrors `source.repository`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Repository {
    /// Name of the archive / library / institution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Physical location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Optional URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Local shelfmark or catalog reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A recorded disagreement between two sources on the same fact. Mirrors
/// `source.conflicts[]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConflict {
    /// UUID of the other source.
    pub source_id: String,
    /// Field the two sources disagree on.
    pub field: String,
    /// Value asserted by *this* source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub this_value: Option<String>,
    /// Value asserted by the other source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_value: Option<String>,
    /// Resolution outcome (`this_preferred | other_preferred | unresolved`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    /// Free-form note explaining the resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// DNA-match block on a DNA source. Mirrors `source.dna.match`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DnaMatch {
    /// Referenced [`crate::model::person::Person`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// Shared centimorgans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_cm: Option<f64>,
    /// Shared percent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_percent: Option<f64>,
    /// Predicted relationship (`first_cousin`, `parent_child`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_relationship: Option<String>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// DNA metadata attached to a DNA source. Mirrors `source.dna`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dna {
    /// Test provider (e.g. `23andMe`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_provider: Option<String>,
    /// Test kind (`autosomal | y_dna | mt_dna | x_dna`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_type: Option<String>,
    /// Date the test was performed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_date: Option<String>,
    /// Anonymized kit identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit_id: Option<String>,
    /// Match record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "match")]
    pub match_: Option<DnaMatch>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A Source entity — an evidence record justifying factual claims.
/// See SPEC §5.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Human-readable title.
    pub title: String,
    /// One of the SPEC §5.4.1 source-type values.
    pub source_type: String,
    /// One of `primary | secondary | derivative | authored | oral | unknown`.
    pub reliability: String,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Verification status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Repository (archive / library) holding the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,
    /// Date the source was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Referenced [`crate::model::place::Place`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Referenced [`crate::model::document::Document`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Recorded disagreements with other sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<SourceConflict>,
    /// Full transcription text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription: Option<String>,
    /// BCP 47 language of the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Script (`latin`, `cyrillic`, `hebrew`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// DNA metadata (present only when `source_type == "dna"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dna: Option<Dna>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
