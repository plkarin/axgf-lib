// SPDX-License-Identifier: Apache-2.0
//! # gedcom — GEDCOM 5.5.1 → flat AXGF bundle
//!
//! Feature-gated behind `gedcom` (default-on). Port of the ax-genealogy
//! converter with all its hard-won behaviors:
//!
//! - **Encoding auto-detect**: UTF-8 BOM, UTF-16, UTF-8, latin-1 fallback.
//! - **Localized date qualifiers and month names** in English, Polish,
//!   French and German. Unparseable date values are preserved as notes
//!   rather than dropped.
//! - **Partial dates** and BEF / AFT / BET ranges.
//! - **webtrees OBJE nesting** (FORM and TITL under FILE in 5.5.1
//!   exports); real MIME-type mapping; `status = present` only if the
//!   referenced file exists next to the input.
//! - **Per-file xref namespaces** mapped to distinct UUIDs so a
//!   multi-file merge cannot collide.
//!
//! Filled in during Phase 7.

use crate::boundary::envelope::{DiagnosticCode, Envelope};

/// See [`crate::convert_gedcom`].
pub fn convert(_gedcom_bytes: &[u8], _default_confidence: f64, _place_lang: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "convert_gedcom is not implemented yet (phase 7)",
    )
}
