// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the bundle lifecycle operations.
//!
//! Covers the Phase 3 contract:
//!
//! - `create_bundle` produces a valid empty bundle whose manifest
//!   declares the current spec version and zero stats.
//! - `inspect` returns a computed stats block that reflects the actual
//!   entity counts.
//! - `import_bundle` refuses any bundle whose `manifest.axgf` is not in
//!   `SUPPORTED_SPEC_VERSIONS` with a stable `UNSUPPORTED_SPEC_VERSION`
//!   diagnostic.
//! - Round-trip: `create → export → import → export` yields the same
//!   set of entities and preserves attachments verbatim.
//! - `export_bundle` recomputes stats and embeds the canonical schema.

use axgf_rs::boundary::envelope::{Envelope, Status};
use axgf_rs::{create_bundle, export_bundle, import_bundle, inspect};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde_json::{json, Value};

/// Parse an `Envelope` JSON string back into a rich value for testing.
fn parse_env(s: &str) -> Envelope {
    serde_json::from_str(s).expect("envelope should be valid JSON")
}

/// Read a specific file's bytes out of a ZIP by path.
fn read_zip_entry(zip_bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let cur = std::io::Cursor::new(zip_bytes);
    let mut ar = zip::ZipArchive::new(cur).ok()?;
    let mut file = ar.by_name(name).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf).ok()?;
    Some(buf)
}

fn list_zip_names(zip_bytes: &[u8]) -> Vec<String> {
    let cur = std::io::Cursor::new(zip_bytes);
    let mut ar = zip::ZipArchive::new(cur).unwrap();
    (0..ar.len()).map(|i| ar.by_index(i).unwrap().name().to_string()).collect()
}

// ---------- create_bundle ----------

#[test]
fn create_bundle_returns_valid_empty_bundle() {
    let env = create_bundle(None);
    assert_eq!(env.status, Status::Ok);
    assert!(env.diagnostics.is_empty());

    let flat = &env.data;
    assert_eq!(flat["manifest"]["axgf"], "1.0");
    // A well-formed created_at looks like "2026-...T..." — spec-compliant ISO 8601.
    assert!(flat["manifest"]["created_at"].as_str().unwrap().starts_with("20"));
    // Empty bundle: stats all zero.
    let s = &flat["manifest"]["stats"];
    for k in ["persons", "families", "events", "links", "occupations", "sources", "places", "documents"] {
        assert_eq!(s[k], 0, "expected {k} = 0 in fresh bundle");
    }
    // No family_name provided ⇒ no `family` block.
    assert!(flat["manifest"].get("family").is_none());
}

#[test]
fn create_bundle_with_family_name_populates_manifest() {
    let env = create_bundle(Some("Famille Pierre-Léonard"));
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["manifest"]["family"]["name"], "Famille Pierre-Léonard");
}

// ---------- inspect ----------

#[test]
fn inspect_recomputes_stats_from_actual_entities() {
    // A bundle where the manifest's stats disagree with reality —
    // inspect must return the *computed* stats, not the stale ones.
    let flat = json!({
        "manifest": {"axgf": "1.0", "created_at": "2026-06-15T10:00:00Z",
                     "stats": {"persons": 9999}},
        "persons": {"a": {"id": "a"}, "b": {"id": "b"}, "c": {"id": "c"}},
        "families": {"x": {"id": "x"}}
    });
    let s = serde_json::to_string(&flat).unwrap();
    let env = inspect(&s);
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["stats"]["persons"], 3);
    assert_eq!(env.data["stats"]["families"], 1);
    assert_eq!(env.data["stats"]["events"], 0);
    // The manifest is returned as-was (stale stats preserved so callers
    // can compare declared vs computed).
    assert_eq!(env.data["manifest"]["stats"]["persons"], 9999);
}

#[test]
fn inspect_rejects_unsupported_spec_version() {
    let flat = json!({"manifest": {"axgf": "9.9", "created_at": "x", "stats": {"persons": 0}}});
    let s = serde_json::to_string(&flat).unwrap();
    let env = inspect(&s);
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics.len(), 1);
    assert_eq!(env.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
    assert!(env.data.is_null());
}

