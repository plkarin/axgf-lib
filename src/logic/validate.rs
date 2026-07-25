// SPDX-License-Identifier: Apache-2.0
//! # validate — structural and semantic bundle validation
//!
//! Structural checks use the embedded JSON Schema
//! (`schema/axgf-1.0.schema.json` from the spec repo). Semantic checks are
//! implemented on typed [`crate::model`] structs and cover:
//!
//! - **Dangling references** — every `*_id` field points to an entity that
//!   exists in the bundle (or is null/absent).
//! - **Cycles** — parent/child relationships form a DAG.
//! - **Chronology** — a child MUST NOT be born before their parents, a
//!   spouse's marriage MUST NOT precede their birth, etc.
//! - **Duplicate unique refs** — two families with the same `union.persons`
//!   set, two persons with identical `id`, etc.
//!
//! Warnings are non-blocking: an [`crate::boundary::envelope::Envelope`]
//! MAY carry `Warning`-severity diagnostics with `Status::Ok`.
//!
//! Filled in during Phase 4.

use crate::boundary::envelope::{DiagnosticCode, Envelope};

/// See [`crate::validate`].
pub fn validate(_flat_json: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "validate is not implemented yet (phase 4)",
    )
}
