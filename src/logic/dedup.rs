// SPDX-License-Identifier: Apache-2.0
//! # dedup — safe deduplication of persons and families
//!
//! Merge passes performed:
//!
//! - **Identical-spouse families** — two families whose unions reference
//!   the same set of spouse persons are merged; children are unioned and
//!   the richest surviving record is kept.
//! - **Duplicated couples** — two persons that clearly denote the same
//!   individual (identical normalized name + compatible dates + same
//!   family membership) are merged.
//!
//! **Never merged automatically:**
//!
//! - Father / son homonyms (same name across generations).
//! - Same-name cousins.
//! - Anything where dates or family structure make the identity
//!   ambiguous — these emit `MANUAL_REVIEW_REQUIRED` diagnostics so a
//!   human decides.
//!
//! Filled in during Phase 6.

use crate::boundary::envelope::{DiagnosticCode, Envelope};

/// See [`crate::deduplicate`].
pub fn deduplicate(_flat_json: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "deduplicate is not implemented yet (phase 6)",
    )
}
