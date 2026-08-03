// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the `axgf` command-line binary.
//!
//! The tests spawn the freshly-built binary via `CARGO_BIN_EXE_axgf` (set
//! automatically by Cargo when a `[[bin]]` target is in scope) and inspect
//! its stdout as an [`Envelope`]. The whole file is elided when the `cli`
//! feature is off — plain `cargo test` still passes with the baseline 82.

#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

use axgf_rs::boundary::envelope::{Envelope, Status};
use serde_json::{json, Value};

fn axgf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_axgf"))
}

/// Run `axgf` with the given args, feeding `stdin_bytes` if non-empty.
/// Panics on spawn failure. Returns (exit_code, stdout_bytes, stderr_string).
fn run(args: &[&str], stdin_bytes: &[u8]) -> (i32, Vec<u8>, String) {
    let mut child = axgf()
        .args(args)
        .stdin(if stdin_bytes.is_empty() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn axgf");
    if !stdin_bytes.is_empty() {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin_bytes)
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    (
        out.status.code().unwrap_or(-1),
        out.stdout,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn parse_envelope(stdout: &[u8]) -> Envelope {
    let text = std::str::from_utf8(stdout).expect("utf-8 stdout");
    // The binary prints exactly one envelope followed by a newline.
    serde_json::from_str(text.trim())
        .unwrap_or_else(|e| panic!("expected an Envelope on stdout, got: {text}\nparse error: {e}"))
}

/// A minimal but structurally valid person entity used across CRUD tests.
fn person(name: &str) -> Value {
    json!({
        "identity": {
            "name":   { "display": name, "components": [] },
            "gender": { "value": "M" },
            "is_living": false,
            "visibility": "members"
        }
    })
}

// -------------------------------------------------------------------------
// help / usage
// -------------------------------------------------------------------------

#[test]
fn help_prints_usage_and_exits_zero() {
    let (code, stdout, _stderr) = run(&["--help"], b"");
    assert_eq!(code, 0, "--help should exit 0");
    let s = std::str::from_utf8(&stdout).unwrap();
    // Every V1 API function must be listed as a subcommand.
    for sub in [
        "create",
        "import",
        "export",
        "inspect",
        "validate",
        "add",
        "update",
        "delete",
        "dedup",
        "convert-gedcom",
    ] {
        assert!(s.contains(sub), "help missing subcommand `{sub}`:\n{s}");
    }
}

#[test]
fn unknown_subcommand_is_rejected_by_clap() {
    let (code, _stdout, stderr) = run(&["not-a-thing"], b"");
    assert_ne!(code, 0);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("error"),
        "expected clap usage error on stderr, got: {stderr}"
    );
}

// -------------------------------------------------------------------------
// create
// -------------------------------------------------------------------------

#[test]
fn create_prints_ok_envelope_with_current_spec_version() {
    let (code, stdout, _stderr) = run(&["create"], b"");
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["manifest"]["axgf"], "1.0");
    // family key is omitted when no name is given.
    assert!(env.data["manifest"].get("family").is_none());
}

#[test]
fn create_with_family_name_populates_manifest() {
    let (code, stdout, _stderr) = run(&["create", "--family-name", "Karin"], b"");
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert_eq!(env.data["manifest"]["family"]["name"], "Karin");
}

// -------------------------------------------------------------------------
// inspect / validate on stdin
// -------------------------------------------------------------------------

#[test]
fn inspect_reads_flat_bundle_from_stdin() {
    let bundle = axgf_rs::create_bundle(Some("Karin")).data.to_string();
    let (code, stdout, _stderr) = run(&["inspect", "--input", "-"], bundle.as_bytes());
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["stats"]["persons"], 0);
}

#[test]
fn validate_of_clean_bundle_is_zero_exit_and_zero_diagnostics() {
    let bundle = axgf_rs::create_bundle(None).data.to_string();
    let (code, stdout, _stderr) = run(&["validate", "--input", "-"], bundle.as_bytes());
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert!(env.diagnostics.is_empty());
    assert_eq!(env.data["errors"], 0);
}

