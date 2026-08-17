// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the streaming payload API.
//!
//! Covers the contract added in 0.3.0:
//!
//! - `import_bundle_textual` returns the manifest and all eight collections
//!   with document metadata intact and not one payload byte decoded.
//! - `import_bundle_streaming` hands over each payload exactly once, in
//!   archive order, and records what it streamed in `external_payloads`.
//! - Streaming import → streaming export reproduces the source archive
//!   payload-for-payload, compared by SHA-256.
//! - A bundle whose payloads were streamed out is **refused** by the
//!   non-streaming `export_bundle` with `PAYLOADS_EXTERNAL`, rather than
//!   silently written with its media missing.
//! - A caller that fails, or that supplies nothing for a declared payload,
//!   aborts the export instead of producing a hollow archive.
//!
//! The bounded-memory claim is measured separately, in
//! `tests/streaming_memory.rs`.

mod common;

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use axgf_rs::boundary::envelope::{DiagnosticCode, Status};
use axgf_rs::{
    export_bundle, export_bundle_streaming, import_bundle, import_bundle_streaming,
    import_bundle_textual,
};
use common::{entry_names, payload_digests, sha256_hex, specs, synthetic_axgf, Spec};
use serde_json::Value;

/// A small fixture: four payloads, a few KiB each. Enough to exercise every
/// path without the cost of the memory fixtures.
fn small_fixture() -> (Vec<u8>, Vec<(String, String)>) {
    let s = vec![
        Spec {
            path: "documents/files/aaaaaaaa-0000-4000-8000-000000000000.bin".into(),
            size: 4096,
        },
        Spec {
            path: "documents/files/bbbbbbbb-0000-4000-8000-000000000000.bin".into(),
            size: 8192,
        },
        Spec {
            path: "documents/files/cccccccc-0000-4000-8000-000000000000.bin".into(),
            size: 1024,
        },
        Spec {
            path: "vault/wiki/persons/note.md".into(),
            size: 512,
        },
    ];
    synthetic_axgf(&s, true)
}

// ---------- import_bundle_textual ----------

#[test]
fn textual_import_returns_every_collection_without_decoding_a_payload() {
    let (zip, digests) = small_fixture();
    let env = import_bundle_textual(Cursor::new(&zip));
    assert_eq!(env.status, Status::Ok);

    let flat = &env.data;
    // The manifest and the eight collections are all present.
    assert_eq!(flat["manifest"]["axgf"], "1.0");
    for key in [
        "persons",
        "families",
        "events",
        "links",
        "occupations",
        "sources",
        "places",
        "documents",
    ] {
        assert!(
            flat[key].is_object(),
            "collection {key} must be present even when empty"
        );
    }
    assert_eq!(flat["persons"].as_object().unwrap().len(), 2);

    // Document metadata survives in full — path, size, the lot.
    let docs = flat["documents"].as_object().expect("documents object");
    assert_eq!(docs.len(), 4);
    let a_doc = docs.values().next().unwrap();
    assert!(a_doc["file"]["path"].as_str().is_some());
    assert!(a_doc["file"]["size_bytes"].as_u64().is_some());

    // Not one payload byte was decoded: `attachments` is absent entirely.
    assert!(
        flat.get("attachments").is_none(),
        "textual import must not populate attachments, got {:?}",
        flat.get("attachments")
    );

    // Every payload is instead declared, with the metadata the central
    // directory already carried.
    let ext = flat["external_payloads"]
        .as_object()
        .expect("external_payloads must be present");
    assert_eq!(ext.len(), 4);
    for (path, _) in &digests {
        let e = &ext[path];
        assert!(e["size_bytes"].as_u64().unwrap() > 0);
        // crc32 is present (it may legitimately be any value, including 0).
        assert!(e["crc32"].is_number(), "crc32 recorded for {path}");
    }
    // The vault page is a payload too — anything that is not entity JSON is.
    assert!(ext.contains_key("vault/wiki/persons/note.md"));
}

