<!-- SPDX-License-Identifier: Apache-2.0 -->
# `axgf` — command-line reference

`axgf` is the standalone command-line entry point to the `axgf-rs` library.
Every V1 boundary function is exposed as one subcommand; every subcommand
prints the same JSON [`Envelope`](API.md) on stdout so calls compose in a
shell pipeline through `jq`.

**Exit codes.** They are the machine-readable answer:

| Code | Meaning |
|---|---|
| `0` | Operation succeeded (`status: ok`). Warnings may still be present. |
| `1` | Operation was refused (`status: error`). `data` is `null`. |
| `2` | Reserved for `axgf validate`: the operation succeeded but the report contains at least one `error`-severity diagnostic. |

The `1` vs `2` split exists so a CI job can gate saves on `axgf validate`
without misinterpreting an invalid bundle as a broken pipeline.

---

## Installation

Pick whichever is convenient. All three produce the same binary named `axgf`.

### 1. Precompiled binary from GitHub Releases *(fastest)*

Each tagged release attaches archives for five targets. On Linux the
statically-linked musl build is the one to grab — it runs on every
distribution regardless of libc:

```bash
# Replace v0.2.0 with the tag you want.
curl -L https://github.com/plkarin/axgf-lib/releases/download/v0.2.0/axgf-v0.2.0-x86_64-unknown-linux-musl.tar.gz \
  | tar -xz
sudo mv axgf-v0.2.0-x86_64-unknown-linux-musl/axgf /usr/local/bin/
```

Other archives in the same release: `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
`x86_64-pc-windows-msvc` (`.zip`). Every archive ships with a `.sha256`
sidecar.

### 2. `cargo install` from crates.io

```bash
cargo install axgf-rs --features cli
```

The crate publishes under the name **`axgf-rs`** but the binary it installs
is **`axgf`** (in `~/.cargo/bin/axgf`). The `cli` feature pulls in `clap`;
plain library consumers who omit it never take on that dependency.

### 3. Build from source

```bash
git clone https://github.com/plkarin/axgf-lib
cd axgf-lib
cargo build --release --features cli
./target/release/axgf --help
```

Requires Rust ≥ 1.88 (the crate's MSRV).

---

## 60-second quickstart

Convert a GEDCOM file, validate the result, and read back the stats — three
commands, plain bash:

```bash
$ axgf convert-gedcom --input tree.ged \
    | jq -c .data.bundle > tree.json

$ axgf validate --input tree.json \
    | jq '{status, errors: .data.errors, warnings: .data.warnings}'
{
  "status": "ok",
  "errors": 0,
  "warnings": 3
}

$ axgf inspect --input tree.json | jq .data.stats
{
  "persons": 3,
  "families": 1,
  "events": 1,
  "links": 0,
  "occupations": 1,
  "sources": 1,
  "places": 2,
  "documents": 2
}
```

The pattern repeats across every subcommand: run the library function,
extract the payload with `jq -c .data...`, pipe into the next call.

---

## Entity kinds

The `--kind` flag on `add`, `update`, and `delete` accepts the plural
collection names as their singular schema types. The list is fixed by the
AXGF specification:

| `--kind` | Collection in the flat bundle | Purpose |
|---|---|---|
| `person`     | `persons/`     | An individual. |
| `family`     | `families/`    | Union of persons + children. |
| `event`      | `events/`      | A dated fact touching one or more entities. |
| `link`       | `links/`       | Typed relationship outside the family graph. |
| `occupation` | `occupations/` | Dated profession/state of a person. |
| `source`     | `sources/`     | A cited piece of evidence. |
| `place`      | `places/`      | Geographic entity, reused across others. |
| `document`   | `documents/`   | A byte-carrying attachment (with optional metadata). |

---

## Subcommands

Every subcommand accepts `-h`/`--help`. Where an input flag is documented
as accepting `PATH`, passing `-` reads from stdin — this is what makes the
pipe pattern work.

### `axgf create`

Create an empty bundle stamped with the current spec version.

```
axgf create [--family-name <NAME>]
```

`--family-name` populates `manifest.family.name`. When omitted, the
`family` key is absent from the manifest.

```bash
$ axgf create --family-name "Karin" | jq '.data.manifest'
{
  "axgf": "1.0",
  "created_at": "2026-08-03T10:08:09.364671804Z",
  "updated_at": "2026-08-03T10:08:09.364671804Z",
  "stats": {
    "persons": 0, "families": 0, "events": 0, "links": 0,
    "occupations": 0, "sources": 0, "places": 0, "documents": 0
  },
  "family": { "name": "Karin" }
}
```

Exit codes: `0` always (the library never refuses this operation).

---

### `axgf inspect`

Return the manifest as-was plus freshly computed stats.

```
axgf inspect --input <PATH>
```

```bash
$ axgf inspect --input tree.json | jq .data.stats
{ "persons": 3, "families": 1, "events": 1, "links": 0,
  "occupations": 1, "sources": 1, "places": 2, "documents": 2 }
