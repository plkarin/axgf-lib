// SPDX-License-Identifier: Apache-2.0
//! # crud — add / update / delete with referential integrity
//!
//! Every operation takes a flat bundle JSON string, produces a new flat
//! bundle (returned in the envelope's `data`), and refreshes the
//! manifest's `stats` and `updated_at` fields to match the mutation.
//! The library never mutates the caller's input.
//!
//! **`add_entity`** generates a UUID v4 when the caller omits `id`,
//! fills in `type` and `axgf_version` if missing, then structurally
//! validates the entity against the embedded schema; any schema
//! findings surface as non-blocking `SCHEMA_VALIDATION_FAILED`
//! warnings, but the add still succeeds. Refuses with
//! `ENTITY_ALREADY_EXISTS` if the target collection already has that
//! id.
//!
//! **`update_entity`** requires an `id` on the incoming entity and
//! rejects the update with `ENTITY_NOT_FOUND` if it is not already in
//! the target collection. The stored value is replaced verbatim, so
//! callers should send the *full* entity (not a patch).
//!
//! **`delete_entity`** is where the library earns its keep. The client
//! picks the [`DeletePolicy`] and the library guarantees the resulting
//! bundle has no dangling references produced by this delete:
//!
//! - [`DeletePolicy::Reject`] scans every other entity in the bundle
//!   for a reference to the target id; if any is found, the bundle is
//!   returned **unchanged** with a single `DELETE_BLOCKED_BY_REFERENCE`
//!   diagnostic listing the referrers.
//! - [`DeletePolicy::Cascade`] removes the entity and physically
//!   removes references: array items that hold a matching `_id` are
//!   dropped from their arrays; scalar `_id` fields matching the
//!   target are removed from their containing object. Referring
//!   entities that become schema-invalid as a result (for example, an
//!   `occupation` whose `person_id` was removed) are left in place —
//!   [`crate::validate`] will flag them if the caller runs it
//!   afterwards. V1 does *not* recursively delete referring entities;
//!   chain the deletes if you want deeper cleanup.
//! - [`DeletePolicy::Orphan`] removes the entity but keeps the shape
//!   of every referring container: matching scalar `_id` fields are
//!   set to `null` (key preserved), and array items are kept with the
//!   `_id` inside them nulled. Consumers can see "a link once existed
//!   here."

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::FlatBundle;
use crate::boundary::lifecycle::{
    check_manifest_version, compute_stats, now_iso8601_utc, parse_flat, EMBEDDED_SCHEMA,
};

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
    /// The plural collection name (matches the flat-bundle JSON key
    /// and the on-disk `.axgf` directory name).
    pub fn collection(self) -> &'static str {
        match self {
            EntityKind::Person => "persons",
            EntityKind::Family => "families",
            EntityKind::Event => "events",
            EntityKind::Link => "links",
            EntityKind::Occupation => "occupations",
            EntityKind::Source => "sources",
            EntityKind::Place => "places",
            EntityKind::Document => "documents",
        }
    }

    /// The singular schema-defs name (matches `$defs/{singular}` in
    /// the embedded JSON Schema).
    pub fn singular(self) -> &'static str {
        match self {
            EntityKind::Person => "person",
            EntityKind::Family => "family",
            EntityKind::Event => "event",
            EntityKind::Link => "link",
            EntityKind::Occupation => "occupation",
            EntityKind::Source => "source",
            EntityKind::Place => "place",
            EntityKind::Document => "document",
        }
    }
}

/// How [`delete_entity`] handles other entities that reference the one
/// being removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletePolicy {
    /// Refuse the delete if any references remain; the bundle is
    /// returned unchanged with a `DELETE_BLOCKED_BY_REFERENCE`
    /// diagnostic listing the referring entities.
    Reject,
    /// Remove the entity and physically remove references (array
    /// items dropped, scalar `_id` fields removed from their objects).
    Cascade,
    /// Remove the entity but keep the shape of every referring
    /// container: scalar `_id` fields are set to `null`, array-item
    /// `_id` fields are nulled with the rest of the item preserved.
    Orphan,
}

// -------------------------------------------------------------------------
// add
// -------------------------------------------------------------------------