#[test]
fn textual_import_reports_what_it_left_behind() {
    let (zip, _) = small_fixture();
    let env = import_bundle_textual(Cursor::new(&zip));
    // Informational, not a warning: streaming is a deliberate choice. But it
    // must be visible in the envelope that this JSON has no media in it.
    let d = env
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::PayloadsExternal)
        .expect("an informational PAYLOADS_EXTERNAL diagnostic");
    assert!(d.message.contains("export_bundle_streaming"));
}

#[test]
fn a_bundle_with_no_payloads_streams_without_the_marker_or_the_note() {
    let (zip, _) = synthetic_axgf(&[], true);
    let env = import_bundle_textual(Cursor::new(&zip));
    assert_eq!(env.status, Status::Ok);
    // No payloads means no marker, so such a bundle stays exportable by the
    // ordinary path — streaming must not infect bundles that do not need it.
    assert!(env.data.get("external_payloads").is_none());
    assert!(env.diagnostics.is_empty());
    let round = export_bundle(&env.data.to_string());
    assert_eq!(round.status, Status::Ok);
}

// ---------- import_bundle_streaming ----------

#[test]
fn streaming_import_hands_over_every_payload_exactly_once() {
    let (zip, digests) = small_fixture();
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut sizes: Vec<(String, u64)> = Vec::new();

    let env = import_bundle_streaming(Cursor::new(&zip), |p| {
        sizes.push((p.path().to_string(), p.size()));
        let bytes = p.read_to_end()?;
        seen.push((p.path().to_string(), sha256_hex(&bytes)));
        Ok(())
    });
    assert_eq!(env.status, Status::Ok);

    seen.sort();
    let mut expected = digests.clone();
    expected.sort();
    assert_eq!(seen, expected, "every payload delivered, bytes intact");

    // The size handed to the callback matches the real payload length, and
    // is known before any decompression.
    for (path, size) in sizes {
        let want = match path.as_str() {
            p if p.ends_with("aaaaaaaa-0000-4000-8000-000000000000.bin") => 4096,
            p if p.ends_with("bbbbbbbb-0000-4000-8000-000000000000.bin") => 8192,
            p if p.ends_with("cccccccc-0000-4000-8000-000000000000.bin") => 1024,
            _ => 512,
        };
        assert_eq!(size, want, "declared size for {path}");
    }
}

#[test]
fn a_payload_the_caller_ignores_is_skipped_not_buffered() {
    // Reading nothing must still advance to the next entry correctly — this
    // is what makes `import_bundle_textual` a one-line special case.
    let (zip, _) = small_fixture();
    let mut count = 0;
    let env = import_bundle_streaming(Cursor::new(&zip), |_p| {
        count += 1;
        Ok(())
    });
    assert_eq!(env.status, Status::Ok);
    assert_eq!(count, 4);
    assert_eq!(env.data["persons"].as_object().unwrap().len(), 2);
}

#[test]
fn copy_to_moves_a_payload_without_holding_it() {
    let (zip, digests) = small_fixture();
    let mut out: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let env = import_bundle_streaming(Cursor::new(&zip), |p| {
        let mut sink = Vec::new();
        let n = p.copy_to(&mut sink)?;
        assert_eq!(n, p.size(), "copy_to reports the full length");
        out.insert(p.path().to_string(), sink);
        Ok(())
    });
    assert_eq!(env.status, Status::Ok);
    for (path, digest) in digests {
        assert_eq!(sha256_hex(&out[&path]), digest, "{path} copied intact");
    }
}

#[test]
fn a_failing_payload_handler_aborts_the_import_with_a_named_diagnostic() {
    let (zip, _) = small_fixture();
    let env = import_bundle_streaming(Cursor::new(&zip), |p| {
        if p.path().contains("bbbbbbbb") {
            return Err(std::io::Error::other("disk full"));
        }
        Ok(())
    });
    assert_eq!(env.status, Status::Error);
    assert!(env.data.is_null());
    assert_eq!(env.diagnostics[0].code, DiagnosticCode::PayloadSinkFailed);
    let m = &env.diagnostics[0].message;
    assert!(m.contains("bbbbbbbb"), "names the payload: {m}");
    assert!(m.contains("disk full"), "carries the caller's error: {m}");
}

