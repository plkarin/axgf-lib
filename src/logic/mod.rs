// SPDX-License-Identifier: Apache-2.0
//! # logic — the value core (validation, CRUD, dedup)
//!
//! Pure, side-effect-free operations expressed on strongly-typed
//! [`crate::model`] structs. None of these functions touch the filesystem or
//! any language-binding shim; they take flat-bundle JSON strings on the
//! boundary and delegate to typed logic underneath.
//!
//! Sub-modules:
//!
//! - [`validate`] — structural (JSON Schema) and semantic checks (dangling
//!   references, cycles, chronology conflicts, duplicate unique refs).
//! - [`crud`] — add / update / delete for every entity kind, plus the
//!   caller-selected referential-integrity [`crud::DeletePolicy`].
//! - [`dedup`] — safe merging of duplicated persons and families, flagging
//!   ambiguous cases with `MANUAL_REVIEW_REQUIRED` diagnostics rather than
//!   performing them.

pub mod crud;
pub mod dedup;
pub mod validate;