```

Useful for detecting manifest drift: compare `.data.manifest.stats`
against `.data.stats` — if they differ, the bundle's header is out of
sync with its entities.

Exit codes: `0` on ok, `1` on unparseable input or unsupported spec version.

---

### `axgf validate`

Run structural (JSON Schema) and semantic checks and print the report.

```
axgf validate --input <PATH>
```

Validation is non-blocking: the library returns `Status::Ok` even on
error-severity findings so callers can decide what to do about them. The
CLI escalates to **exit code 2** when any error-severity diagnostic is
present — the report *is* the answer.

```bash
$ axgf validate --input clean.json | jq .data
{ "errors": 0, "warnings": 0, "infos": 0, "total": 0 }
$ echo $?
0
```

A person listed as both spouse and child of the same family:

```bash
$ axgf validate --input cycle.json \
    | jq '{summary: .data, first: .diagnostics[0]}'
{
  "summary": { "errors": 1, "warnings": 0, "infos": 0, "total": 1 },
  "first": {
    "code": "CYCLE_DETECTED",
    "severity": "error",
    "message": "...",
    "entity_ref": "persons/550e8400-e29b-41d4-a716-446655440001"
  }
}
$ echo $?
2
```

Exit codes: `0` clean or warnings-only, `1` unparseable / unsupported spec
version, `2` at least one error-severity diagnostic.

---

### `axgf add`

Insert a new entity into a bundle.

```
axgf add --input <PATH> --kind <KIND> --entity <PATH>
```

Both `--input` and `--entity` accept `-` for stdin (only one at a time,
since stdin is a single stream). A missing `id` on the entity is filled
in with a fresh UUID v4; the minted id is returned in `data.id`.

```bash
$ cat person.json
{ "identity": {
    "name":   { "display": "Elise Bernard", "components": [] },
    "gender": { "value": "F" },
    "is_living": false } }

$ axgf add --input bundle.json --kind person --entity person.json \
    | jq '{status, id: .data.id, persons: (.data.bundle.persons | length)}'
{
  "status": "ok",
  "id": "8b51a62d-5a99-4725-b4a7-18bf4d4852ed",
  "persons": 1
}
```

Exit codes: `0` on ok (schema warnings do not block the add), `1` on
`ENTITY_ALREADY_EXISTS` or bad input.

---

### `axgf update`

Replace an existing entity in full. The incoming JSON *must* carry the
target `id`.

```
axgf update --input <PATH> --kind <KIND> --entity <PATH>
```

```bash
$ axgf update --input bundle.json --kind person --entity ghost.json \
    | jq '{status, code: .diagnostics[0].code}'
{
  "status": "error",
  "code": "ENTITY_NOT_FOUND"
}
$ echo $?
1
```

Exit codes: `0` on ok, `1` on `ENTITY_NOT_FOUND` or bad input.

---

### `axgf delete`

Delete an entity by id under a caller-chosen referential-integrity policy.

```
axgf delete --input <PATH> --kind <KIND> --id <UUID> [--policy <POLICY>]
```

`--policy` is `reject` (default), `cascade`, or `orphan`. Semantics match
[`DeletePolicy`](API.md#delete_entity):

- `reject` — refuse the delete if anything references the target;
- `cascade` — remove the target and physically remove all references;
- `orphan` — remove the target but preserve the shape of referring
  containers (scalar refs become `null`, array items keep their slot).

```bash
$ axgf delete --input bundle.json --kind person --id "$JEAN" \
    | jq '{status, code: .diagnostics[0].code, referrer: .diagnostics[0].entity_ref}'
{
  "status": "error",
  "code": "DELETE_BLOCKED_BY_REFERENCE",
  "referrer": "families/a71c..."
}
$ echo $?
1
```

Exit codes: `0` on ok, `1` on `DELETE_BLOCKED_BY_REFERENCE`,
`ENTITY_NOT_FOUND`, or bad input.

---

### `axgf dedup`

Run the two safe deduplication passes. Ambiguous merges are flagged with
`MANUAL_REVIEW_REQUIRED` diagnostics rather than performed.

```
axgf dedup --input <PATH>
```

```bash
$ axgf dedup --input bundle.json | jq .data
{
  "bundle": { "...": "..." },
  "merged_persons": 2,
  "merged_families": 1,
  "manual_review": 0
}
```

Exit codes: `0` on ok, `1` on bad input.

---

### `axgf import`

Decode a `.axgf` ZIP archive into a flat-bundle envelope.

```
axgf import --input <PATH>
```

`--input -` reads the archive bytes from stdin.

```bash
$ axgf import --input tree.axgf \
    | jq '{status, persons: (.data.persons | length), families: (.data.families | length)}'
{ "status": "ok", "persons": 3, "families": 1 }
```

Exit codes: `0` on ok, `1` on `ZIP_READ_ERROR`, `INVALID_JSON`,
`INVALID_BUNDLE_STRUCTURE`, or `UNSUPPORTED_SPEC_VERSION`.

---

### `axgf export`

Rebuild a `.axgf` ZIP archive from a flat bundle. Stats are recomputed
before writing so the archive is always internally consistent.

```
axgf export --input <PATH> [--output <PATH>]
```

Without `--output`, the returned envelope carries the ZIP as base64 in
`data.zip_base64`. With `--output`, the CLI decodes and writes the file
for you — the envelope is still printed on stdout.

```bash
$ axgf export --input tree.json --output tree.axgf \
    | jq '{status, size_bytes: .data.size_bytes}'
{ "status": "ok", "size_bytes": 9124 }

