// SPDX-License-Identifier: Apache-2.0
//! # lifecycle — create / import / export / inspect
//!
//! The four operations that translate between a caller's bytes and the
//! working-form flat bundle. The ZIP layout is defined by
//! [SPEC_1.0.md §2](https://github.com/plkarin/axgf-spec/blob/main/SPEC_1.0.md#2-bundle-structure).
//!
//! Every entry point here (except [`create_bundle`]) checks
//! `manifest.axgf` against [`crate::SUPPORTED_SPEC_VERSIONS`] and refuses
//! to proceed on an unknown version with a stable
//! `UNSUPPORTED_SPEC_VERSION` diagnostic.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{json, Value};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::boundary::envelope::{DiagnosticCode, Envelope};
use crate::boundary::flat::FlatBundle;
use crate::{CURRENT_SPEC_VERSION, SUPPORTED_SPEC_VERSIONS};

/// The canonical AXGF 1.0 JSON Schema, embedded from
/// `schema/axgf-1.0.schema.json` in the crate root.
///
/// This is written verbatim into every exported bundle at
/// `schema/axgf-1.0.schema.json` per SPEC §2 and §12.1.
pub const EMBEDDED_SCHEMA: &str = include_str!("../../schema/axgf-1.0.schema.json");

/// The eight entity kinds and their on-disk directory names. Order
/// matches the manifest stats field order.
const ENTITY_DIRS: [(&str, &str); 8] = [
    ("persons",     "persons"),
    ("families",    "families"),
    ("events",      "events"),
    ("links",       "links"),
    ("occupations", "occupations"),
    ("sources",     "sources"),
    ("places",      "places"),
    // Documents are asymmetric: metadata is in documents/index.json,
    // binary payloads under documents/files/. We list the dir here for
    // stats, but read/write is handled specially.
    ("documents",   "documents"),
];

/// Format a `SystemTime`-equivalent as an ISO 8601 UTC string.
fn now_iso8601_utc() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

/// Verify a manifest's `axgf` field is a supported spec version. On
/// mismatch, returns an `UNSUPPORTED_SPEC_VERSION` envelope for the
/// caller to bubble up.
pub(crate) fn check_manifest_version(manifest: &Value) -> Result<(), Envelope> {
    let axgf = manifest.get("axgf").and_then(Value::as_str);
    match axgf {
        Some(v) if SUPPORTED_SPEC_VERSIONS.contains(&v) => Ok(()),
        Some(v) => Err(Envelope::error(
            DiagnosticCode::UnsupportedSpecVersion,
            format!(
                "unsupported AXGF spec version {v:?}; this build supports {SUPPORTED_SPEC_VERSIONS:?}"
            ),
        )),
        None => Err(Envelope::error(
            DiagnosticCode::InvalidBundleStructure,
            "manifest.axgf is missing or not a string",
        )),
    }
}

/// Parse a flat-bundle JSON string, or return an `INVALID_JSON` /
/// `INVALID_BUNDLE_STRUCTURE` envelope.
pub(crate) fn parse_flat(flat_json: &str) -> Result<FlatBundle, Envelope> {
    serde_json::from_str::<FlatBundle>(flat_json).map_err(|e| {
        Envelope::error(DiagnosticCode::InvalidJson, format!("cannot parse flat bundle: {e}"))
    })
}

/// Compute a fresh stats object from the entity maps in a `FlatBundle`.
fn compute_stats(b: &FlatBundle) -> Value {
    json!({
        "persons":     b.persons.len(),
        "families":    b.families.len(),
        "events":      b.events.len(),
        "links":       b.links.len(),
        "occupations": b.occupations.len(),
        "sources":     b.sources.len(),
        "places":      b.places.len(),
        "documents":   b.documents.len(),
    })
}

// -------------------------------------------------------------------------
// create_bundle
// -------------------------------------------------------------------------

/// See [`crate::create_bundle`].
pub fn create_bundle(family_name: Option<&str>) -> Envelope {
    let now = now_iso8601_utc();
    let mut manifest = json!({
        "axgf": CURRENT_SPEC_VERSION,
        "created_at": now,
        "updated_at": now,
        "stats": {
            "persons": 0, "families": 0, "events": 0, "links": 0,
            "occupations": 0, "sources": 0, "places": 0, "documents": 0
        }
    });
    if let Some(name) = family_name {
        manifest["family"] = json!({ "name": name });
    }
    let bundle = FlatBundle {
        manifest,
        ..Default::default()
    };
    let value = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok(value)
}

