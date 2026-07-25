// SPDX-License-Identifier: Apache-2.0
//! # boundary — the JSON / ZIP / bytes edge of the library
//!
//! This layer is the **only** part of the crate that speaks in-boundary
//! formats. It owns:
//!
//! - [`envelope`] — the uniform [`envelope::Envelope`] returned by every
//!   public function (status, data, diagnostics).
//! - [`flat`] — the working-form flat-bundle representation, a JSON object
//!   containing the manifest and one map per entity kind.
//! - [`lifecycle`] — the ZIP-facing operations: `create_bundle`,
//!   `import_bundle`, `export_bundle`, `inspect`.
//!
//! Everything inside [`crate::logic`] operates on strongly-typed
//! [`crate::model`] structs; the boundary is the translation point between
//! those and the outside world.

pub mod envelope;
pub mod flat;
pub mod lifecycle;
