# Changelog

All notable changes to `axgf-rs` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] — 2026-10-07

`deduplicate()` merges a couple entered twice when one record leaves the kind
of union unrecorded, and keeps everything either record knew. The boundary is
unchanged. MSRV unchanged at **1.88.0**, verified with `cargo msrv`.

### Changed

- **The embedded 1.1 schema is the frozen AXGF 1.1.0** (axgf-spec `v1.1.0`):
  identical in every definition to the one 0.4.0 carried, with its description
  and `version` no longer saying draft. Bundles written by 0.4.0 stay valid.

### Fixed

- **The `unknown` union type no longer counts as a disagreement.**
  `union.type` is required by the schema, so `"unknown"` is the only way a
  conforming bundle can say the kind of union was not recorded. The ambiguity
  check put it in the set of types like any other value, so a pair with
  `marriage` on one side and `unknown` on the other was refused with
  `MANUAL_REVIEW_REQUIRED`. It is now treated as the absence it is — which the
  check already did for a missing key. Two recorded types that differ are still
  refused.
- **`union` is merged field by field.** It was one top-level key among the
  others, so the keeper's `union` — the lowest UUID's, which says nothing about
  which record is fuller — was kept whole and the victim's discarded. The
  keeper now gains every union field it leaves unrecorded (absent, `null`,
  empty or `unknown`), `start` and `end` are merged the same way one level
  down, `persons` are unioned on `person_id`, and a value the keeper did record
  is never overwritten.

The two ship together deliberately: the first alone turns a reported duplicate
into a merge that silently drops the marriage's date, place and event whenever
the thin record has the lower UUID. On the bundle this was found on, 0.4.0
merged none of its two remaining duplicate couples and refused both; 0.5.0
merges both, and the surviving record of each keeps its date.

## [0.4.0] — 2026-09-15

AXGF 1.1: the extended person profile. The library reads, validates, writes
and exports 1.1 bundles as it does 1.0 ones, and says in its own terms what is
wrong with the new data. The boundary is unchanged — every function takes and
returns what it did — and a bundle with no 1.1 content behaves exactly as it
did in 0.3.0, down to the schema file in its archive. MSRV unchanged at
**1.88.0**.

### Added

- **Both spec versions.** `SUPPORTED_SPEC_VERSIONS` is `["1.0", "1.1"]`, and
  `LATEST_SPEC_VERSION` is `"1.1"`. The 1.1 schema is vendored beside the 1.0
  one (`schema/axgf-1.1.schema.json`, `EMBEDDED_SCHEMA_1_1`), and
  `boundary::lifecycle::schema_for` returns the schema and archive path for a
  version. `validate`, `add_entity` and `update_entity` check against the
  schema the manifest declares; `export_bundle` and `export_bundle_streaming`
  write that schema into the archive.
- **A bundle becomes 1.1 when it gets 1.1 content.** Every CRUD write and both
  exports raise `manifest.axgf` to cover what the bundle holds — the newest
  version any entity declares or any entity's content needs — in the same step
  that refreshes `stats`. It is never lowered. `create_bundle` still stamps
  `"1.0"` (`CURRENT_SPEC_VERSION`): a bundle with nothing 1.1 in it is a 1.0
  bundle, and stamping it 1.1 would make every 1.0 reader refuse a file it
  could read. `add_entity` fills a missing `axgf_version` with the oldest
  version the entity's content fits, for the same reason.
- **`model::profile`** — the 1.1 addition as data, not behaviour:
  - `claim::Claim<T>`, with `ClaimBound` and `Coordinates` aliased to the 1.0
    shapes they are (`LinkValidity`, `place::Coordinates`) and `ArtefactRef`
    for Document references;
  - `vocab` — all 102 closed vocabularies as `Vocabulary` constants, the
    analytes each laboratory panel may contain, and the six national rank
    vocabularies with each rank's own title and category;
  - `registry` — all 132 attributes in their fourteen groups: path, single or
    series, value `Shape`, and `SensitiveClass`, with lookups by path, group
    and class;
  - `types` — typed structs for the twelve new Person blocks. Numbers are
    `serde_json::Number`, so `158` round-trips as `158`.
