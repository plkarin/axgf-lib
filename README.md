<div align="center">

# axgf-lib

**Official reference library for the [Axiom Genealogy Format (AXGF)](https://github.com/plkarin/axgf-spec)**

[![Crates.io](https://img.shields.io/crates/v/axgf-rs.svg?style=flat-square)](https://crates.io/crates/axgf-rs)
[![Docs.rs](https://img.shields.io/docsrs/axgf-rs?style=flat-square)](https://docs.rs/axgf-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0-43d9a2?style=flat-square)](./LICENSE)
[![Spec](https://img.shields.io/badge/spec-AXGF_1.0-764ba2?style=flat-square)](https://github.com/plkarin/axgf-spec)
[![Status](https://img.shields.io/badge/status-published_%C2%B7_pre--1.0-43d9a2?style=flat-square)](https://github.com/plkarin/axgf-lib/issues)

*One core. Every platform. The single point of contact for reading, writing, validating, and converting AXGF bundles — so no application ever re-implements the format.*

[Specification →](https://github.com/plkarin/axgf-spec) · [API contract →](#api-contract) · [Bindings →](#platform-bindings) · [Issues →](https://github.com/plkarin/axgf-lib/issues)

</div>

---

## What this is

`axgf-lib` is the reference implementation of the AXGF standard, written in Rust and compiled to run everywhere: in the browser (WebAssembly), on desktop clients (native library / Tauri), on mobile (Android & iOS via UniFFI), and from any language over a C ABI.

Applications — SaaS backends, desktop apps, CLIs — call this library to manipulate genealogy data. They never parse, validate, or merge AXGF themselves. Axiom provides the specification and this library; clients build their own products on top.

The crate is published as **`axgf-rs`**.

```
┌──────────────────────────────────────────────────────────┐
│  Your application (web SaaS · desktop · mobile · CLI)     │
│  renders, persists, and exposes genealogy data           │
└───────────────────────────┬──────────────────────────────┘
                            │ calls
┌───────────────────────────▼──────────────────────────────┐
│  axgf-lib  (crate: axgf-rs)                               │
│  create · import · export · validate · CRUD · convert     │
│  one stateless core → WASM · native · mobile · C-FFI      │
└──────────────────────────────────────────────────────────┘
```

---

## Installation

Published on crates.io as **`axgf-rs`**. Documentation is on [docs.rs](https://docs.rs/axgf-rs).

```sh
cargo add axgf-rs
```

Or add it manually to `Cargo.toml`:

```toml
[dependencies]
axgf-rs = "0.3"
```

Optional adapters are gated behind Cargo features (see the [Platform bindings](#platform-bindings) section for the full table):

```toml
[dependencies]
axgf-rs = { version = "0.3", features = ["wasm"] }
```

The pre-1.0 version signals that the public API may still change. The AXGF **format** version and the crate **version** are independent — this crate targets AXGF 1.0.

### Command-line binary

The same core is shipped as a standalone `axgf` executable. Because the
`cli` feature is on by default, plain `cargo install axgf-rs` produces
the binary. See the **[Command line](#command-line)** section below for
the fast path.

```sh
cargo install axgf-rs
```

Pre-built binaries for Linux (musl static + glibc), macOS, and Windows are
attached to each tagged release on GitHub. Library-only consumers who
want to avoid the `clap` dependency can opt out with
`default-features = false, features = ["gedcom"]`.

---

## Command line

The `axgf` binary is the fastest way to evaluate the project. Every V1
boundary function is a subcommand; each prints a concise human summary
by default. Pass `--json` to receive the raw JSON envelope for piping
through `jq`.

```console
$ axgf convert-gedcom tests/fixtures/small.ged -o /tmp/t.axgf
converted small.ged
  persons       3
  families      1
  events        1
  links         0
  occupations   1
  sources       1
  places        2
  documents     2
wrote t.axgf (8 KiB)

$ axgf validate /tmp/t.axgf
validated t.axgf
  errors                     0
  warnings                   3
  SCHEMA_VALIDATION_FAILED   3

$ axgf inspect /tmp/t.axgf
t.axgf
  axgf          1.0
  persons       3
  families      1
  events        1
  links         0
  occupations   1
  sources       1
  places        2
  documents     2
```

Read-only commands (`inspect`, `validate`, `import`) never touch the
input file. Mutating commands (`create`, `convert-gedcom`, `add`,
`update`, `delete`, `dedup`, `export`) take `-o/--output` and edit their
input in place when it is omitted — atomically, so a mid-write failure
never leaves you with a truncated bundle.

The exit code is `0` on success, `1` when the operation is refused, and
`2` when `axgf validate` reports at least one error-severity diagnostic
(warnings do not count). See **[`docs/CLI.md`](docs/CLI.md)** for the
full reference: every subcommand, every flag, real captured output,
scripting patterns, and installation from precompiled binaries.

---

## Design contract

The library's behavior is fixed by five rules. They exist so that a single core can serve every platform without duplicating logic, and so that clients get identical, predictable behavior everywhere.

**Stateless and immutable.** Every operation takes a bundle in and returns a new bundle out. The library keeps nothing between calls. No sessions, no handles, no hidden mutation. This makes behavior reproducible and bindings trivial.

**Data-oriented boundary.** Callers exchange JSON and receive a uniform envelope. No native Rust objects cross the boundary, so bindings to JavaScript, Kotlin, Swift, or C stay mechanical. The caller manipulates plain data it already understands, in its own language.

**Flat JSON is the working form.** The on-disk `.axgf` is a ZIP archive (one file per entity, plus embedded documents). The library converts it to a single flat JSON object for editing. The ZIP is produced only at export and parsed only at import — every operation in between is a fast JSON manipulation.

**No disk, no graph traversal, no rendering.** The library produces and validates correct bundles. Reading rich views, walking the family graph, persisting to a database, and rendering to HTML are the client's responsibility, not the library's.

**Explicit spec-version gating.** Every operation checks the bundle's declared AXGF version and refuses unknown versions with a stable diagnostic rather than misbehaving. A library built for AXGF 1.0 will never silently corrupt a 2.0 bundle.

---

## API contract

Every function returns the same **envelope** shape, so success, produced data, and diagnostics are inspected identically in any language:

```json
{
  "status": "ok",
  "data": { "...": "the flat bundle, a manifest summary, or ZIP bytes" },
  "diagnostics": [
    { "code": "DANGLING_REFERENCE", "severity": "warning",
      "message": "...", "entity_ref": "<uuid>" }
  ]
}
```

Diagnostic **codes** are part of the public contract and never change meaning across versions; human-readable messages may. An operation can succeed *with warnings* — validation is non-blocking, mirroring the format's own confidence model.

### Operations (V1)

| Group | Function | Purpose |
|---|---|---|
| **Lifecycle** | `create_bundle` | New empty bundle stamped with the current spec version |
| | `import_bundle` | `.axgf` ZIP bytes → flat JSON (the only ZIP reader) |
| | `export_bundle` | flat JSON → `.axgf` ZIP bytes (the only ZIP writer) |
| | `inspect` | Read manifest + stats without materializing every entity |
| **Validation** | `validate` | JSON Schema + semantic checks (dangling refs, cycles, chronology) |
| **CRUD** | `add_entity` | Insert a person/family/event/link/source/place/document |
| | `update_entity` | Replace an entity by id |
| | `delete_entity` | Remove by id under an explicit referential-integrity policy |
| **Cleanup** | `deduplicate` | Safely merge duplicates; flag ambiguous cases for review |
| **Conversion** | `convert_gedcom` | GEDCOM 5.5.1 bytes → flat AXGF bundle |

### Streaming payloads (0.3.0)

`import_bundle` decodes every binary payload to base64 inside the flat JSON and `export_bundle` needs them all present to write, so a media-heavy bundle costs several times its own size to open. These three take the payloads one at a time instead:

| Group | Function | Purpose |
|---|---|---|
| **Streaming** | `import_bundle_textual` | Manifest + all eight entity collections, document *metadata* included, with **no payload decoded** — each is recorded by path, size and CRC-32 in `external_payloads` |
| | `import_bundle_streaming` | The same textual result, plus one callback per payload: a live reader over that ZIP entry, opened and forgotten before the next |
| | `export_bundle_streaming` | Writes the archive into a caller-supplied `Write + Seek`, asking for one payload at a time — so the finished bundle never exists in memory either |

They exist because peak memory is then bounded by the largest single payload — or by a 64 KiB copy buffer if the caller never holds one whole — rather than by their sum: a real 417 MB bundle went from **1.53 GB to 29 MB** at axgf-cms startup.

Streaming is never the default: `import_bundle` and `export_bundle` are unchanged, and a caller who wants the whole bundle as one JSON value still gets exactly that. A streamed import marks the bundle's `external_payloads`, and `export_bundle` refuses such a bundle with `PAYLOADS_EXTERNAL` rather than writing an archive with the media silently missing.

Deliberately **not** in V1: graph traversal, a query engine, sessions, disk access, and rendering. Those belong to the client, or to a later version. Disk access stays out under streaming too — the library never opens a path; the caller supplies the reader and the writer it already owns, and the library only reads and writes through them.

---

## Quick start

Create a bundle, add a person, validate it, write the archive, read it back.
Compiled and run against `axgf-rs` 0.3.0 exactly as printed:

```rust
use std::io::Cursor;

use axgf_rs::boundary::envelope::Status;
use axgf_rs::{
    add_entity, create_bundle, export_bundle_streaming, import_bundle, validate, EntityKind,
};

fn main() {
    // 1. Start an empty bundle. Every call takes JSON in and hands back an
    //    envelope whose `data` is JSON out.
    let created = create_bundle(Some("Famille Pierre-Léonard"));
    assert_eq!(created.status, Status::Ok);
    let mut flat = created.data.to_string();

    // 2. Add a person. The library stamps `id`, `type` and `axgf_version`;
    //    the updated bundle comes back under `data["bundle"]`.
    let person = r#"{
      "identity": {
        "name": {"display": "Jean Pierre-Léonard", "components": []},
        "gender": {"value": "M"},
        "is_living": false
      }
    }"#;
    let added = add_entity(&flat, EntityKind::Person, person);
    assert_eq!(added.status, Status::Ok);
    flat = added.data["bundle"].to_string();

    // 3. Validate. Non-blocking: warnings surface as diagnostics while the
    //    status stays `Ok`.
    let report = validate(&flat);
    for d in &report.diagnostics {
        println!("{} {:?}: {}", d.code.as_str(), d.severity, d.message);
    }
    assert_eq!(report.status, Status::Ok);

    // 4. Write the `.axgf` archive. `export_bundle_streaming` writes into any
    //    `Write + Seek` — an in-memory buffer here, a `File` in an
    //    application — and asks the closure for one payload at a time. This
    //    bundle carries no documents, so it is never called.
    let mut archive = Cursor::new(Vec::new());
    let exported = export_bundle_streaming(&flat, &mut archive, |_slot| Ok(()));
    assert_eq!(exported.status, Status::Ok);
    println!("wrote {} bytes", exported.data["size_bytes"]);

    // 5. Round-trip: read the bytes back into the same flat working form.
    let imported = import_bundle(archive.get_ref());
    assert_eq!(imported.status, Status::Ok);
    println!("persons: {}", imported.data["persons"].as_object().unwrap().len());
}
```

```console
$ cargo run
wrote 5137 bytes        # varies by a byte or two — the manifest is timestamped
persons: 1
```

The only dependencies are `axgf-rs` and `serde_json`. Step 4 uses the streaming
writer because it needs no base64 decode on the way back; `export_bundle`
returns the same archive as base64 inside the envelope when a single value is
what you want.

See [`docs/API.md`](./docs/API.md) for the full function-by-function surface.

---

## Platform bindings

The same core is exposed to every target through thin, logic-free adapters, selected by Cargo feature:

| Target | Feature | Mechanism |
|---|---|---|
| Rust | *(default)* | native crate |
| Web / Node / Tauri webview | `wasm` | WebAssembly via `wasm-bindgen` |
| Desktop / other languages | `cffi` | C ABI |
| Android / iOS | `mobile` | Kotlin & Swift via UniFFI |

```toml
[dependencies]
axgf-rs = { version = "0.3", features = ["wasm"] }
```

---

## Status

**Pre-1.0, in production.** The V1 surface described above is complete and published on crates.io as `axgf-rs` 0.3.0, and [axgf-cms](https://github.com/plkarin/axgf-cms) runs on it: every bundle it serves is created, validated, imported and exported through this library. The API and the design contract are settled, and the diagnostic codes are a stable contract.

The version stays below `1.0.0` because the Rust signatures may still change — 0.3.0 added functions without breaking any, but a future minor release may break one. Pin a minor version if that matters to you. The AXGF **format** version is independent of the crate version; this crate targets AXGF 1.0. Open questions and planned work are in [Issues](https://github.com/plkarin/axgf-lib/issues).

---

## Relationship to the specification

This library implements the format defined in **[plkarin/axgf-spec](https://github.com/plkarin/axgf-spec)**. The specification and its JSON Schema are the authority; where this library and the spec disagree, the spec wins and the library is the bug.

- **Specification** — the AXGF standard (CC0, public domain)
- **This library** — the reference implementation (Apache-2.0)

---

## License

Licensed under the **Apache License, Version 2.0**. See [LICENSE](./LICENSE).

Apache-2.0 is chosen over MIT for its explicit patent grant, which protects adopters — important for a component meant to be embedded widely, including in commercial software. The library stays free and open in perpetuity while remaining usable by everyone, which is what a standard needs to spread.

```
SPDX-License-Identifier: Apache-2.0
```

---

<div align="center">

**axgf-lib** · reference implementation of the Axiom Genealogy Format
*crate: `axgf-rs` · spec: [plkarin/axgf-spec](https://github.com/plkarin/axgf-spec)*

</div>
