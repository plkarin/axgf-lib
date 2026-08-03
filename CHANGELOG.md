# Changelog

All notable changes to `axgf-rs` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`axgf` command-line interface** behind the new `cli` Cargo feature.
  One subcommand per V1 boundary function (`create`, `import`, `export`,
  `inspect`, `validate`, `add`, `update`, `delete`, `dedup`,
  `convert-gedcom`); every subcommand emits the same JSON envelope on
  stdout and returns the operation's status through the process exit code
  (0 = ok, 1 = refused, 2 = `validate` reported error-severity
  diagnostics). Bundle inputs accept `-` to mean stdin so calls compose
  in a shell pipeline. Install with `cargo install axgf-rs --features cli`.
- **Multi-platform binary release automation.** A new
  `.github/workflows/release.yml` triggers on `v*` tag pushes, gates on
  `cargo test --features cli`, then cross-builds `axgf` for
  `x86_64`/`aarch64` Linux (GNU), `x86_64`/`aarch64` macOS, and
  `x86_64-pc-windows-msvc`. Each artifact is packaged as a `tar.gz` (or
  `zip` on Windows) with `README`, `LICENSE`, `NOTICE`, and `CHANGELOG`
  alongside a SHA256 sidecar, and uploaded to the corresponding GitHub
  Release.

### Changed

- **`Cargo.lock` is now tracked** so binary releases are reproducible
  from a given tag. `--locked` is used throughout the release workflow.
  Library consumers on crates.io are unaffected: `cargo add axgf-rs`
  still ignores the checked-in lock file.

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