#[test]
fn validate_escalates_to_exit_two_on_error_severity_diagnostics() {
    // A person listed as both spouse and child of the same family is a
    // self-parent cycle. validate() emits `CYCLE_DETECTED` at Error severity
    // but the envelope stays Ok — the report is the answer. The CLI escalates
    // to exit 2 so shell one-liners can gate on it.
    let mut flat = axgf_rs::create_bundle(None).data;
    let p = "550e8400-e29b-41d4-a716-446655440001";
    let f = "aaaa1234-e29b-41d4-a716-446655440001";
    flat["persons"] = json!({
        p: { "id": p, "type": "person", "axgf_version": "1.0",
             "identity": { "name": {"display": "Loop", "components": []},
                           "gender": {"value": "U"}, "is_living": false } }
    });
    flat["families"] = json!({
        f: { "id": f, "type": "family", "axgf_version": "1.0",
             "union": { "type": "marriage",
                        "persons": [ { "person_id": p, "role": "spouse" } ] },
             "children": [ { "person_id": p } ] }
    });
    let (code, stdout, _stderr) = run(&["validate", "--input", "-"], flat.to_string().as_bytes());
    assert_eq!(code, 2, "expected exit 2 on error-severity diagnostic");
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Ok);
    assert!(env
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "CYCLE_DETECTED"));
}

#[test]
fn validate_input_that_is_not_json_yields_exit_one() {
    let (code, stdout, _stderr) = run(&["validate", "--input", "-"], b"definitely not JSON");
    assert_eq!(code, 1);
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Error);
}

// -------------------------------------------------------------------------
// add / update / delete
// -------------------------------------------------------------------------

