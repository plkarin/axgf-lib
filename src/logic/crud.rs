// SPDX-License-Identifier: Apache-2.0
//! # crud — add / update / delete with referential integrity
//!
//! The client picks the [`DeletePolicy`]; the library guarantees the
//! resulting bundle stays internally consistent. Add generates a UUID v4
//! when the caller omits `id`. Update refuses to touch a missing entity
//! and returns `ENTITY_NOT_FOUND`.
//!
//! Filled in during Phase 5.

use crate::boundary::envelope::{DiagnosticCode, Envelope};

/// One of the eight AXGF entity kinds. String forms match the on-disk
/// bundle directory names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// `persons/`
    Person,
    /// `families/`
    Family,
    /// `events/`
    Event,
    /// `links/`
    Link,
    /// `occupations/`
    Occupation,
    /// `sources/`
    Source,
    /// `places/`
    Place,
    /// `documents/`
    Document,
}

impl EntityKind {
    /// Return the plural collection name (matches the flat-bundle JSON key
    /// and the on-disk `.axgf` directory name).
    pub fn collection(self) -> &'static str {
        match self {
            EntityKind::Person     => "persons",
            EntityKind::Family     => "families",
            EntityKind::Event      => "events",
            EntityKind::Link       => "links",
            EntityKind::Occupation => "occupations",
            EntityKind::Source     => "sources",
            EntityKind::Place      => "places",
            EntityKind::Document   => "documents",
        }
    }
}

/// How [`delete_entity`] handles other entities that reference the one
/// being removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletePolicy {
    /// Refuse the delete if any references remain; the bundle is returned
    /// unchanged with a `DELETE_BLOCKED_BY_REFERENCE` diagnostic.
    Reject,
    /// Remove the entity and every field that referenced it (including
    /// removing referring family-child entries, participant entries, etc.).
    Cascade,
    /// Remove the entity but leave the reference fields present with
    /// `null` values, so consumers see that a link once existed but is now
    /// unresolved.
    Orphan,
}

/// See [`crate::add_entity`].
pub fn add_entity(_flat_json: &str, _kind: EntityKind, _entity_json: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "add_entity is not implemented yet (phase 5)",
    )
}

/// See [`crate::update_entity`].
pub fn update_entity(_flat_json: &str, _kind: EntityKind, _entity_json: &str) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "update_entity is not implemented yet (phase 5)",
    )
}

/// See [`crate::delete_entity`].
pub fn delete_entity(
    _flat_json: &str,
    _kind: EntityKind,
    _id: &str,
    _policy: DeletePolicy,
) -> Envelope {
    Envelope::error(
        DiagnosticCode::Internal,
        "delete_entity is not implemented yet (phase 5)",
    )
}
