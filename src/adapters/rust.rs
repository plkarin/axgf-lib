// SPDX-License-Identifier: Apache-2.0
//! # rust adapter — direct re-export of the crate's public API
//!
//! No-op adapter for Rust callers; simply re-exports the top-level
//! functions and boundary types so downstream code can `use
//! axgf_rs::adapters::rust::*;` without pulling in adapter-specific
//! wrappers meant for other languages.

pub use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity, Status};
pub use crate::boundary::flat::ExternalPayload;
pub use crate::boundary::stream::{Payload, PayloadSlot};
pub use crate::logic::crud::{DeletePolicy, EntityKind};
pub use crate::{
    add_entity, create_bundle, deduplicate, delete_entity, export_bundle, export_bundle_streaming,
    import_bundle, import_bundle_streaming, import_bundle_textual, inspect, update_entity,
    validate, CURRENT_SPEC_VERSION, LATEST_SPEC_VERSION, SUPPORTED_SPEC_VERSIONS,
};

#[cfg(feature = "gedcom")]
pub use crate::convert_gedcom;