#[test]
fn add_person_via_files_updates_the_bundle() {
    let dir = tempdir();
    let bundle_path = dir.join("bundle.json");
    let entity_path = dir.join("person.json");
    std::fs::write(&bundle_path, axgf_rs::create_bundle(None).data.to_string()).unwrap();
    std::fs::write(&entity_path, person("Jean").to_string()).unwrap();

    let (code, stdout, _stderr) = run(
        &[
            "add",
            "--input",
            bundle_path.to_str().unwrap(),
            "--kind",
            "person",
            "--entity",
            entity_path.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Ok);
    let new_id = env.data["id"].as_str().expect("id in data");
    assert_eq!(env.data["bundle"]["persons"][new_id]["type"], "person");
    assert_eq!(env.data["bundle"]["manifest"]["stats"]["persons"], 1);
}

#[test]
fn update_missing_entity_reports_entity_not_found_and_exits_one() {
    let bundle = axgf_rs::create_bundle(None).data.to_string();
    let dir = tempdir();
    let bundle_path = dir.join("bundle.json");
    let entity_path = dir.join("stale.json");
    std::fs::write(&bundle_path, &bundle).unwrap();
    let mut ghost = person("Ghost");
    ghost["id"] = json!("00000000-0000-4000-8000-000000000000");
    std::fs::write(&entity_path, ghost.to_string()).unwrap();

    let (code, stdout, _stderr) = run(
        &[
            "update",
            "--input",
            bundle_path.to_str().unwrap(),
            "--kind",
            "person",
            "--entity",
            entity_path.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(code, 1);
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Error);
    assert_eq!(env.diagnostics[0].code.as_str(), "ENTITY_NOT_FOUND");
}

#[test]
fn delete_defaults_to_reject_policy() {
    // Build a bundle: two persons + a family binding them. Deleting one
    // person should be refused because the family references it.
    let flat = axgf_rs::create_bundle(None).data.to_string();
    let a = axgf_rs::add_entity(&flat, axgf_rs::EntityKind::Person, &person("A").to_string());
    let flat = a.data["bundle"].to_string();
    let a_id = a.data["id"].as_str().unwrap().to_string();
    let b = axgf_rs::add_entity(&flat, axgf_rs::EntityKind::Person, &person("B").to_string());
    let flat = b.data["bundle"].to_string();
    let b_id = b.data["id"].as_str().unwrap().to_string();
    let family = json!({
        "union": { "type": "marriage",
                   "persons": [
                       { "person_id": a_id, "role": "spouse" },
                       { "person_id": b_id, "role": "spouse" }
                   ],
                   "confidence": 0.99 },
        "children": []
    });
    let f = axgf_rs::add_entity(&flat, axgf_rs::EntityKind::Family, &family.to_string());
    let flat = f.data["bundle"].to_string();

    let dir = tempdir();
    let bundle_path = dir.join("bundle.json");
    std::fs::write(&bundle_path, &flat).unwrap();

    let (code, stdout, _stderr) = run(
        &[
            "delete",
            "--input",
            bundle_path.to_str().unwrap(),
            "--kind",
            "person",
            "--id",
            &a_id,
        ],
        b"",
    );
    assert_eq!(code, 1);
    let env = parse_envelope(&stdout);
    assert_eq!(
        env.diagnostics[0].code.as_str(),
        "DELETE_BLOCKED_BY_REFERENCE"
    );
}

// -------------------------------------------------------------------------
// export / import round-trip
// -------------------------------------------------------------------------

#[test]
fn export_writes_zip_to_output_path_and_import_round_trips() {
    let flat = axgf_rs::create_bundle(Some("Round")).data.to_string();
    let dir = tempdir();
    let bundle_path = dir.join("bundle.json");
    let zip_path = dir.join("bundle.axgf");
    std::fs::write(&bundle_path, &flat).unwrap();

    let (code, stdout, _stderr) = run(
        &[
            "export",
            "--input",
            bundle_path.to_str().unwrap(),
            "--output",
            zip_path.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert!(env.data["zip_base64"].is_string());
    assert!(zip_path.exists(), "--output should have created a file");
    let zip_bytes = std::fs::read(&zip_path).unwrap();
    assert!(zip_bytes.starts_with(b"PK"), "not a ZIP archive");

    // Import back through the CLI, from stdin.
    let (code, stdout, _stderr) = run(&["import", "--input", "-"], &zip_bytes);
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Ok);
    assert_eq!(env.data["manifest"]["family"]["name"], "Round");
}

// -------------------------------------------------------------------------
// dedup smoke test (contract is exercised in dedicated library tests)
// -------------------------------------------------------------------------

#[test]
fn dedup_of_empty_bundle_is_a_no_op_ok() {
    let flat = axgf_rs::create_bundle(None).data.to_string();
    let (code, stdout, _stderr) = run(&["dedup", "--input", "-"], flat.as_bytes());
    assert_eq!(code, 0);
    let env = parse_envelope(&stdout);
    assert_eq!(env.data["merged_persons"], 0);
    assert_eq!(env.data["merged_families"], 0);
}

// -------------------------------------------------------------------------
// GEDCOM conversion (feature-gated, matches library default)
// -------------------------------------------------------------------------

#[cfg(feature = "gedcom")]
#[test]
fn convert_gedcom_of_minimal_ged_produces_persons() {
    // The tiniest GEDCOM the converter accepts: one individual, one family,
    // wrapped in the mandatory HEAD/TRLR envelope.
    let ged = b"0 HEAD\n\
                1 SOUR test\n\
                1 GEDC\n\
                2 VERS 5.5.1\n\
                2 FORM LINEAGE-LINKED\n\
                1 CHAR UTF-8\n\
                0 @I1@ INDI\n\
                1 NAME Jean /Pierre-Leonard/\n\
                1 SEX M\n\
                0 TRLR\n";
    let (code, stdout, _stderr) = run(&["convert-gedcom", "--input", "-"], ged);
    assert_eq!(code, 0, "convert-gedcom exit code");
    let env = parse_envelope(&stdout);
    assert_eq!(env.status, Status::Ok);
    let persons = env.data["bundle"]["persons"]
        .as_object()
        .expect("persons object");
    assert_eq!(persons.len(), 1);
}

// -------------------------------------------------------------------------
// tiny local tempdir helper
// -------------------------------------------------------------------------

/// Cargo does not ship a tempdir crate to dev-deps and pulling `tempfile`
/// just for the CLI tests is overkill. This helper returns a unique per-test
/// directory under `target/tmp/`, created on demand and left in place for
/// post-mortem inspection (Cargo cleans the target directory on `cargo
/// clean`).
fn tempdir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("axgf-cli-test-{pid}-{n}"));
    std::fs::create_dir_all(&path).expect("mkdir tempdir");
    path
}