/// See [`crate::add_entity`].
pub fn add_entity(flat_json: &str, kind: EntityKind, entity_json: &str) -> Envelope {
    let mut bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }
    let mut entity: Value = match serde_json::from_str(entity_json) {
        Ok(v) => v,
        Err(e) => {
            return Envelope::error(
                DiagnosticCode::InvalidJson,
                format!("cannot parse entity JSON: {e}"),
            );
        }
    };
    let Some(obj) = entity.as_object_mut() else {
        return Envelope::error(
            DiagnosticCode::InvalidBundleStructure,
            "entity JSON must be a JSON object",
        );
    };

    // Fill in mandatory identity fields if the caller omitted them.
    if !obj.contains_key("id") {
        obj.insert("id".into(), Value::String(Uuid::new_v4().to_string()));
    }
    if !obj.contains_key("type") {
        obj.insert("type".into(), Value::String(kind.singular().into()));
    }
    if !obj.contains_key("axgf_version") {
        obj.insert("axgf_version".into(), Value::String("1.0".into()));
    }

    let id = match obj.get("id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return Envelope::error(
                DiagnosticCode::InvalidBundleStructure,
                "entity.id is not a string",
            );
        }
    };

    let map = collection_mut(&mut bundle, kind);
    if map.contains_key(&id) {
        return Envelope::error(
            DiagnosticCode::EntityAlreadyExists,
            format!("{} already contains id {id}", kind.collection()),
        );
    }
    let diags = validate_entity_in_isolation(kind, &entity, &id);
    map.insert(id.clone(), entity);
    refresh_manifest(&mut bundle);

    let data = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok_with(json!({"id": id, "bundle": data}), diags)
}

// -------------------------------------------------------------------------
// update
// -------------------------------------------------------------------------

/// See [`crate::update_entity`].
pub fn update_entity(flat_json: &str, kind: EntityKind, entity_json: &str) -> Envelope {
    let mut bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }
    let entity: Value = match serde_json::from_str(entity_json) {
        Ok(v) => v,
        Err(e) => {
            return Envelope::error(
                DiagnosticCode::InvalidJson,
                format!("cannot parse entity JSON: {e}"),
            );
        }
    };
    let id = match entity.get("id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return Envelope::error(
                DiagnosticCode::InvalidBundleStructure,
                "update_entity requires entity.id to be present and a string",
            );
        }
    };
    let map = collection_mut(&mut bundle, kind);
    if !map.contains_key(&id) {
        return Envelope::error(
            DiagnosticCode::EntityNotFound,
            format!("{} does not contain id {id}", kind.collection()),
        );
    }
    let diags = validate_entity_in_isolation(kind, &entity, &id);
    map.insert(id.clone(), entity);
    refresh_manifest(&mut bundle);

    let data = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok_with(json!({"id": id, "bundle": data}), diags)
}

// -------------------------------------------------------------------------
// delete
// -------------------------------------------------------------------------

/// See [`crate::delete_entity`].
pub fn delete_entity(
    flat_json: &str,
    kind: EntityKind,
    id: &str,
    policy: DeletePolicy,
) -> Envelope {
    let mut bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }
    // Entity must exist in the target collection.
    {
        let map = collection_mut(&mut bundle, kind);
        if !map.contains_key(id) {
            return Envelope::error(
                DiagnosticCode::EntityNotFound,
                format!("{} does not contain id {id}", kind.collection()),
            );
        }
    }

    // For Reject, scan every other entity for references and abort
    // *before* touching the bundle.
    if matches!(policy, DeletePolicy::Reject) {
        let referrers = find_referrers(&bundle, id, kind);
        if !referrers.is_empty() {
            return Envelope::error_many(vec![Diagnostic {
                code: DiagnosticCode::DeleteBlockedByReference,
                severity: Severity::Error,
                message: format!(
                    "cannot delete {}/{id} under Reject: still referenced by {} entities: {:?}",
                    kind.collection(),
                    referrers.len(),
                    referrers
                ),
                entity_ref: Some(format!("{}/{id}", kind.collection())),
            }]);
        }
    }

    // Otherwise apply the scrub policy across every entity, then
    // remove the target from its own collection.
    if matches!(policy, DeletePolicy::Cascade | DeletePolicy::Orphan) {
        scrub_bundle(&mut bundle, id, policy);
    }
    let map = collection_mut(&mut bundle, kind);
    map.remove(id);
    refresh_manifest(&mut bundle);

    let data = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok_with(json!({"id": id, "bundle": data}), Vec::new())
}

// -------------------------------------------------------------------------
// Internals
// -------------------------------------------------------------------------

fn collection_mut(b: &mut FlatBundle, kind: EntityKind) -> &mut BTreeMap<String, Value> {
    match kind {
        EntityKind::Person => &mut b.persons,
        EntityKind::Family => &mut b.families,
        EntityKind::Event => &mut b.events,
        EntityKind::Link => &mut b.links,
        EntityKind::Occupation => &mut b.occupations,
        EntityKind::Source => &mut b.sources,
        EntityKind::Place => &mut b.places,
        EntityKind::Document => &mut b.documents,
    }
}

fn refresh_manifest(b: &mut FlatBundle) {
    let stats = compute_stats(b);
    let now = now_iso8601_utc();
    if let Value::Object(ref mut m) = b.manifest {
        m.insert("stats".into(), stats);
        m.insert("updated_at".into(), Value::String(now));
    }
}