// -------------------------------------------------------------------------
// inspect
// -------------------------------------------------------------------------

/// See [`crate::inspect`].
pub fn inspect(flat_json: &str) -> Envelope {
    let bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }
    let stats = compute_stats(&bundle);
    Envelope::ok(json!({
        "manifest": bundle.manifest,
        "stats": stats,
    }))
}

// -------------------------------------------------------------------------
// import_bundle
// -------------------------------------------------------------------------

/// See [`crate::import_bundle`].
pub fn import_bundle(zip_bytes: &[u8]) -> Envelope {
    let reader = Cursor::new(zip_bytes);
    let mut archive = match ZipArchive::new(reader) {
        Ok(a) => a,
        Err(e) => {
            return Envelope::error(
                DiagnosticCode::ZipReadError,
                format!("cannot open ZIP: {e}"),
            );
        }
    };

    let mut manifest: Value = Value::Null;
    let mut persons     = BTreeMap::new();
    let mut families    = BTreeMap::new();
    let mut events      = BTreeMap::new();
    let mut links       = BTreeMap::new();
    let mut occupations = BTreeMap::new();
    let mut sources     = BTreeMap::new();
    let mut places      = BTreeMap::new();
    let mut documents   = BTreeMap::new();
    let mut attachments = BTreeMap::new();

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                return Envelope::error(
                    DiagnosticCode::ZipReadError,
                    format!("cannot read ZIP entry #{i}: {e}"),
                );
            }
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();

        // manifest.json
        if name == "manifest.json" {
            match read_json(&mut entry) {
                Ok(v) => manifest = v,
                Err(e) => return e,
            }
            continue;
        }

        // schema/** — dropped on import; a fresh embedded copy is written
        // on every export so bundles carry the canonical schema.
        if name.starts_with("schema/") {
            continue;
        }

        // Entity dirs: persons/{uuid}.json, families/{uuid}.json, …
        if let Some((collection, id)) = split_entity_path(&name) {
            let target = match collection {
                "persons"     => &mut persons,
                "families"    => &mut families,
                "events"      => &mut events,
                "links"       => &mut links,
                "occupations" => &mut occupations,
                "sources"     => &mut sources,
                "places"      => &mut places,
                _ => unreachable!(),
            };
            match read_json(&mut entry) {
                Ok(v) => {
                    target.insert(id.to_string(), v);
                    continue;
                }
                Err(e) => return e,
            }
        }

        // documents/index.json → flat.documents (map of uuid → meta)
        if name == "documents/index.json" {
            match read_json(&mut entry) {
                Ok(Value::Object(map)) => {
                    for (k, v) in map {
                        documents.insert(k, v);
                    }
                    continue;
                }
                Ok(_) => {
                    return Envelope::error(
                        DiagnosticCode::InvalidBundleStructure,
                        "documents/index.json is not a JSON object",
                    );
                }
                Err(e) => return e,
            }
        }

        // Everything else (documents/files/**, vault/**, unknown) →
        // attachments map, base64-encoded, keyed by ZIP path.
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        if let Err(e) = entry.read_to_end(&mut bytes) {
            return Envelope::error(
                DiagnosticCode::ZipReadError,
                format!("cannot read ZIP entry {name:?}: {e}"),
            );
        }
        attachments.insert(name, BASE64.encode(&bytes));
    }

    if let Err(env) = check_manifest_version(&manifest) {
        return env;
    }

    let bundle = FlatBundle {
        manifest,
        persons, families, events, links,
        occupations, sources, places, documents,
        attachments,
        extra: BTreeMap::new(),
    };
    let value = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok(value)
}

fn read_json<R: Read>(reader: &mut R) -> Result<Value, Envelope> {
    let mut buf = String::new();
    if let Err(e) = reader.read_to_string(&mut buf) {
        return Err(Envelope::error(
            DiagnosticCode::ZipReadError,
            format!("entry is not UTF-8 text: {e}"),
        ));
    }
    serde_json::from_str(&buf).map_err(|e| {
        Envelope::error(DiagnosticCode::InvalidJson, format!("invalid JSON: {e}"))
    })
}

