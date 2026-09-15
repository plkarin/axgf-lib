// SPDX-License-Identifier: Apache-2.0
//! # axgf-rs — Reference implementation of the Axiom Genealogy Format (AXGF) 1.0 and 1.1
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
//!    read only by [`import_bundle`] / [`import_bundle_streaming`] and written
//!    only by [`export_bundle`] / [`export_bundle_streaming`].
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
//! - [`model`] — Typed structs for the 8 entity kinds and the manifest, and
//!   in [`model::profile`] the AXGF 1.1 person profile: its claim shape, its
//!   closed vocabularies and the registry of every attribute. Never crosses
//!   the boundary.
//! - [`logic`] — Pure value-core: validation, CRUD, deduplication. Operates on
//!   [`model`] types, never on raw JSON.
//! - [`boundary`] — The only layer that speaks JSON, ZIP and bytes: envelope
//!   type, [`boundary::flat::FlatBundle`], and lifecycle helpers.
//! - [`convert`] — Foreign-format converters (GEDCOM 5.5.1 → AXGF).
//! - [`adapters`] — Thin per-target wrappers (rust, wasm, cffi, mobile) behind
//!   feature flags.
//!
//! ## Minimal example
//!
//! Create an empty bundle, add a person, and validate the result. Every
//! function takes and returns JSON, wrapped in a uniform
//! [`boundary::envelope::Envelope`].
//!
//! ```
//! use axgf_rs::{add_entity, create_bundle, validate, EntityKind};
//! use axgf_rs::boundary::envelope::Status;
//!
//! // 1. Create an empty bundle. `data` is a serde_json::Value; convert to a
//! //    string for the next call.
//! let bundle = create_bundle(Some("Karin")).data.to_string();
//!
//! // 2. Add a minimal person. The library generates a UUID v4 if none given
//! //    and fills in `type` and `axgf_version`. The envelope's `data` here
//! //    is `{ "id": <uuid>, "bundle": <updated flat bundle> }`.
//! let person = r#"{
//!     "identity": {
//!         "name":   {"display": "Jean Pierre-Léonard", "components": []},
//!         "gender": {"value": "M"},
//!         "is_living": true
//!     }
//! }"#;
//! let added = add_entity(&bundle, EntityKind::Person, person);
//! assert_eq!(added.status, Status::Ok);
//!
//! // 3. Structural + semantic validation over the updated bundle. Warnings
//! //    are non-blocking, so `status == Ok` even if diagnostics are present.
//! let updated_bundle = added.data["bundle"].to_string();
//! let checked = validate(&updated_bundle);
//! assert_eq!(checked.status, Status::Ok);
//! ```
//!
//! ## AXGF 1.1: the person profile
//!
//! AXGF 1.1 adds fourteen groups of person attributes — morphology, health,
//! genomics, military service, personality and the rest. Each is a claim: a
//! value with a date, a source and a confidence, or a series of them for
//! anything that changes in a lifetime. They are data, not behaviour: no
//! function changes, and a 1.1 attribute passes through [`add_entity`],
//! [`validate`] and the exports like any other field. [`model::profile`]
//! holds the specification's tables — every attribute with its value shape
//! and sensitive class, every closed vocabulary — for a client to build
//! forms and filters from, instead of keeping lists of its own.
//!
//! A bundle becomes 1.1 when it gets 1.1 content. [`create_bundle`] stamps
//! `"1.0"`, and the first 1.1 attribute raises the manifest in the same call.
//! A value outside its vocabulary is saved, and reported:
//!
//! ```
//! use axgf_rs::model::profile::{registry, vocab, SensitiveClass};
//! use axgf_rs::{add_entity, create_bundle, EntityKind};
//!
//! let bundle = create_bundle(None).data.to_string();
//! let person = r#"{
//!     "identity": {"name": {"display": "Zofia", "components": []},
//!                  "gender": {"value": "F"}, "is_living": false},
//!     "health": {"blood_group": {"value": "A", "confidence": 0.95},
//!                "rhesus": {"value": "sideways"}}
//! }"#;
//! let added = add_entity(&bundle, EntityKind::Person, person);
//! assert_eq!(added.data["bundle"]["manifest"]["axgf"], "1.1");
//! assert_eq!(added.diagnostics.len(), 1);
//! assert_eq!(added.diagnostics[0].code.as_str(), "OUT_OF_VOCABULARY");
//!
//! // What "rhesus" may hold, and that it is health data.
//! let rhesus = registry::attribute("health.rhesus").unwrap();
//! assert_eq!(rhesus.class, Some(SensitiveClass::Health));
//! assert!(vocab::RHESUS.contains("negative"));
//! ```
//!
//! ## Bundles too big for memory
//!
//! [`import_bundle`] decodes every binary payload into base64 inside the flat
//! JSON, and [`export_bundle`] needs them all present to write. Base64
//! inflates by a third, and encoded and decoded forms coexist while the map
//! is built, so a bundle carrying 417 MB of media costs upward of 1.5 GB to
//! open and more than that to save. For an archive of scanned certificates
//! that is fine; for one full of photographs it is not.
//!
//! [`import_bundle_textual`], [`import_bundle_streaming`] and
//! [`export_bundle_streaming`] are the payload-at-a-time alternative. The
//! textual half — manifest, entities, document *metadata* — is bounded by the
//! size of the tree and is still returned whole; payloads are handed over one
//! at a time, as a reader on the way in and a writer on the way out, and the
//! library forgets each before opening the next. Peak memory is bounded by
//! the largest single payload, or by a 64 KiB copy buffer if the caller never
//! holds one whole — never by their sum.
//!
//! This is an addition, not a replacement: streaming is never the default,
//! and callers who want the whole bundle as one JSON value get exactly what
//! they always did.
//!
//! A streaming import records what it streamed in
//! [`boundary::flat::FlatBundle::external_payloads`], and [`export_bundle`]
//! refuses any bundle carrying that marker. Otherwise a caller could stream
//! the media out, call the familiar export, and get a structurally valid
//! archive with every photograph silently missing. See [`docs/API.md`] for
//! worked demos.
//!
//! ## Command-line binary
//!
//! The same core is shipped as a standalone `axgf` executable. The `cli`
//! Cargo feature is on by default so `cargo install axgf-rs` produces
//! the binary; each subcommand prints a concise human summary by default
//! and the raw [`boundary::envelope::Envelope`] under `--json`. See
//! [`docs/CLI.md`] for the full reference. Library-only consumers can
//! opt out with `default-features = false, features = ["gedcom"]`.
//!
//! ## Further reading
//!
//! - [`docs/API.md`] — a longer walk-through of every public function.
//! - [`docs/CLI.md`] — the `axgf` binary: subcommands, flags, scripting.
//! - [`SETUP.md`] — build instructions and per-target adapter notes.
//! - [AXGF specification] — the format itself.
//!
//! [`docs/API.md`]: https://github.com/plkarin/axgf-lib/blob/main/docs/API.md
//! [`docs/CLI.md`]: https://github.com/plkarin/axgf-lib/blob/main/docs/CLI.md
//! [`SETUP.md`]: https://github.com/plkarin/axgf-lib/blob/main/SETUP.md

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod adapters;
pub mod boundary;
pub mod convert;
pub mod logic;
pub mod model;