#[test]
fn streaming_import_rejects_an_unsupported_spec_version() {
    // The version gate applies to every entry point, streaming included.
    let (zip, _) = small_fixture();
    let mut patched = Vec::new();
    {
        // Rewrite manifest.json with a future spec version.
        let mut ar = zip::ZipArchive::new(Cursor::new(&zip)).unwrap();
        let cursor = Cursor::new(&mut patched);
        let mut w = zip::ZipWriter::new(cursor);
        let opts =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for i in 0..ar.len() {
            let mut f = ar.by_index(i).unwrap();
            let name = f.name().to_string();
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            if name == "manifest.json" {
                let mut v: Value = serde_json::from_slice(&buf).unwrap();
                v["axgf"] = Value::String("2.0".into());
                buf = serde_json::to_vec_pretty(&v).unwrap();
            }
            w.start_file(name, opts).unwrap();
            w.write_all(&buf).unwrap();
        }
        w.finish().unwrap();
    }
    let env = import_bundle_textual(Cursor::new(&patched));
    assert_eq!(env.status, Status::Error);
    assert_eq!(
        env.diagnostics[0].code,
        DiagnosticCode::UnsupportedSpecVersion
    );
}

#[test]
fn streaming_import_rejects_non_zip_input() {
    let env = import_bundle_textual(Cursor::new(b"not a zip at all".to_vec()));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code, DiagnosticCode::ZipReadError);
}

// ---------- the silent-empty-bundle case ----------

#[test]
fn the_old_export_refuses_a_bundle_whose_payloads_were_streamed_out() {
    // This is the failure the marker exists to prevent: stream the media out,
    // reach for the familiar export, and get an archive with every photograph
    // missing. It must be refused, loudly, before anything is written.
    let (zip, _) = small_fixture();
    let flat = import_bundle_textual(Cursor::new(&zip)).data.to_string();

    let env = export_bundle(&flat);
    assert_eq!(env.status, Status::Error);
    assert!(env.data.is_null(), "nothing is produced at all");
    assert_eq!(env.diagnostics[0].code, DiagnosticCode::PayloadsExternal);
    let m = &env.diagnostics[0].message;
    assert!(m.contains("export_bundle_streaming"), "names the fix: {m}");
    assert!(m.contains('4'), "says how many are missing: {m}");
}

#[test]
fn the_marker_survives_an_edit_so_the_refusal_cannot_be_laundered_away() {
    // A caller who streams payloads out, then edits an entity, then exports
    // must still be refused — otherwise a round-trip through CRUD would
    // quietly clear the marker and reintroduce the data loss.
    let (zip, _) = small_fixture();
    let flat = import_bundle_textual(Cursor::new(&zip)).data.to_string();

    let person = r#"{"identity": {"name": {"display": "Added Later", "components": []},
                     "is_living": true}}"#;
    let added = axgf_rs::add_entity(&flat, axgf_rs::EntityKind::Person, person);
    assert_eq!(added.status, Status::Ok);
    let edited = added.data["bundle"].to_string();
    assert!(
        added.data["bundle"]["external_payloads"].is_object(),
        "add_entity must carry the marker through"
    );

    let env = export_bundle(&edited);
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code, DiagnosticCode::PayloadsExternal);
}

// ---------- export_bundle_streaming ----------