/// Split a ZIP path of the form `persons/<uuid>.json` into
/// `(collection, uuid)`. Returns `None` for paths that are not a
/// per-entity file under one of the seven per-file directories
/// (documents are handled separately via `documents/index.json`).
fn split_entity_path(name: &str) -> Option<(&str, &str)> {
    for (collection, dir) in ENTITY_DIRS.iter().take(7) {
        let prefix = format!("{dir}/");
        if let Some(rest) = name.strip_prefix(&prefix) {
            if let Some(id) = rest.strip_suffix(".json") {
                if !id.is_empty() && !id.contains('/') {
                    return Some((collection, id));
                }
            }
        }
    }
    None
}

// -------------------------------------------------------------------------
// export_bundle
// -------------------------------------------------------------------------

/// See [`crate::export_bundle`].
pub fn export_bundle(flat_json: &str) -> Envelope {
    let mut bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }

    // Recompute stats and refresh updated_at. Compute stats first to
    // avoid borrowing bundle both mutably (as .manifest) and immutably
    // (for entity counts) in the same expression.
    let fresh_stats = compute_stats(&bundle);
    let now = now_iso8601_utc();
    if let Value::Object(ref mut m) = bundle.manifest {
        m.insert("stats".into(), fresh_stats);
        m.insert("updated_at".into(), Value::String(now));
    }

    let mut buf = Vec::with_capacity(16 * 1024);
    let cursor = Cursor::new(&mut buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);

    // Helper closure to add a JSON entry.
    let write_json_entry = |zip: &mut ZipWriter<_>, path: &str, value: &Value| -> Result<(), String> {
        let text = serde_json::to_string_pretty(value)
            .map_err(|e| format!("serialize {path}: {e}"))?;
        zip.start_file(path, opts).map_err(|e| format!("zip start {path}: {e}"))?;
        zip.write_all(text.as_bytes())
            .map_err(|e| format!("zip write {path}: {e}"))?;
        Ok(())
    };

    let write_bytes_entry = |zip: &mut ZipWriter<_>, path: &str, bytes: &[u8]| -> Result<(), String> {
        zip.start_file(path, opts).map_err(|e| format!("zip start {path}: {e}"))?;
        zip.write_all(bytes).map_err(|e| format!("zip write {path}: {e}"))?;
        Ok(())
    };

    if let Err(e) = write_json_entry(&mut zip, "manifest.json", &bundle.manifest) {
        return Envelope::error(DiagnosticCode::ZipWriteError, e);
    }

    // Embed the canonical schema.
    if let Err(e) = write_bytes_entry(
        &mut zip,
        "schema/axgf-1.0.schema.json",
        EMBEDDED_SCHEMA.as_bytes(),
    ) {
        return Envelope::error(DiagnosticCode::ZipWriteError, e);
    }

    // Per-entity directories (seven of them).
    let per_entity: [(&str, &BTreeMap<String, Value>); 7] = [
        ("persons",     &bundle.persons),
        ("families",    &bundle.families),
        ("events",      &bundle.events),
        ("links",       &bundle.links),
        ("occupations", &bundle.occupations),
        ("sources",     &bundle.sources),
        ("places",      &bundle.places),
    ];
    for (dir, map) in per_entity {
        for (id, value) in map {
            let path = format!("{dir}/{id}.json");
            if let Err(e) = write_json_entry(&mut zip, &path, value) {
                return Envelope::error(DiagnosticCode::ZipWriteError, e);
            }
        }
    }

    // documents/index.json aggregates all document metadata.
    let doc_index: Value = bundle
        .documents
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<serde_json::Map<_, _>>()
        .into();
    if let Err(e) = write_json_entry(&mut zip, "documents/index.json", &doc_index) {
        return Envelope::error(DiagnosticCode::ZipWriteError, e);
    }

    // Attachments (document files, vault pages, anything else) restored
    // at their original ZIP paths.
    for (path, b64) in &bundle.attachments {
        let bytes = match BASE64.decode(b64.as_bytes()) {
            Ok(b) => b,
            Err(e) => {
                return Envelope::error(
                    DiagnosticCode::InvalidBundleStructure,
                    format!("attachment {path:?} is not valid base64: {e}"),
                );
            }
        };
        if let Err(e) = write_bytes_entry(&mut zip, path, &bytes) {
            return Envelope::error(DiagnosticCode::ZipWriteError, e);
        }
    }

    if let Err(e) = zip.finish() {
        return Envelope::error(DiagnosticCode::ZipWriteError, format!("zip finish: {e}"));
    }
    drop(zip);

    Envelope::ok(json!({
        "zip_base64": BASE64.encode(&buf),
        "size_bytes": buf.len(),
    }))
}