- **1.1 fields on the 1.0 model**: `Identity::{titles, sex_at_birth,
  gender_identity, class_visibility}`, the new `Vital` attributes (time,
  coordinates, and on death causes, contributing factors, autopsy,
  disposition, grave), the twelve blocks on `Person`, `FamilyChild::lineage`,
  `Link::relation`, `Occupation::position` and
  `Privacy::withheld_classes`.
- **Four diagnostic codes**, all non-blocking:
  - `OUT_OF_VOCABULARY` (warning) — a value that is not a term of its
    vocabulary, naming the attribute path, the value and the vocabulary. Also
    a rank outside the list its country selects, a rank for a country 1.1
    registers no list for, an analyte outside its panel, an artefact type its
    attribute does not allow. The schema's own enumeration failure for the
    same value is not reported a second time.
  - `CLAIM_INCONSISTENT` (warning) — SPEC_1.1 §7.3's rules a schema cannot
    express: a haplogroup subclade outside its major clade, a rank whose
    category is not its own, an epigenetic clock with the wrong kind of
    result, a period that ends before it starts, two causes of death with one
    sequence number.
  - `SPEC_VERSION_MISMATCH` (warning) — an entity with 1.1 attributes that
    declares 1.0, one newer than its manifest, or a 1.0 manifest carrying
    `withheld_classes`.
  - `UNKNOWN_ATTRIBUTE` (info) — a key inside a 1.1 block that 1.1 does not
    define. Preserved, as 1.0 P9 requires; reported, because a misspelt
    attribute is otherwise invisible.

  The 1.1 checks run on content rather than on the declared version, so a
  bundle that forgot to say 1.1 is still checked and the forgetting is
  reported.

### Changed

- **`Family::union` is `Option<Union>`.** The specification made a union
  optional for a sibling group whose parents are unknown (1.0 §4.2.3) in
  0.2.0; the typed model still required one and refused the shape. The only
  change in this release that can break a caller, and only one that reads
  `Family` through the typed model.
- **Compiled schemas are cached per process.** `add_entity` and
  `update_entity` compiled the whole schema on every call, which with 1.1's
  vocabularies cost more than the write. The embedded schemas are constants,
  so compiling one is a pure function and is done once per version and kind.
- `scripts/sync-schema.sh` and `.github/workflows/schema-drift.yml` sync and
  compare both schema files.

### Tests

63 new: `tests/profile_registry.rs` holds the registry against the embedded
schema in both directions — attributes, cardinalities, classes, every term of
every vocabulary, artefact restrictions, the rank rules;
`tests/profile_roundtrip.rs` round-trips every group through the typed model,
with every attribute and field filled from the registry, plus the
specification's worked example and a bundle through ZIP and back;
`tests/profile_validation.rs` puts a term and a non-term into every
vocabulary slot of every group — the non-term must draw exactly one
`OUT_OF_VOCABULARY` naming its vocabulary — and covers the semantic rules and
the version rules. Mutation-checked: silencing the vocabulary check fails 16
of them, and letting the schema's enumeration failure through as well fails
15.

[0.5.0]: https://github.com/plkarin/axgf-lib/compare/v0.4.0...main
[0.4.0]: https://github.com/plkarin/axgf-lib/releases/tag/v0.4.0

## [0.3.0] — 2026-08-17

Streaming payload access, for bundles whose media does not fit in memory.
Purely additive: every existing function behaves exactly as it did, the MSRV
(**1.88.0**) is unchanged, and a bundle that never streams is byte-identical
to what 0.2.0 produced. The minor bump signals the new capability rather
than any break.

### Added

- **`import_bundle_textual(src)`** — the manifest and all eight entity
  collections, document metadata intact, with **no payload decoded**. Each
  payload is recorded by path, size and CRC-32 in the new
  `external_payloads` field, all of which the ZIP central directory already
  carried.