/// AXGF specification versions this build understands, oldest first. Every
/// lifecycle operation verifies `manifest.axgf` against this set and refuses
/// to proceed on an unrecognized value with a stable
/// `UNSUPPORTED_SPEC_VERSION` diagnostic.
pub const SUPPORTED_SPEC_VERSIONS: &[&str] = &["1.0", "1.1"];

/// The AXGF specification version [`create_bundle`] stamps on a new bundle.
///
/// Still `"1.0"`, and deliberately: a bundle with no 1.1 content is a 1.0
/// bundle, and stamping it 1.1 would make every 1.0 reader refuse a file it
/// could read. A bundle is raised to [`LATEST_SPEC_VERSION`] by the first
/// write that gives it 1.1 content (SPEC_1.1 §2.1), in the same step that
/// refreshes its stats.
pub const CURRENT_SPEC_VERSION: &str = "1.0";

/// The newest AXGF specification version this build understands: `"1.1"`,
/// the extended person profile (draft).
pub const LATEST_SPEC_VERSION: &str = "1.1";

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
///
/// Refuses with `PAYLOADS_EXTERNAL` when the bundle declares
/// [`boundary::flat::FlatBundle::external_payloads`], because those bytes are
/// not present to write; use [`export_bundle_streaming`] for such a bundle.
pub fn export_bundle(flat_json: &str) -> Envelope {
    boundary::lifecycle::export_bundle(flat_json)
}