#[test]
fn streaming_import_then_streaming_export_reproduces_every_payload() {
    let (zip, source_digests) = small_fixture();

    // Stream in, keeping payloads in a side store (as a caller would keep
    // them on disk).
    let mut store: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let imported = import_bundle_streaming(Cursor::new(&zip), |p| {
        let mut v = Vec::new();
        p.copy_to(&mut v)?;
        store.insert(p.path().to_string(), v);
        Ok(())
    });
    assert_eq!(imported.status, Status::Ok);
    let flat = imported.data.to_string();

    // Stream back out, supplying each payload on demand.
    let mut out = Cursor::new(Vec::new());
    let exported = export_bundle_streaming(&flat, &mut out, |slot| {
        let bytes = store
            .get(slot.path())
            .ok_or_else(|| std::io::Error::other(format!("no payload for {}", slot.path())))?;
        slot.write_all(bytes)?;
        Ok(())
    });
    assert_eq!(exported.status, Status::Ok, "{:?}", exported.diagnostics);
    assert_eq!(exported.data["payloads_written"], 4);
    assert!(exported.data["size_bytes"].as_u64().unwrap() > 0);

    let rebuilt = out.into_inner();

    // Payload for payload, by digest.
    let mut expect = source_digests.clone();
    expect.sort();
    assert_eq!(payload_digests(&rebuilt), expect);

    // And the archive carries the same entries (plus the embedded schema,
    // which export always writes fresh).
    let names = entry_names(&rebuilt);
    assert!(names.contains(&"manifest.json".to_string()));
    assert!(names.contains(&"documents/index.json".to_string()));
    assert!(names.contains(&"schema/axgf-1.0.schema.json".to_string()));
    for (path, _) in &source_digests {
        assert!(
            names.contains(path),
            "{path} present in the rebuilt archive"
        );
    }

    // Re-importing the rebuilt archive gives back the same entities.
    let reimported = import_bundle_textual(Cursor::new(&rebuilt));
    assert_eq!(reimported.status, Status::Ok);
    assert_eq!(reimported.data["persons"], imported.data["persons"]);
    assert_eq!(reimported.data["documents"], imported.data["documents"]);
}

#[test]
fn a_streamed_export_equals_the_non_streaming_export_payload_for_payload() {
    // The two export paths must agree: same payloads, same digests. Only
    // `updated_at` in the manifest legitimately differs between two exports.
    let (zip, _) = small_fixture();

    let inline = import_bundle(&zip);
    assert_eq!(inline.status, Status::Ok);
    let via_old = export_bundle(&inline.data.to_string());
    assert_eq!(via_old.status, Status::Ok);
    let old_bytes = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(via_old.data["zip_base64"].as_str().unwrap())
            .unwrap()
    };

    let mut store: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let streamed = import_bundle_streaming(Cursor::new(&zip), |p| {
        store.insert(p.path().to_string(), p.read_to_end()?);
        Ok(())
    });
    let mut out = Cursor::new(Vec::new());
    let via_new = export_bundle_streaming(&streamed.data.to_string(), &mut out, |slot| {
        slot.write_all(&store[slot.path()])?;
        Ok(())
    });
    assert_eq!(via_new.status, Status::Ok);
    let new_bytes = out.into_inner();

    assert_eq!(
        payload_digests(&old_bytes),
        payload_digests(&new_bytes),
        "both export paths must write identical payloads"
    );
    assert_eq!(entry_names(&old_bytes), entry_names(&new_bytes));
}

#[test]
fn a_caller_that_supplies_nothing_is_refused_rather_than_writing_hollow_media() {
    // The other half of the silent-empty-bundle defence: even on the
    // streaming path, a payload the bundle declares but the caller does not
    // produce must stop the export.
    let (zip, _) = small_fixture();
    let flat = import_bundle_textual(Cursor::new(&zip)).data.to_string();

    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat, &mut out, |_slot| Ok(()));
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code, DiagnosticCode::PayloadSourceFailed);
    let m = &env.diagnostics[0].message;
    assert!(m.contains("wrote nothing"), "{m}");
    assert!(m.contains("missing media"), "{m}");
}

#[test]
fn a_failing_payload_source_aborts_the_export() {
    let (zip, _) = small_fixture();
    let flat = import_bundle_textual(Cursor::new(&zip)).data.to_string();
    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat, &mut out, |slot| {
        Err(std::io::Error::other(format!(
            "cache miss for {}",
            slot.path()
        )))
    });
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code, DiagnosticCode::PayloadSourceFailed);
    assert!(env.diagnostics[0].message.contains("cache miss"));
}

