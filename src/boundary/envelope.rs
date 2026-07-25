// SPDX-License-Identifier: Apache-2.0
//! # envelope — the uniform boundary response type
//!
//! Every public function in this crate returns an [`Envelope`] serialized to
//! JSON with the shape:
//!
//! ```json
//! {
//!   "status": "ok" | "error",
//!   "data":   <value | null>,
//!   "diagnostics": [
//!     { "code": "SCREAMING_SNAKE_CASE", "severity": "info|warning|error",
//!       "message": "human text", "entity_ref": "kind/uuid" }
//!   ]
//! }
//! ```
//!
//! # Rules
//!
//! - **Codes are a stable public contract.** They MUST NOT change spelling
//!   between versions. New codes may be added.
//! - **Messages are human text and MAY change** between releases.
//! - **Validation is non-blocking.** An `Envelope` with `status = "ok"` may
//!   still carry `warning` diagnostics.
//!
//! Filled in during Phase 1.

use serde::{Deserialize, Serialize};

/// Overall status of an operation. `Ok` may co-exist with `warning` or
/// `info` diagnostics; `Error` means the operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The operation completed. Warnings may still be present in `diagnostics`.
    Ok,
    /// The operation was refused. `data` is `null`.
    Error,
}

/// Severity of a single [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational — non-actionable context about the operation.
    Info,
    /// A recoverable issue the operation succeeded despite.
    Warning,
    /// A blocking condition; typically accompanied by `Status::Error`.
    Error,
}

/// Stable diagnostic codes. Strings are the public contract; enum variant
/// spelling is a Rust-side convenience.
#[allow(missing_docs)] // variant names are self-descriptive and doc-locked to code strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    UnsupportedSpecVersion,
    InvalidJson,
    InvalidBundleStructure,
    SchemaValidationFailed,
    DanglingReference,
    DuplicateEntityId,
    DuplicateUniqueRef,
    CycleDetected,
    ChronologyConflict,
    EntityNotFound,
    EntityAlreadyExists,
    UnknownEntityKind,
    DeleteBlockedByReference,
    ManualReviewRequired,
    ZipReadError,
    ZipWriteError,
    GedcomParseError,
    GedcomUnrecognizedTag,
    Internal,
}

impl DiagnosticCode {
    /// The canonical wire-form string for this code.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::UnsupportedSpecVersion   => "UNSUPPORTED_SPEC_VERSION",
            DiagnosticCode::InvalidJson              => "INVALID_JSON",
            DiagnosticCode::InvalidBundleStructure   => "INVALID_BUNDLE_STRUCTURE",
            DiagnosticCode::SchemaValidationFailed   => "SCHEMA_VALIDATION_FAILED",
            DiagnosticCode::DanglingReference        => "DANGLING_REFERENCE",
            DiagnosticCode::DuplicateEntityId        => "DUPLICATE_ENTITY_ID",
            DiagnosticCode::DuplicateUniqueRef       => "DUPLICATE_UNIQUE_REF",
            DiagnosticCode::CycleDetected            => "CYCLE_DETECTED",
            DiagnosticCode::ChronologyConflict       => "CHRONOLOGY_CONFLICT",
            DiagnosticCode::EntityNotFound           => "ENTITY_NOT_FOUND",
            DiagnosticCode::EntityAlreadyExists      => "ENTITY_ALREADY_EXISTS",
            DiagnosticCode::UnknownEntityKind        => "UNKNOWN_ENTITY_KIND",
            DiagnosticCode::DeleteBlockedByReference => "DELETE_BLOCKED_BY_REFERENCE",
            DiagnosticCode::ManualReviewRequired     => "MANUAL_REVIEW_REQUIRED",
            DiagnosticCode::ZipReadError             => "ZIP_READ_ERROR",
            DiagnosticCode::ZipWriteError            => "ZIP_WRITE_ERROR",
            DiagnosticCode::GedcomParseError         => "GEDCOM_PARSE_ERROR",
            DiagnosticCode::GedcomUnrecognizedTag    => "GEDCOM_UNRECOGNIZED_TAG",
            DiagnosticCode::Internal                 => "INTERNAL",
        }
    }
}

impl Serialize for DiagnosticCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

/// A single diagnostic returned inside an [`Envelope`].
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Stable machine-readable code.
    pub code: DiagnosticCode,
    /// Severity of the issue.
    pub severity: Severity,
    /// Human-readable message; wording may change between releases.
    pub message: String,
    /// Optional pointer to a specific entity in the bundle, formatted as
    /// `"{kind}/{uuid}"` (for example `"persons/550e8400-…"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_ref: Option<String>,
}

/// The uniform response returned by every public function.
#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    /// Overall status of the call.
    pub status: Status,
    /// Operation payload. Convention: `null` when `status` is `Error`.
    pub data: serde_json::Value,
    /// Ordered list of diagnostics; MAY be empty.
    pub diagnostics: Vec<Diagnostic>,
}

impl Envelope {
    /// Construct an `ok` envelope with the given payload and no diagnostics.
    pub fn ok(data: serde_json::Value) -> Self {
        Self { status: Status::Ok, data, diagnostics: Vec::new() }
    }

    /// Construct an `ok` envelope carrying the given diagnostics (typically
    /// warnings).
    pub fn ok_with(data: serde_json::Value, diagnostics: Vec<Diagnostic>) -> Self {
        Self { status: Status::Ok, data, diagnostics }
    }

    /// Construct an `error` envelope with a single diagnostic and `data =
    /// null`.
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            status: Status::Error,
            data: serde_json::Value::Null,
            diagnostics: vec![Diagnostic {
                code,
                severity: Severity::Error,
                message: message.into(),
                entity_ref: None,
            }],
        }
    }

    /// Construct an `error` envelope carrying multiple diagnostics.
    pub fn error_many(diagnostics: Vec<Diagnostic>) -> Self {
        Self { status: Status::Error, data: serde_json::Value::Null, diagnostics }
    }

    /// Serialize the envelope to a JSON string (never fails: all fields are
    /// serde-representable primitives).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            // Fallback if serde ever fails (should not happen for this shape).
            format!(
                "{{\"status\":\"error\",\"data\":null,\"diagnostics\":[{{\"code\":\"INTERNAL\",\"severity\":\"error\",\"message\":\"envelope serialization failed: {}\"}}]}}",
                e.to_string().replace('"', "'")
            )
        })
    }
}
