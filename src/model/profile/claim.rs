// SPDX-License-Identifier: Apache-2.0
//! The claim (SPEC_1.1 §3.1) and the shared value shapes (§3.5).
//!
//! A claim is one statement about a person with its evidence, and it is built
//! from nothing new: the date is 1.0's [`AxgfDate`], a period's bounds are the
//! `{ date, event_id }` shape 1.0 Links already use, and the source and
//! confidence are the `source_id` and `confidence` every 1.0 fact carries.

use serde::{Deserialize, Serialize};

use crate::model::common::{AxgfDate, Extra};

/// The start or end of the period a claim holds for. The same shape as a
/// Link's `valid_from` / `valid_until` (1.0 §4.4), and the same type.
pub type ClaimBound = crate::model::link::LinkValidity;

/// A position. The same shape as a Place's `coordinates` (1.0 §5.3), and the
/// same type.
pub type Coordinates = crate::model::place::Coordinates;

/// One dated, sourced, confidence-bearing claim. See SPEC_1.1 §3.1.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Claim<T> {
    /// What is claimed.
    pub value: T,
    /// The moment the claim describes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// The start of a period the claim holds for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<ClaimBound>,
    /// The end of that period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<ClaimBound>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Referenced [`crate::model::event::Event`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Confidence in \[0.0, 1.0\]; absent means unknown (1.0 §8.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Free text: the source's own wording, a conversion, a doubt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A reference to a Document holding an artefact (SPEC_1.1 §3.5.1).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtefactRef {
    /// Referenced [`crate::model::document::Document`] UUID.
    pub document_id: String,
    /// A term of [`super::vocab::ARTEFACT_TYPE`], restricted per attribute.
    pub artefact_type: String,
    /// The file format by its usual name: `OBJ`, `FLAC`, `GGUF`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// The software, device or model that produced it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    /// Another Document this one was made from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from_id: Option<String>,
    /// A term of [`super::vocab::CONSENT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
