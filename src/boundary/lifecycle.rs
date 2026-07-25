// SPDX-License-Identifier: Apache-2.0
//! # lifecycle — create / import / export / inspect
//!
//! The four operations that translate between a caller's bytes and the
//! working-form flat bundle. The ZIP layout is defined by
//! [SPEC_1.0.md §2](https://github.com/plkarin/axgf-spec/blob/main/SPEC_1.0.md#2-bundle-structure).
//!
//! Every entry point here checks `manifest.axgf` against
//! [`crate::SUPPORTED_SPEC_VERSIONS`] and refuses to proceed on an unknown
//! version with a stable `UNSUPPORTED_SPEC_VERSION` diagnostic.
//!
//! Filled in during Phase 3.

use crate::boundary::envelope::{DiagnosticCode, Envelope};

/// See [`crate::create_bundle`].
pub fn create_bundle(_family_name: Option<&str>) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "create_bundle is not implemented yet (phase 3)",
    )
}

/// See [`crate::import_bundle`].
pub fn import_bundle(_zip_bytes: &[u8]) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "import_bundle is not implemented yet (phase 3)",
    )
}

/// See [`crate::export_bundle`].
pub fn export_bundle(_flat_json: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "export_bundle is not implemented yet (phase 3)",
    )
}

/// See [`crate::inspect`].
pub fn inspect(_flat_json: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "inspect is not implemented yet (phase 3)",
    )
}
