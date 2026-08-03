# Changelog

All notable changes to `axgf-rs` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-08-03

The headline change is a full command-line interface plus the release
automation that ships it as pre-built binaries. The library surface,
diagnostic vocabulary, and MSRV (**1.88.0**) are unchanged from 0.1.0 —
existing library consumers upgrade transparently.

### Added

- **`axgf` command-line interface** behind the new `cli` Cargo feature.
  One subcommand per V1 boundary function:
  - `create` — new empty bundle stamped with the current spec version.
  - `import` — decode a `.axgf` archive into flat JSON.
  - `export` — encode flat JSON back into a `.axgf` archive
    (optionally writing the ZIP bytes to `--output`).
  - `inspect` — manifest + freshly computed stats.
  - `validate` — structural + semantic report (exit code `2` on
    error-severity diagnostics, so CI can gate on it without
    misinterpreting an invalid bundle as a broken pipeline).
  - `add`, `update`, `delete` — CRUD; `--policy` picks
    `reject|cascade|orphan` for the delete's referential integrity.
  - `dedup` — safe merges + `MANUAL_REVIEW_REQUIRED` flags.
  - `convert-gedcom` — GEDCOM 5.5.1 → flat AXGF, with
    `--confidence` and `--place-lang` controls.
  Every subcommand emits the same JSON envelope on stdout; every file
  argument accepts `-` for stdin so calls compose through `jq` in a shell
  pipeline. Install with `cargo install axgf-rs --features cli`, or grab
  a pre-built binary from GitHub Releases.
- **Multi-platform binary release automation.**
  `.github/workflows/release.yml` triggers on `v*` tag pushes, gates on
  `cargo test --features cli`, and cross-builds `axgf` for six targets:
  `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` (statically
  linked, runs on every Linux distribution regardless of libc),
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`. Each artifact is
  packaged as a `tar.gz` (or `zip` on Windows) with `README`, `LICENSE`,
  `NOTICE`, and `CHANGELOG` alongside a SHA256 sidecar, and attached to a
  fresh GitHub Release via `gh release create --generate-notes`.
- **`docs/CLI.md`** — end-to-end CLI reference: installation (precompiled
  binary, `cargo install`, from-source), a 60-second quickstart, one
  section per subcommand with real captured output, entity-kind
  vocabulary, and scripting patterns (`jq` pipelines + CI gates).

### Changed

- **`Cargo.lock` is now tracked** so binary releases are reproducible
  from a given tag. `--locked` is used throughout the release workflow.
  Library consumers pulling from crates.io are unaffected: `cargo
  publish` excludes the lock file and downstream builds resolve their
  own versions.

### MSRV

- Unchanged at **1.88.0**. The `cli` feature adds `clap 4.5` (which
  itself supports back to Rust 1.74), so the crate floor is set by
  `time` 0.3.55, not `clap`.

[0.2.0]: https://github.com/plkarin/axgf-lib/releases/tag/v0.2.0

## [0.1.0] — 2026-08-02

First public release on crates.io. This is a **0.x release**: the API is
functionally complete for the V1 design contract but may change in incompatible
ways before `1.0.0`. Track breaking changes here and in the git history.

### Added

- **Ten public functions on the stateless JSON boundary.**
  - `create_bundle` — new empty bundle stamped with the current spec version.
  - `import_bundle` — decode a `.axgf` ZIP archive into a flat bundle.
  - `export_bundle` — encode a flat bundle back to a `.axgf` ZIP archive
    with recomputed stats and the embedded canonical schema.
  - `inspect` — read the manifest and computed stats without mutation.
  - `validate` — structural (JSON Schema) and semantic checks
    (dangling refs, cycles, chronology, duplicate unique refs).
  - `add_entity`, `update_entity`, `delete_entity` — CRUD with a
    caller-provided `DeletePolicy` for referential integrity.
  - `deduplicate` — safe merges only; ambiguous cases are flagged with
    `MANUAL_REVIEW_REQUIRED` diagnostics rather than performed.
  - `convert_gedcom` (feature `gedcom`, default on) — convert
    GEDCOM 5.5.1 to a flat AXGF bundle.
- **Uniform `Envelope` response** with `status`, `data`, and stable
  `diagnostic` codes (`UNSUPPORTED_SPEC_VERSION`, `INVALID_JSON`,
  `DANGLING_REFERENCE`, `CYCLE_DETECTED`, `MANUAL_REVIEW_REQUIRED`, …).
  Validation is non-blocking: operations may return `Ok` alongside
  warnings.
- **Explicit spec-version gating** against `SUPPORTED_SPEC_VERSIONS`
  in every lifecycle operation.
- **Forward compatibility**: unknown fields on entities survive a
  round-trip untouched.
- **Adapter scaffolding** behind Cargo features for WebAssembly (`wasm`),
  the C ABI (`cffi`), and mobile via UniFFI (`mobile`). The Rust adapter
  is on by default.
- **Test suite**: 82 unit and integration tests plus 1 crate-level
  runnable doc example. The `e2e/` directory ships a separate binary
  driving a 63-assertion end-to-end suite; it is not part of the
  published tarball.
- **Vendored JSON schema** (`schema/axgf-1.0.schema.json`) embedded at
  compile time via `include_str!` so validation works offline and in
  WASM. Drift against `axgf-spec` main is guarded by
  `.github/workflows/schema-drift.yml`; `scripts/sync-schema.sh` is the
  only supported way to refresh the vendored copy.

### MSRV

- Minimum Supported Rust Version: **1.88.0**. Transitive dependencies
  (`time-core`, `idna_adapter`) require Rust 2024 edition support, which
  stabilised in 1.85, and `time` 0.3.55 raises the effective floor to
  1.88. The MSRV was measured with `cargo msrv find --all-features`.

### Known limitations

- No disk I/O, no query engine, no graph traversal, no rendering — by
  design (see the design contract in the README).
- Only GEDCOM 5.5.1 is supported for conversion; other genealogy formats
  are out of scope for V1.
- The `wasm`, `cffi`, and `mobile` adapters are scaffolded; a real
  target integration (npm package, `.dylib`/`.so` layout, UniFFI
  bindings publication) is out of scope for this crate release.

[0.1.0]: https://github.com/plkarin/axgf-lib/releases/tag/v0.1.0
