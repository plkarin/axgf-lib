// SPDX-License-Identifier: Apache-2.0
//! # axgf-rs — Reference implementation of the Axiom Genealogy Format (AXGF) 1.0
//!
//! This crate is the canonical Rust implementation of the [AXGF specification].
//! It provides a **stateless, data-oriented boundary**: every public function
//! takes JSON strings or bytes and returns a single uniform [`boundary::envelope::Envelope`]
//! serialized to JSON. No native Rust types cross the boundary — this is what
//! makes language bindings mechanical.
//!
//! ## Design contract (V1)
//!
//! 1. **Stateless & immutable.** Every operation takes a bundle in, returns a
//!    new bundle out. No sessions, handles, or hidden mutation.
//! 2. **Flat JSON is the working form.** The on-disk `.axgf` is a ZIP, but the
//!    library converts it to a single flat JSON object for all editing. ZIP is
//!    read only by [`import_bundle`] and written only by [`export_bundle`].
//! 3. **No disk, no graph traversal, no query engine, no rendering in V1.**
//!    The caller passes bytes; the library never touches the filesystem.
//! 4. **Explicit spec-version gating.** Every operation checks `manifest.axgf`
//!    against [`SUPPORTED_SPEC_VERSIONS`] and refuses unknown versions.
//! 5. **Uniform envelope with stable diagnostic codes.** Validation is
//!    non-blocking: operations may succeed with warnings.
//! 6. **Forward compatibility.** Unknown fields survive a round-trip untouched.
//!
//! [AXGF specification]: https://github.com/plkarin/axgf-spec
//!
//! ## Module layout
//!
//! - [`model`] — Typed structs for the 8 entity kinds and the manifest.
//!   Internal to the library; never crosses the boundary.
//! - [`logic`] — Pure value-core: validation, CRUD, deduplication. Operates on
//!   [`model`] types, never on raw JSON.
//! - [`boundary`] — The only layer that speaks JSON, ZIP and bytes: envelope
//!   type, [`boundary::flat::FlatBundle`], and lifecycle helpers.
//! - [`convert`] — Foreign-format converters (GEDCOM 5.5.1 → AXGF).
//! - [`adapters`] — Thin per-target wrappers (rust, wasm, cffi, mobile) behind
//!   feature flags.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod adapters;
pub mod boundary;
pub mod convert;
pub mod logic;
pub mod model;

/// AXGF specification versions this build understands. Every lifecycle
/// operation verifies `manifest.axgf` against this set and refuses to proceed
/// on an unrecognized value with a stable `UNSUPPORTED_SPEC_VERSION`
/// diagnostic.
pub const SUPPORTED_SPEC_VERSIONS: &[&str] = &["1.0"];

/// The AXGF specification version this build writes when creating or
/// re-exporting bundles.
pub const CURRENT_SPEC_VERSION: &str = "1.0";

// -------------------------------------------------------------------------
// Public API surface
//
// Every function on the boundary takes and returns JSON (as `&str` or bytes)
// and yields an `Envelope` serialized to a JSON string. See individual layer
// modules for the underlying implementations.
// -------------------------------------------------------------------------

use boundary::envelope::Envelope;
pub use logic::crud::{DeletePolicy, EntityKind};

/// Create a new, empty AXGF bundle as flat JSON.
///
/// The optional `family_name` populates `manifest.family.name` when provided.
/// The returned envelope's `data` is the flat-bundle JSON.
pub fn create_bundle(family_name: Option<&str>) -> Envelope {
    boundary::lifecycle::create_bundle(family_name)
}

/// Import a `.axgf` ZIP archive (bytes) and return its flat-bundle JSON.
///
/// The manifest's `axgf` version is checked against [`SUPPORTED_SPEC_VERSIONS`]
/// and the operation fails with `UNSUPPORTED_SPEC_VERSION` on mismatch.
pub fn import_bundle(zip_bytes: &[u8]) -> Envelope {
    boundary::lifecycle::import_bundle(zip_bytes)
}

/// Export a flat-bundle JSON string to a `.axgf` ZIP archive.
///
/// Stats are recomputed and the canonical JSON Schema is embedded. The
/// returned envelope's `data` carries the ZIP bytes as base64 in a
/// `{"zip_base64": ...}` object.
pub fn export_bundle(flat_json: &str) -> Envelope {
    boundary::lifecycle::export_bundle(flat_json)
}

/// Return manifest and computed stats for the given flat bundle without
/// modifying it.
pub fn inspect(flat_json: &str) -> Envelope {
    boundary::lifecycle::inspect(flat_json)
}

/// Validate a flat bundle structurally (JSON Schema) and semantically
/// (dangling refs, cycles, chronology, duplicate unique refs). Warnings do
/// **not** cause a non-`ok` status.
pub fn validate(flat_json: &str) -> Envelope {
    logic::validate::validate(flat_json)
}

/// Add a new entity of the given kind to a flat bundle. A UUID v4 is
/// generated when `entity_json.id` is missing.
pub fn add_entity(flat_json: &str, kind: EntityKind, entity_json: &str) -> Envelope {
    logic::crud::add_entity(flat_json, kind, entity_json)
}

/// Update an existing entity in a flat bundle, keyed by `id`.
pub fn update_entity(flat_json: &str, kind: EntityKind, entity_json: &str) -> Envelope {
    logic::crud::update_entity(flat_json, kind, entity_json)
}

/// Delete an entity by id, applying the caller's referential-integrity
/// [`DeletePolicy`].
pub fn delete_entity(
    flat_json: &str,
    kind: EntityKind,
    id: &str,
    policy: DeletePolicy,
) -> Envelope {
    logic::crud::delete_entity(flat_json, kind, id, policy)
}

/// Run the safe deduplication passes on a flat bundle. Ambiguous merges are
/// flagged with `MANUAL_REVIEW_REQUIRED` diagnostics rather than performed.
pub fn deduplicate(flat_json: &str) -> Envelope {
    logic::dedup::deduplicate(flat_json)
}

/// Convert a GEDCOM 5.5.1 byte stream to a flat AXGF bundle.
///
/// - `default_confidence` is applied to imported facts when the source implies
///   no explicit confidence.
/// - `place_lang` is the BCP 47 language tag stored on imported `Place` names
///   when the GEDCOM record has no explicit language.
#[cfg(feature = "gedcom")]
pub fn convert_gedcom(gedcom_bytes: &[u8], default_confidence: f64, place_lang: &str) -> Envelope {
    convert::gedcom::convert(gedcom_bytes, default_confidence, place_lang)
}