#[test]
fn inspect_rejects_invalid_json() {
    let env = inspect("not-json");
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "INVALID_JSON");
}

// ---------- export_bundle ----------

#[test]
fn export_bundle_produces_zip_with_manifest_and_schema() {
    // Create → export.
    let created = create_bundle(Some("test"));
    let flat_json = serde_json::to_string(&created.data).unwrap();
    let exported = export_bundle(&flat_json);
    assert_eq!(exported.status, Status::Ok);
    let b64 = exported.data["zip_base64"].as_str().expect("zip_base64 field");
    let zip_bytes = BASE64.decode(b64).unwrap();
    assert!(exported.data["size_bytes"].as_u64().unwrap() > 0);

    let names = list_zip_names(&zip_bytes);
    assert!(names.contains(&"manifest.json".to_string()), "names={names:?}");
    assert!(names.contains(&"schema/axgf-1.0.schema.json".to_string()));
    assert!(names.contains(&"documents/index.json".to_string()));

    // Embedded schema must be the canonical (fixed) one — spot-check
    // that primitives AND entities are both present in $defs.
    let schema_bytes = read_zip_entry(&zip_bytes, "schema/axgf-1.0.schema.json").unwrap();
    let schema: Value = serde_json::from_slice(&schema_bytes).unwrap();
    let defs = schema["$defs"].as_object().unwrap();
    for k in ["uuid", "axgf_date", "base_entity", "person", "manifest", "document"] {
        assert!(defs.contains_key(k), "embedded schema missing $defs/{k}");
    }
}

#[test]
fn export_bundle_recomputes_stats_before_writing() {
    // Feed a bundle with wrong stats declared in the manifest; the
    // export must overwrite them with the true counts.
    let mut flat = create_bundle(None).data;
    flat["persons"] = json!({"a": {"id": "a"}, "b": {"id": "b"}});
    flat["families"] = json!({"f1": {"id": "f1"}});
    flat["manifest"]["stats"]["persons"] = json!(9999);

    let exp = export_bundle(&serde_json::to_string(&flat).unwrap());
    assert_eq!(exp.status, Status::Ok);
    let zip_bytes = BASE64.decode(exp.data["zip_base64"].as_str().unwrap()).unwrap();
    let manifest_bytes = read_zip_entry(&zip_bytes, "manifest.json").unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["stats"]["persons"], 2);
    assert_eq!(manifest["stats"]["families"], 1);
    // updated_at was refreshed.
    assert!(manifest["updated_at"].is_string());
}

#[test]
fn export_rejects_unsupported_spec_version() {
    let mut flat = create_bundle(None).data;
    flat["manifest"]["axgf"] = json!("9.9");
    let env = export_bundle(&serde_json::to_string(&flat).unwrap());
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
}

// ---------- import_bundle ----------

#[test]
fn import_of_freshly_exported_bundle_yields_equivalent_flat_json() {
    // Round-trip: create → export → import ⇒ equivalent flat bundle.
    let mut flat = create_bundle(Some("Round-Trip")).data;
    // Add one of each kind.
    flat["persons"] = json!({"p1": {"id": "p1", "type": "person", "axgf_version": "1.0",
                                     "identity": {"name": {"display": "A", "components": []},
                                                  "gender": {"value": "U"}, "is_living": false}}});
    flat["families"] = json!({"f1": {"id": "f1", "type": "family", "axgf_version": "1.0",
                                      "union": {"type": "unknown",
                                                "persons": [{"person_id": "p1", "role": "spouse"}]}}});
    flat["events"] = json!({"e1": {"id": "e1", "type": "event", "axgf_version": "1.0",
                                    "category": "birth", "date": {"value": "1900"}}});
    let flat_str = serde_json::to_string(&flat).unwrap();

    let exp = export_bundle(&flat_str);
    let zip_bytes = BASE64.decode(exp.data["zip_base64"].as_str().unwrap()).unwrap();

    let imp = import_bundle(&zip_bytes);
    assert_eq!(imp.status, Status::Ok, "import failed: {:?}", imp.diagnostics);

    // Manifest survives with the right axgf version.
    assert_eq!(imp.data["manifest"]["axgf"], "1.0");
    // Stats reflect the two persons/one family/one event we stuffed in.
    // (Note: manifest.stats was recomputed by export.)
    assert_eq!(imp.data["manifest"]["stats"]["persons"], 1);
    assert_eq!(imp.data["manifest"]["stats"]["families"], 1);
    assert_eq!(imp.data["manifest"]["stats"]["events"], 1);
    // Entities themselves round-tripped.
    assert_eq!(imp.data["persons"]["p1"]["identity"]["name"]["display"], "A");
    assert_eq!(imp.data["families"]["f1"]["union"]["persons"][0]["person_id"], "p1");
    assert_eq!(imp.data["events"]["e1"]["date"]["value"], "1900");
}

