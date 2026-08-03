<div align="center">

# axgf-lib

**Official reference library for the [Axiom Genealogy Format (AXGF)](https://github.com/plkarin/axgf-spec)**

[![Crates.io](https://img.shields.io/crates/v/axgf-rs.svg?style=flat-square)](https://crates.io/crates/axgf-rs)
[![Docs.rs](https://img.shields.io/docsrs/axgf-rs?style=flat-square)](https://docs.rs/axgf-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0-43d9a2?style=flat-square)](./LICENSE)
[![Spec](https://img.shields.io/badge/spec-AXGF_1.0-764ba2?style=flat-square)](https://github.com/plkarin/axgf-spec)
[![Status](https://img.shields.io/badge/status-alpha-ffd93d?style=flat-square)](https://github.com/plkarin/axgf-lib/issues)

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
axgf-rs = "0.1"
```

Optional adapters are gated behind Cargo features (see the [Platform bindings](#platform-bindings) section for the full table):

```toml
[dependencies]
axgf-rs = { version = "0.1", features = ["wasm"] }
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

Deliberately **not** in V1: graph traversal, a query engine, sessions, disk access, and rendering. Those belong to the client, or to a later version.

---

## Quick start

```rust
use axgf_rs::{
    add_entity, create_bundle, export_bundle, import_bundle, validate,
    logic::crud::EntityKind,
};

// 1. Start an empty bundle.
let empty = create_bundle(Some("Famille Pierre-Léonard"));
let mut flat = serde_json::to_string(&empty.data).unwrap();

// 2. Add a person.
let entity = r#"{
  "identity": {
    "name": {"display": "Jean Pierre-Léonard", "components": []},
    "gender": {"value": "M"},
    "is_living": false
  }
}"#;
let added = add_entity(&flat, EntityKind::Person, entity);
flat = serde_json::to_string(&added.data["bundle"]).unwrap();

// 3. Validate — non-blocking; warnings and errors both surface in diagnostics.
let report = validate(&flat);
for d in &report.diagnostics {
    eprintln!("{} {}: {}", d.code.as_str(), format!("{:?}", d.severity), d.message);
}

// 4. Export to .axgf bytes (base64 inside the envelope's data).
let exp = export_bundle(&flat);
let zip_b64 = exp.data["zip_base64"].as_str().unwrap();

// 5. Round-trip: import the same bytes back into a flat bundle.
let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, zip_b64).unwrap();
let imp = import_bundle(&bytes);
assert_eq!(imp.status.to_string(), "ok");  // pseudo — see the API docs
```

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
axgf-rs = { version = "0.1", features = ["wasm"] }
```

---

## Status

**Alpha.** The API surface and the design contract are settled; implementation is in progress. Expect breaking changes before `1.0.0`. Track progress and open questions in [Issues](https://github.com/plkarin/axgf-lib/issues).

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
