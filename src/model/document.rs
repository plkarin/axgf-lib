// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Document`] entity, mirroring
//! `#/$defs/document` in the schema and SPEC §5.5.

use serde::{Deserialize, Serialize};

use super::common::{AxgfDate, BaseEntity, Extra};

/// File-metadata block on a Document. Mirrors `document.file`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentFile {
    /// Path inside the ZIP, e.g. `documents/files/doc-001.pdf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Size in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// SHA-256 hex digest (64 lowercase hex chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Reference from a Document to another entity. Mirrors `document.linked_to[]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLinkedTo {
    /// Referenced entity kind (`person`, `family`, `event`, `source`).
    pub entity_type: String,
    /// UUID of the referenced entity.
    pub entity_id: String,
    /// Role of the entity w.r.t. this document (`subject`, `evidence`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// OCR text extracted from a Document. Mirrors `document.ocr`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentOcr {
    /// Extracted plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// BCP 47 language of the OCR text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// OCR engine identifier (e.g. `tesseract-5`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// AI-suggested link from a Document to another entity. Mirrors an item
/// of `document.ai.suggested_links[]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSuggestedLink {
    /// Referenced entity kind.
    pub entity_type: String,
    /// UUID of the referenced entity.
    pub entity_id: String,
    /// Confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// AI metadata block on a Document. Mirrors `document.ai`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentAi {
    /// Free-form summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Machine-generated suggested links.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_links: Vec<DocumentSuggestedLink>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A Document entity — a binary or textual artifact attached to any
/// other entity. See SPEC §5.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Original filename.
    pub filename: String,
    /// MIME type (`image/jpeg`, `application/pdf`, …).
    pub mime_type: String,
    /// One of SPEC §5.5.2 document-type values.
    pub document_type: String,
    /// Presence status: `present | referenced | known_missing | lost | unknown`.
    pub status: String,
    /// File-storage metadata (for `status = present`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<DocumentFile>,
    /// External URL (for `status = referenced`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Date the document represents (event date, photo date, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Referenced [`crate::model::place::Place`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// BCP 47 language of the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Entities this document is linked to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_to: Vec<DocumentLinkedTo>,
    /// OCR block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr: Option<DocumentOcr>,
    /// AI block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<DocumentAi>,
    /// Human-readable caption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