/// Return referrers of `target` across the bundle as `"collection/id"`
/// strings, excluding `target` itself. Deduped and sorted (BTreeSet)
/// so diagnostics are deterministic.
fn find_referrers(b: &FlatBundle, target: &str, target_kind: EntityKind) -> Vec<String> {
    let mut hits: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (kind, coll, map) in entity_collections(b) {
        for (id, value) in map {
            if kind == target_kind && id == target {
                continue;
            }
            if entity_references_target(value, target) {
                hits.insert(format!("{coll}/{id}"));
            }
        }
    }
    hits.into_iter().collect()
}

fn entity_references_target(v: &Value, target: &str) -> bool {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                if k == "id" {
                    continue;
                }
                if k.ends_with("_id") && val.as_str() == Some(target) {
                    return true;
                }
                if entity_references_target(val, target) {
                    return true;
                }
            }
            false
        }
        Value::Array(a) => a.iter().any(|item| entity_references_target(item, target)),
        _ => false,
    }
}

/// Apply the `Cascade` or `Orphan` scrub across every entity in the
/// bundle. Idempotent: after a scrub, no reference to `target`
/// remains anywhere.
fn scrub_bundle(b: &mut FlatBundle, target: &str, policy: DeletePolicy) {
    for map in [
        &mut b.persons,
        &mut b.families,
        &mut b.events,
        &mut b.links,
        &mut b.occupations,
        &mut b.sources,
        &mut b.places,
        &mut b.documents,
    ] {
        for value in map.values_mut() {
            scrub_value(value, target, policy);
        }
    }
}

/// Recursive walker that removes or nulls references to `target`
/// inside `v`. See the module docs for the exact rule per policy.
fn scrub_value(v: &mut Value, target: &str, policy: DeletePolicy) {
    match v {
        Value::Object(m) => scrub_object(m, target, policy),
        Value::Array(a) => {
            if matches!(policy, DeletePolicy::Cascade) {
                a.retain(|item| !object_holds_ref(item, target));
            }
            for item in a.iter_mut() {
                scrub_value(item, target, policy);
            }
        }
        _ => {}
    }
}

fn scrub_object(m: &mut Map<String, Value>, target: &str, policy: DeletePolicy) {
    let matching_keys: Vec<String> = m
        .iter()
        .filter_map(|(k, v)| {
            if k == "id" || !k.ends_with("_id") {
                return None;
            }
            if v.as_str() == Some(target) {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect();
    for k in matching_keys {
        match policy {
            DeletePolicy::Cascade => {
                m.remove(&k);
            }
            DeletePolicy::Orphan => {
                m.insert(k, Value::Null);
            }
            DeletePolicy::Reject => {}
        }
    }
    for val in m.values_mut() {
        scrub_value(val, target, policy);
    }
}

/// True if `v` is a JSON object with at least one `*_id` field
/// (excluding the entity's own `id`) whose string value equals
/// `target`. Used by cascade to decide whether to drop an array item.
fn object_holds_ref(v: &Value, target: &str) -> bool {
    match v {
        Value::Object(m) => m
            .iter()
            .any(|(k, val)| k != "id" && k.ends_with("_id") && val.as_str() == Some(target)),
        _ => false,
    }
}

/// Validate `entity` against its schema-defs branch. Any failure
/// surfaces as a non-blocking warning; the caller may still succeed.
fn validate_entity_in_isolation(kind: EntityKind, entity: &Value, id: &str) -> Vec<Diagnostic> {
    use jsonschema::JSONSchema;

    let root: Value = match serde_json::from_str(EMBEDDED_SCHEMA) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let defs = root.get("$defs").cloned().unwrap_or(Value::Null);
    if defs.is_null() {
        return Vec::new();
    }
    let wrapper = json!({
        "$defs": defs,
        "$ref": format!("#/$defs/{}", kind.singular()),
    });
    let compiled = match JSONSchema::compile(&wrapper) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Err(errors) = compiled.validate(entity) {
        for e in errors {
            out.push(Diagnostic {
                code: DiagnosticCode::SchemaValidationFailed,
                severity: Severity::Warning,
                message: format!("{}: {e}", kind.singular()),
                entity_ref: Some(format!("{}/{id}", kind.collection())),
            });
        }
    }
    out
}

fn entity_collections(
    b: &FlatBundle,
) -> [(EntityKind, &'static str, &BTreeMap<String, Value>); 8] {
    [
        (EntityKind::Person, "persons", &b.persons),
        (EntityKind::Family, "families", &b.families),
        (EntityKind::Event, "events", &b.events),
        (EntityKind::Link, "links", &b.links),
        (EntityKind::Occupation, "occupations", &b.occupations),
        (EntityKind::Source, "sources", &b.sources),
        (EntityKind::Place, "places", &b.places),
        (EntityKind::Document, "documents", &b.documents),
    ]
}
