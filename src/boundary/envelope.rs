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

use serde::de::{self, Deserializer};
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

    /// Parse a wire-form string back into a [`DiagnosticCode`]. Returns
    /// `None` for unrecognized codes (this keeps forward-compatibility:
    /// consumers on an older library reading a newer library's envelope
    /// can inspect the raw string and treat unknown codes as generic
    /// errors without crashing).
    pub fn from_wire(s: &str) -> Option<Self> {
        use DiagnosticCode::*;
        Some(match s {
            "UNSUPPORTED_SPEC_VERSION"    => UnsupportedSpecVersion,
            "INVALID_JSON"                => InvalidJson,
            "INVALID_BUNDLE_STRUCTURE"    => InvalidBundleStructure,
            "SCHEMA_VALIDATION_FAILED"    => SchemaValidationFailed,
            "DANGLING_REFERENCE"          => DanglingReference,
            "DUPLICATE_ENTITY_ID"         => DuplicateEntityId,
            "DUPLICATE_UNIQUE_REF"        => DuplicateUniqueRef,
            "CYCLE_DETECTED"              => CycleDetected,
            "CHRONOLOGY_CONFLICT"         => ChronologyConflict,
            "ENTITY_NOT_FOUND"            => EntityNotFound,
            "ENTITY_ALREADY_EXISTS"       => EntityAlreadyExists,
            "UNKNOWN_ENTITY_KIND"         => UnknownEntityKind,
            "DELETE_BLOCKED_BY_REFERENCE" => DeleteBlockedByReference,
            "MANUAL_REVIEW_REQUIRED"      => ManualReviewRequired,
            "ZIP_READ_ERROR"              => ZipReadError,
            "ZIP_WRITE_ERROR"             => ZipWriteError,
            "GEDCOM_PARSE_ERROR"          => GedcomParseError,
            "GEDCOM_UNRECOGNIZED_TAG"     => GedcomUnrecognizedTag,
            "INTERNAL"                    => Internal,
            _ => return None,
        })
    }
}

impl Serialize for DiagnosticCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticCode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        DiagnosticCode::from_wire(&raw).ok_or_else(|| {
            de::Error::custom(format!("unknown diagnostic code: {raw}"))
        })
    }
}

/// A single diagnostic returned inside an [`Envelope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable machine-readable code.
    pub code: DiagnosticCode,
    /// Severity of the issue.
    pub severity: Severity,
    /// Human-readable message; wording may change between releases.
    pub message: String,
    /// Optional pointer to a specific entity in the bundle, formatted as
    /// `"{kind}/{uuid}"` (for example `"persons/550e8400-…"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_ref: Option<String>,
}

/// The uniform response returned by every public function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Overall status of the call.
    pub status: Status,
    /// Operation payload. Convention: `null` when `status` is `Error`.
    #[serde(default)]
    pub data: serde_json::Value,
    /// Ordered list of diagnostics; MAY be empty.
    #[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ok_envelope_has_ok_status_and_no_diagnostics() {
        let env = Envelope::ok(json!({"hello": "world"}));
        assert_eq!(env.status, Status::Ok);
        assert!(env.diagnostics.is_empty());
        assert_eq!(env.data, json!({"hello": "world"}));
    }

    #[test]
    fn error_envelope_has_null_data_and_one_diagnostic() {
        let env = Envelope::error(DiagnosticCode::InvalidJson, "not JSON");
        assert_eq!(env.status, Status::Error);
        assert!(env.data.is_null());
        assert_eq!(env.diagnostics.len(), 1);
        assert_eq!(env.diagnostics[0].code, DiagnosticCode::InvalidJson);
        assert_eq!(env.diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn diagnostic_code_wire_form_is_stable() {
        // The wire strings are a public contract; test that each code
        // maps to its documented SCREAMING_SNAKE_CASE value both ways.
        let cases = [
            (DiagnosticCode::UnsupportedSpecVersion, "UNSUPPORTED_SPEC_VERSION"),
            (DiagnosticCode::DanglingReference, "DANGLING_REFERENCE"),
            (DiagnosticCode::DeleteBlockedByReference, "DELETE_BLOCKED_BY_REFERENCE"),
            (DiagnosticCode::ManualReviewRequired, "MANUAL_REVIEW_REQUIRED"),
            (DiagnosticCode::ChronologyConflict, "CHRONOLOGY_CONFLICT"),
        ];
        for (code, wire) in cases {
            assert_eq!(code.as_str(), wire);
            assert_eq!(DiagnosticCode::from_wire(wire), Some(code));
        }
    }

    #[test]
    fn envelope_json_round_trip_preserves_all_fields() {
        let original = Envelope::ok_with(
            json!({"nested": [1, 2, {"a": "b"}]}),
            vec![
                Diagnostic {
                    code: DiagnosticCode::DanglingReference,
                    severity: Severity::Warning,
                    message: "person X not found".into(),
                    entity_ref: Some("families/abc".into()),
                },
                Diagnostic {
                    code: DiagnosticCode::ManualReviewRequired,
                    severity: Severity::Info,
                    message: "ambiguous merge".into(),
                    entity_ref: None,
                },
            ],
        );
        let wire = original.to_json();
        let parsed: Envelope = serde_json::from_str(&wire).expect("re-parse");

        assert_eq!(parsed.status, original.status);
        assert_eq!(parsed.data, original.data);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(parsed.diagnostics[0].code, DiagnosticCode::DanglingReference);
        assert_eq!(parsed.diagnostics[0].severity, Severity::Warning);
        assert_eq!(parsed.diagnostics[0].message, "person X not found");
        assert_eq!(parsed.diagnostics[0].entity_ref.as_deref(), Some("families/abc"));
        assert!(parsed.diagnostics[1].entity_ref.is_none());
    }

    #[test]
    fn omitted_entity_ref_is_absent_from_wire_form() {
        let env = Envelope::error(DiagnosticCode::InvalidJson, "boom");
        let wire = env.to_json();
        // entity_ref is skipped when None so it should not appear at all.
        assert!(!wire.contains("entity_ref"), "wire form had entity_ref: {wire}");
        // Status and code strings must be present verbatim.
        assert!(wire.contains("\"status\":\"error\""));
        assert!(wire.contains("\"code\":\"INVALID_JSON\""));
    }

    #[test]
    fn unknown_wire_code_deserializes_to_error() {
        let wire = r#"{"status":"error","data":null,"diagnostics":[
            {"code":"MADE_UP_CODE","severity":"error","message":"x"}]}"#;
        // A future library may emit codes this build does not know. We
        // reject at parse-time rather than silently mislabel — the caller
        // can fall back to inspecting the raw JSON if they want to be
        // forward-compatible.
        let err = serde_json::from_str::<Envelope>(wire).unwrap_err();
        assert!(err.to_string().contains("unknown diagnostic code"));
    }
}