/// Read the textual half of a `.axgf` archive without decoding any payload.
///
/// Returns the manifest and all eight entity collections — document
/// *metadata* included — while every binary payload is left in the archive
/// and merely recorded, by path, size and CRC-32, in
/// [`boundary::flat::FlatBundle::external_payloads`]. Peak memory is
/// proportional to the textual data, not to the media.
///
/// `src` is any seekable byte source: a `File`, a `Cursor<&[u8]>`, anything
/// implementing [`std::io::Read`] + [`std::io::Seek`]. The library does not
/// open it and does not touch the filesystem.
///
/// Use [`import_bundle_streaming`] to take the payloads as well.
pub fn import_bundle_textual<R: std::io::Read + std::io::Seek>(src: R) -> Envelope {
    boundary::stream::import_bundle_textual(src)
}

/// Import a `.axgf` archive, handing each binary payload to `on_payload` one
/// at a time instead of collecting them all into the flat JSON.
///
/// The textual half is returned exactly as [`import_bundle_textual`] returns
/// it. For every payload the archive holds, `on_payload` is called once with
/// a [`boundary::stream::Payload`] — a live reader over that entry — before
/// the next one is opened. Nothing is retained between calls, so peak memory
/// is bounded by the largest single payload the caller chooses to buffer, or
/// by a fixed copy buffer if they use
/// [`boundary::stream::Payload::copy_to`].
///
/// An error returned by `on_payload` aborts the import with a
/// `PAYLOAD_SINK_FAILED` diagnostic.
///
/// ```no_run
/// use axgf_rs::import_bundle_streaming;
///
/// let file = std::fs::File::open("family.axgf")?;   // caller's I/O
/// let env = import_bundle_streaming(file, |payload| {
///     // Straight to disk; the bytes never exist whole in memory.
///     let mut out = std::fs::File::create(format!("cache/{}", payload.crc32()))?;
///     payload.copy_to(&mut out)?;
///     Ok(())
/// });
/// let flat = env.data.to_string();   // textual only: no base64 anywhere
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn import_bundle_streaming<R, F>(src: R, on_payload: F) -> Envelope
where
    R: std::io::Read + std::io::Seek,
    F: FnMut(&mut boundary::stream::Payload<'_>) -> std::io::Result<()>,
{
    boundary::stream::import_bundle_streaming(src, on_payload)
}

/// Write a `.axgf` archive to `dest`, asking `supply` for each payload in
/// turn rather than requiring them all up front.
///
/// The counterpart to [`import_bundle_streaming`]. For every path the bundle
/// declares in [`boundary::flat::FlatBundle::external_payloads`], `supply` is
/// called once with a [`boundary::stream::PayloadSlot`] — a writer feeding
/// straight into the open ZIP entry — and the library moves on to the next
/// without retaining the bytes. Payloads still carried inline in
/// `attachments` are written too, so a mixed bundle exports correctly.
///
/// Unlike [`export_bundle`], the archive is written to `dest` rather than
/// returned as base64, so the finished bundle never exists in memory either.
/// The envelope's `data` is
/// `{"size_bytes": u, "payloads_written": u, "payload_bytes": u}`.
///
/// A `supply` that fails, or that writes nothing for a payload the bundle
/// declares, aborts the export with `PAYLOAD_SOURCE_FAILED` rather than
/// producing an archive with media missing.
pub fn export_bundle_streaming<W, F>(flat_json: &str, dest: W, supply: F) -> Envelope
where
    W: std::io::Write + std::io::Seek,
    F: FnMut(&mut boundary::stream::PayloadSlot<'_>) -> std::io::Result<()>,
{
    boundary::stream::export_bundle_streaming(flat_json, dest, supply)
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
