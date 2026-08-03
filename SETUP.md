# axgf-lib - Setup and End-to-End Test Suite

> Ubuntu 24 LTS quick-start and regression test guide.
> The e2e suite lives at `e2e/` in this repository.
> All tests must print `[PASSED]` on a clean build before any release.

---

## 1. Prerequisites

```bash
# Rust stable toolchain (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
rustup --version    # rustup 1.27+
cargo --version     # cargo 1.78+
```

```bash
# System packages
sudo apt-get update
sudo apt-get install -y build-essential git pkg-config
```

---

## 2. Clone and build

```bash
git clone https://github.com/plkarin/axgf-lib.git
cd axgf-lib
cargo build
cargo test                # 82 tests - all must pass
cargo clippy -- -D warnings
cargo doc --no-deps
```

Expected output of `cargo test`:

```
test result: ok. 82 passed; 0 failed; 0 ignored
```

---

## 3. Repository layout

```
axgf-lib/
|- Cargo.toml           library manifest (crate: axgf-rs)
|- LICENSE              Apache-2.0
|- README.md
|- SETUP.md             this file
|- src/
|   |- lib.rs           public API surface
|   |- model/           typed entity structs
|   |- logic/           validate, crud, dedup
|   |- boundary/        envelope, flat bundle, lifecycle
|   |- convert/         GEDCOM 5.5.1 importer
|   `- adapters/        WASM, C-FFI, mobile stubs
`- e2e/
    |- Cargo.toml       standalone test binary
    `- src/
        `- main.rs      63 assertions across T01-T20
```

---

## 4. End-to-end test suite

The suite is a standalone Cargo binary at `e2e/`. It calls the library
exclusively through its public API, exactly as any real client would.
No internals are accessed. Every assertion prints `[PASSED]` or
`[FAILED]`. A failure exits with code 1, making it safe for CI.

### Run

```bash
cd e2e
cargo run
```

### What it covers

| Test | Group | What is verified |
|---|---|---|
| T01 | Lifecycle | `create_bundle` returns ok, manifest.axgf == 1.0, family name stored |
| T02 | Lifecycle | `inspect` returns ok, stats.persons == 0 on empty bundle |
| T03 | Lifecycle | `validate` on empty bundle is ok with zero diagnostics |
| T04 | CRUD build | `add_entity` person returns ok, bundle contains 1 person |
| T05 | CRUD build | Person without id gets auto-generated UUID v4 |
| T06 | CRUD build | Add family with two spouses, union.type == marriage |
| T07 | CRUD build | Add marriage event with participants including the family |
| T08 | CRUD build | Add link (godfather/godchild) between two persons |
| T09 | CRUD build | Add occupation with valid_from / valid_until dates |
| T10 | CRUD build | Add place with coordinates and source with reliability |
| T11 | CRUD modify | `update_entity` patches bio field correctly |
| T12 | CRUD modify | Update with unknown id returns error + ENTITY_NOT_FOUND |
| T13 | CRUD delete | Delete referenced person with Reject policy is blocked |
| T14 | CRUD delete | Delete with Cascade removes person and cleans all references |
| T15 | Validation | Populated bundle passes validation with zero error diagnostics |
| T16 | Validation | Bundle with axgf == 99.0 is rejected as UNSUPPORTED_SPEC_VERSION |
| T17 | Round-trip | flat JSON -> ZIP -> flat JSON preserves person and family counts |
| T18 | Dedup | Two identical-spouse families are merged into one |
| T19 | GEDCOM | Inline GEDCOM converts: 2 persons, 1 family, 1 event, circa flag |
| T19b | GEDCOM | Optional: real-world 767-person file converts and validates |
| T20 | Compat | Unknown field on an entity survives a full round-trip without loss |

### Expected output