- **`import_bundle_streaming(src, on_payload)`** — the same textual result,
  plus one callback per payload, in archive order. The callback receives a
  `Payload`: a live `Read` over that entry exposing `path()`, `size()` and
  `crc32()` before anything is decompressed, with `copy_to` for a bounded
  move and `read_to_end` when the whole thing is wanted.
- **`export_bundle_streaming(flat_json, dest, supply)`** — writes the archive
  into a caller-provided `Write + Seek`, asking `supply` for one payload at a
  time via a `PayloadSlot`. Because the output goes to `dest` rather than
  into a base64 string, the finished bundle never exists in memory either.
  Bundles still carrying inline `attachments` export correctly too, so a
  caller can convert incrementally.
- **`FlatBundle::external_payloads`** — the marker that says "this bundle's
  payloads live elsewhere". Skipped when empty, so it is invisible to
  everyone else.
- **Three diagnostic codes**: `PAYLOADS_EXTERNAL`, `PAYLOAD_SOURCE_FAILED`,
  `PAYLOAD_SINK_FAILED`.

Measured on 32 payloads totalling 70 MiB, largest 8 MiB — peak heap:

| operation | streaming | non-streaming |
|---|---|---|
| import | 0.21 MiB (`copy_to`) / 8.15 MiB (`read_to_end`) | 186.80 MiB |
| export | 0.53 MiB | 355.13 MiB |

Peak is bounded by the largest single payload, or by a 64 KiB copy buffer if
the caller never holds one whole — never by their sum.

### Changed

- **`export_bundle` now refuses a bundle that declares `external_payloads`**,
  with a `PAYLOADS_EXTERNAL` error naming `export_bundle_streaming`. It can
  only write payloads it can see, and a bundle whose payloads were streamed
  out carries none; writing it would produce a structurally valid archive
  with every photograph silently missing. No bundle produced by 0.2.0 can
  carry the marker, so no existing caller is affected. The marker survives
  CRUD, so editing entities in between cannot launder the refusal away.

### Notes

- No async, no filesystem access, no hidden state: the streaming functions
  take byte streams the caller owns and retain nothing between calls. WASM
  remains viable.
- Streaming is never the default. `import_bundle` and `export_bundle` are
  untouched and remain the simple path.

## [0.2.0] — 2026-08-03

The headline change is a full command-line interface plus the release
automation that ships it as pre-built binaries. The library surface,
diagnostic vocabulary, and MSRV (**1.88.0**) are unchanged from 0.1.0 —
existing library consumers upgrade transparently.

### Added

- **`axgf` command-line interface**, on by default so plain
  `cargo install axgf-rs` produces the binary. One subcommand per V1
  boundary function:
  - `create` — new empty bundle. Requires `-o/--output`.
  - `import` — decode a `.axgf` archive and print a summary
    (read-only).
  - `export` — rebuild a bundle. `-o` chooses `.axgf` (ZIP) or `.json`
    (flat) by extension.
  - `inspect` — manifest + freshly computed stats (read-only).
  - `validate` — structural + semantic report (read-only, exit code `2`
    on error-severity diagnostics so CI can gate on it without
    misinterpreting an invalid bundle as a broken pipeline).
  - `add`, `update`, `delete` — CRUD, taking `<KIND> <PATH>` as
    positionals. `--policy` picks `reject|cascade|orphan` for the
    delete's referential integrity.
  - `dedup` — safe merges + `MANUAL_REVIEW_REQUIRED` flags.
  - `convert-gedcom` — GEDCOM 5.5.1 → AXGF, with `--confidence` and
    `--place-lang` controls. Requires `-o`.
- **Human-first output.** Default: concise summary on stdout, grouped
  diagnostic counts on stderr, error diagnostics on stderr as
  `CODE: message`. `--json`: raw envelope on stdout, nothing else,
  pipes cleanly through `jq`. `-q/--quiet`: no stdout; result in the
  exit code.