#[test]
fn a_payload_of_a_different_length_is_written_but_flagged() {
    // A caller may legitimately have replaced a file since import. That is
    // allowed — but it is not allowed to be silent.
    let (zip, _) = small_fixture();
    let flat = import_bundle_textual(Cursor::new(&zip)).data.to_string();
    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat, &mut out, |slot| {
        slot.write_all(b"replaced")?;
        Ok(())
    });
    assert_eq!(env.status, Status::Ok, "a replacement is not an error");
    let warn = env
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::PayloadSourceFailed)
        .expect("a warning about the length change");
    assert_eq!(
        warn.severity,
        axgf_rs::boundary::envelope::Severity::Warning
    );
    assert!(warn.message.contains("declared as"));
}

#[test]
fn the_slot_tells_the_caller_what_is_expected() {
    let (zip, _) = small_fixture();
    let flat = import_bundle_textual(Cursor::new(&zip)).data.to_string();
    let mut seen: Vec<(String, u64)> = Vec::new();
    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat, &mut out, |slot| {
        seen.push((slot.path().to_string(), slot.expected_size()));
        // Satisfy the slot so the export succeeds.
        let n = slot.expected_size() as usize;
        slot.write_all(&vec![7u8; n])?;
        Ok(())
    });
    assert_eq!(env.status, Status::Ok, "{:?}", env.diagnostics);
    seen.sort();
    assert_eq!(seen.len(), 4);
    assert!(seen.iter().all(|(_, n)| *n > 0), "sizes are known up front");
}

#[test]
fn a_bundle_still_carrying_inline_attachments_exports_by_the_streaming_path_too() {
    // A caller need not convert everything at once: inline attachments are
    // written from the JSON, external ones are requested. Mixed bundles work.
    let (zip, digests) = small_fixture();
    let inline = import_bundle(&zip);
    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&inline.data.to_string(), &mut out, |slot| {
        panic!("no external payloads to ask for, got {}", slot.path())
    });
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["payloads_written"], 4);
    let mut expect = digests;
    expect.sort();
    assert_eq!(payload_digests(&out.into_inner()), expect);
}

#[test]
fn a_payload_declared_both_inline_and_external_is_refused_as_incoherent() {
    let (zip, _) = small_fixture();
    let inline = import_bundle(&zip);
    let mut flat: Value = inline.data;
    // Forge the contradiction: the same path carried both ways.
    let path = "documents/files/aaaaaaaa-0000-4000-8000-000000000000.bin";
    flat["external_payloads"] = serde_json::json!({ path: {"size_bytes": 4096, "crc32": 0} });

    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat.to_string(), &mut out, |_s| Ok(()));
    assert_eq!(env.status, Status::Error);
    assert_eq!(
        env.diagnostics[0].code,
        DiagnosticCode::InvalidBundleStructure
    );
    assert!(env.diagnostics[0].message.contains("both attachments"));
}

#[test]
fn streaming_export_recomputes_stats_like_the_non_streaming_one() {
    let (zip, _) = synthetic_axgf(&specs(0, 0, 64), true);
    let mut flat: Value = import_bundle_textual(Cursor::new(&zip)).data;
    // Hand-corrupt the stats, as a client editing the JSON might.
    flat["manifest"]["stats"]["persons"] = Value::from(99);

    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat.to_string(), &mut out, |slot| {
        slot.write_all(&vec![0u8; slot.expected_size() as usize])?;
        Ok(())
    });
    assert_eq!(env.status, Status::Ok);

    let rebuilt = out.into_inner();
    let back = import_bundle_textual(Cursor::new(&rebuilt));
    assert_eq!(
        back.data["manifest"]["stats"]["persons"], 2,
        "export must reconcile stats against the real entities"
    );
}

#[test]
fn streaming_export_rejects_an_unsupported_spec_version() {
    let (zip, _) = small_fixture();
    let mut flat: Value = import_bundle_textual(Cursor::new(&zip)).data;
    flat["manifest"]["axgf"] = Value::String("9.9".into());
    let mut out = Cursor::new(Vec::new());
    let env = export_bundle_streaming(&flat.to_string(), &mut out, |_s| Ok(()));
    assert_eq!(env.status, Status::Error);
    assert_eq!(
        env.diagnostics[0].code,
        DiagnosticCode::UnsupportedSpecVersion
    );
}