```
axgf-rs end-to-end test suite

---- Lifecycle ------------------------------------------------
[PASSED] T01 create_bundle returns ok
[PASSED] T01 manifest.axgf == 1.0
[PASSED] T01 manifest.family.name present
[PASSED] T02 inspect returns ok
[PASSED] T02 stats.persons == 0
[PASSED] T03 validate empty bundle is ok
[PASSED] T03 no diagnostics on empty bundle

---- CRUD - build a family from scratch -----------------------
[PASSED] T04 add person returns ok
[PASSED] T04 bundle contains 1 person
[PASSED] T05 add person without id returns ok
[PASSED] T05 bundle now has 2 persons
[PASSED] T05 auto-generated id is UUID v4
[PASSED] T06 add family returns ok
[PASSED] T06 bundle has 1 family
[PASSED] T07 add marriage event returns ok
[PASSED] T07 bundle has 1 event
[PASSED] T08 add Jules returns ok
[PASSED] T08 add link (godfather) returns ok
[PASSED] T08 bundle has 1 link
[PASSED] T09 add occupation returns ok
[PASSED] T10 add place returns ok
[PASSED] T10 add source returns ok
[PASSED] T10 bundle has 1 place and 1 source

---- CRUD - update and delete ---------------------------------
[PASSED] T11 update_entity returns ok
[PASSED] T11 bio was updated
[PASSED] T12 update non-existent entity returns error
[PASSED] T12 diagnostic code is ENTITY_NOT_FOUND
[PASSED] T13 delete referenced person with Reject blocks
[PASSED] T13 diagnostic is DELETE_BLOCKED_BY_REFERENCE
[PASSED] T14 delete with Cascade returns ok
[PASSED] T14 Jean removed from persons
[PASSED] T14 Jean removed from all family unions under Cascade

---- Validation -----------------------------------------------
[PASSED] T15 validate populated bundle is ok
[PASSED] T15 no error-severity diagnostics
[PASSED] T16 unknown spec version is rejected
[PASSED] T16 diagnostic is UNSUPPORTED_SPEC_VERSION

---- Export / Import round-trip --------------------------------
[PASSED] T17 export_bundle returns ok
[PASSED] T17 ZIP bytes non-empty
[PASSED] T17 import_bundle of exported ZIP returns ok
[PASSED] T17 round-trip persons count matches
[PASSED] T17 round-trip families count matches

---- Deduplication --------------------------------------------
[PASSED] T18 setup: bundle has 2 duplicate families
[PASSED] T18 deduplicate returns ok
[PASSED] T18 duplicate family merged into 1

---- GEDCOM conversion ----------------------------------------
[PASSED] T19 convert_gedcom returns ok
[PASSED] T19 persons == 2
[PASSED] T19 families == 1
[PASSED] T19 events >= 1 (marriage event created)
[PASSED] T19 occupations == 1
[PASSED] T19 places >= 1
[PASSED] T19 ABT date produces circa=true
[PASSED] T19 converted bundle passes validation
[PASSED] T19b real-world tree2-fixed.ged converts ok
[PASSED] T19b real-world: persons >= 760

---- Forward compatibility ------------------------------------
[PASSED] T20 add entity with unknown field returns ok
[PASSED] T20 unknown field preserved after round-trip

==============================================================
 All tests passed - axgf-rs V1 fully operational
==============================================================
```

Any `[FAILED]` line exits with code 1. Safe to call from CI:

```bash
cd e2e && cargo run && echo "regression: ok" || echo "regression: FAILED"
```

---

## 5. T19b - optional real-world GEDCOM test

T19b converts a real GEDCOM file and validates the result. It is skipped
cleanly when no file is provided - it never fails the suite.

To enable it, point the `AXGF_E2E_GEDCOM` environment variable at a `.ged`
file:

```bash
AXGF_E2E_GEDCOM=/path/to/your/tree.ged cargo run
```

To produce a deduplicated GEDCOM from a raw export, use the tool in the
axgf-spec repository:

```bash
python3 tools/gedcom_dedup.py tree.ged -o tree-fixed.ged
```

---

## 6. Regression workflow

Run before every release or after any change to the library:

```bash
# From the repo root
cargo test && cd e2e && cargo run
```

Both must pass. A failure in either blocks release.

---

## 7. Feature flags

```bash
# Default build (includes GEDCOM conversion)
cargo build

# WebAssembly
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --features wasm

# C-ABI shared library
cargo build --features cffi --release
# Output: target/release/libaxgf_rs.so    (Linux)
#         target/release/axgf_rs.dll      (Windows)
#         target/release/libaxgf_rs.dylib (macOS)

# Mobile bindings via UniFFI
cargo build --features mobile
```

---

## 8. Quick reference - public API

| Function | Input | Output |
|---|---|---|
| `create_bundle(family_name)` | optional label | empty flat bundle |
| `import_bundle(zip_bytes)` | `.axgf` ZIP bytes | flat bundle |
| `export_bundle(flat_json)` | flat bundle | ZIP bytes (base64) |
| `inspect(flat_json)` | flat bundle | manifest + stats |
| `validate(flat_json)` | flat bundle | diagnostics |
| `add_entity(flat, kind, entity)` | flat bundle + entity JSON | updated flat bundle |
| `update_entity(flat, kind, entity)` | flat bundle + entity JSON | updated flat bundle |
| `delete_entity(flat, kind, id, policy)` | flat bundle + DeletePolicy | updated flat bundle |
| `deduplicate(flat_json)` | flat bundle | cleaned flat bundle |
| `convert_gedcom(bytes, confidence, lang)` | GEDCOM bytes | flat bundle |

All functions return a uniform `Envelope`:

```json
{
  "status": "ok",
  "data": { "...": "operation result" },
  "diagnostics": [
    {
      "code": "DANGLING_REFERENCE",
      "severity": "warning",
      "message": "...",
      "entity_ref": "<uuid>"
    }
  ]
}
```

Diagnostic codes are a stable public contract and never change meaning
across versions. Messages are human text and may change. An operation
can succeed with warnings - validation is non-blocking.

---

*axgf-lib - SETUP.md - Apache-2.0 - https://github.com/plkarin/axgf-lib*