$ file tree.axgf
tree.axgf: Zip archive data, at least v2.0 to extract, compression method=deflate
```

Exit codes: `0` on ok, `1` on `ZIP_WRITE_ERROR` or bad input.

---

### `axgf convert-gedcom`

Convert a GEDCOM 5.5.1 byte stream to a flat AXGF bundle.

```
axgf convert-gedcom --input <PATH> [--confidence <FLOAT>] [--place-lang <TAG>]
```

- `--confidence` (default `0.8`) is applied to imported facts that carry
  no explicit confidence signal in the source.
- `--place-lang` (default `en`) is the BCP 47 tag stored on imported place
  names when the GEDCOM has no explicit language.

Feature-gated behind `gedcom` (default-on).

```bash
$ axgf convert-gedcom --input small.ged --confidence 0.8 --place-lang fr \
    | jq '{status,
           persons: (.data.bundle.persons | length),
           families: (.data.bundle.families | length),
           events: (.data.bundle.events | length)}'
{ "status": "ok", "persons": 3, "families": 1, "events": 1 }
```

Exit codes: `0` on ok (unrecognized tags surface as warnings), `1` on
unreadable input.

---

## Scripting patterns

### The `jq` pipeline

The envelope's `data` field carries whatever the operation produced.
`jq -c .data` extracts it in compact form, ready to feed back in as
`--input -`:

```bash
axgf create --family-name "Karin" \
  | jq -c .data \
  | axgf add --input - --kind person --entity elise.json \
  | jq -c .data.bundle \
  | axgf validate --input -
```

Because every step is a pure function on JSON, the pipeline can be
resumed at any point by replaying the previous step's output.

### CI gate: refuse to save a bundle with structural errors

Use exit code 2 as the gate. Warnings do not count:

```yaml
- name: Validate genealogy bundle
  run: axgf validate --input archive/family.json
```

`actions/setup-node` etc. treat exit 1 and 2 identically as failures, so
the job goes red for `CYCLE_DETECTED` but stays green for `SCHEMA_
VALIDATION_FAILED` (a warning). If you *want* to fail on warnings too,
add your own predicate:

```bash
diags=$(axgf validate --input family.json | jq '.data.warnings + .data.errors')
[[ "$diags" -eq 0 ]] || exit 1
```

### Chaining without touching disk

Every subcommand accepts `-` for its file arguments, so bundles can flow
through a script entirely in memory:

```bash
axgf convert-gedcom --input tree.ged \
  | jq -c .data.bundle \
  | axgf dedup --input - \
  | jq -c .data.bundle \
  | axgf export --input - --output tree.axgf
```

---

## See also

- [`API.md`](API.md) — the same operations from Rust.
- [`../README.md`](../README.md) — project overview and installation matrix.
- [`../SETUP.md`](../SETUP.md) — local dev, feature flags, regression suite.