#[test]
fn import_rejects_unsupported_spec_version() {
    // Build a minimal ZIP with axgf = "9.9".
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", opts).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"{"axgf":"9.9","created_at":"x","stats":{"persons":0}}"#,
        ).unwrap();
        zip.finish().unwrap();
    }
    let env = import_bundle(&buf);
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "UNSUPPORTED_SPEC_VERSION");
    assert!(env.diagnostics[0].message.contains("9.9"));
}

#[test]
fn import_rejects_non_zip_input() {
    let env = import_bundle(b"this is not a zip");
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "ZIP_READ_ERROR");
}

#[test]
fn import_preserves_attachments_and_vault_pages_round_trip() {
    // Build a bundle by hand with two attachments: a document binary
    // and a vault Markdown page. Both must survive export→import→export.
    let mut flat = create_bundle(None).data;
    flat["documents"] = json!({"d1": {"id": "d1", "type": "document", "axgf_version": "1.0",
                                       "filename": "hello.txt", "mime_type": "text/plain",
                                       "document_type": "letter", "status": "present",
                                       "file": {"path": "documents/files/d1.txt"}}});
    let flat_obj = flat.as_object_mut().unwrap();
    let attachments = flat_obj.entry("attachments").or_insert_with(|| json!({}));
    attachments["documents/files/d1.txt"] = json!(BASE64.encode(b"Hello, AXGF!"));
    attachments["vault/wiki/persons/example.md"] = json!(BASE64.encode(b"# Example\nBody."));

    let exp = export_bundle(&serde_json::to_string(&flat).unwrap());
    assert_eq!(exp.status, Status::Ok);
    let zip_bytes = BASE64.decode(exp.data["zip_base64"].as_str().unwrap()).unwrap();

    // Attachments landed in the ZIP at their original paths.
    let names = list_zip_names(&zip_bytes);
    assert!(names.contains(&"documents/files/d1.txt".to_string()));
    assert!(names.contains(&"vault/wiki/persons/example.md".to_string()));
    assert_eq!(read_zip_entry(&zip_bytes, "documents/files/d1.txt").unwrap(), b"Hello, AXGF!");
    assert_eq!(read_zip_entry(&zip_bytes, "vault/wiki/persons/example.md").unwrap(),
               b"# Example\nBody.");

    // Now import and check they come back through the attachments map.
    let imp = import_bundle(&zip_bytes);
    assert_eq!(imp.status, Status::Ok);
    let att = &imp.data["attachments"];
    assert_eq!(BASE64.decode(att["documents/files/d1.txt"].as_str().unwrap()).unwrap(),
               b"Hello, AXGF!");
    assert_eq!(BASE64.decode(att["vault/wiki/persons/example.md"].as_str().unwrap()).unwrap(),
               b"# Example\nBody.");
    // The document metadata came back via documents/index.json.
    assert_eq!(imp.data["documents"]["d1"]["filename"], "hello.txt");
}

// ---------- Envelope round-trip through the boundary ----------

#[test]
fn error_envelope_serializes_and_parses_back() {
    let env = inspect("not-json");
    let wire = env.to_json();
    let parsed = parse_env(&wire);
    assert_eq!(parsed.status, Status::Error);
    assert_eq!(parsed.diagnostics[0].code.as_str(), "INVALID_JSON");
}