- **Positional input paths.** Every command that reads a bundle takes
  it as a positional `PATH`; `--input <PATH>` is kept as a
  back-compatible alias for this unreleased 0.2.0. `-` reads bytes
  from stdin.
- **`-o/--output <PATH>` on every mutating command** (`create`,
  `convert-gedcom`, `add`, `update`, `delete`, `dedup`, `export`).
  The output form is chosen by extension: `.axgf` writes the ZIP,
  anything else writes flat JSON. When `-o` is omitted on a command
  that read a file, the input is edited *in place* — atomically, via
  a sibling tempfile + rename, so a mid-write failure never leaves a
  truncated bundle behind.
- **Transparent `.axgf` input.** Any command that reads a bundle now
  accepts a `.axgf` archive directly (imported on the fly) or a
  `.json` flat bundle, driven by extension. This is what makes
  `axgf validate family.axgf` and `axgf add person family.axgf …`
  work without a manual import step.
- **Multi-platform binary release automation.**
  `.github/workflows/release.yml` triggers on `v*` tag pushes, gates on
  `cargo test`, and cross-builds `axgf` for six targets:
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

### Fixed

- **Nameless persons no longer emit an empty `display` string** that
  fails `axgf_name.display` (`minLength: 1`). GEDCOM `INDI` records
  without a usable `NAME` now render as `"display": "[Unknown]"` per
  spec §4.1.1; `components` stays empty rather than fabricating a
  synthetic name component.
- **Bare `1 OCCU` tags with no value are skipped** instead of emitting
  an occupation with `"title": ""` (fails `occupation.title` minLength
  1). A `GEDCOM_UNRECOGNIZED_TAG` warning names the source `INDI`
  xref so the omission is visible rather than silent.
- **Marriage events without a `DATE` no longer emit `date.value: ""`**
  (which fails the `iso8601` string pattern). The MARR event still
  carries `date` — required by the schema — but with only
  `precision: "unknown"`; the `value` key is omitted entirely, the
  same treatment already applied to unparseable dates in 0.2.0.
- **Empty `FAM` records** (no `HUSB`/`WIFE`/`CHIL`, seen occasionally
  as leftover stubs after deletions in real exports) are dropped with
  a `GEDCOM_UNRECOGNIZED_TAG` warning naming the xref, instead of
  producing a family entity that fails validation on both counts.
- Cumulative effect measured on a real-world 767-person webtrees
  export (`tests/fixtures/tree.ged`): 44 → 0 `SCHEMA_VALIDATION_FAILED`
  warnings. A new regression test,
  `converted_real_world_fixture_has_zero_schema_warnings` in
  `tests/gedcom_convert.rs`, guards this end-to-end so the class of
  bug can't return unnoticed.

### Changed

- **`family.union` is now individually optional.** A family with only
  `children` — a *sibling group* whose parents are unknown — is a
  real and common genealogical situation (GEDCOM `FAM` records with
  only `CHIL` entries), previously forced into an invalid
  "marriage of zero persons" by the schema requiring `union`. The
  spec (axgf-spec 1.0, §4.2.3) now requires **at least one** of
  `union` or `children`; an entirely empty family remains invalid.
  The GEDCOM converter emits sibling groups with no `union` block,
  and `docs/API.md → update_entity → Demo E` documents the
  read-modify-write recovery pattern for adding parents later
  without dropping the children (covered by
  `tests/crud.rs::family_gains_union_later_without_losing_children`).
- **`cli` is now a default Cargo feature.** Plain `cargo install
  axgf-rs` produces the `axgf` binary; library consumers who want to
  avoid the `clap` dependency opt out with
  `default-features = false, features = ["gedcom"]`.
- **`--family-name` renamed to `--name` on `create`.** The old spelling
  is kept as an alias.
- **`--entity` renamed to `--data` on `add`/`update`.** The old
  spelling is kept as an alias.
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
