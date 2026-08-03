<!-- SPDX-License-Identifier: Apache-2.0 -->
# `axgf` — command-line reference

`axgf` is the standalone command-line entry point to the `axgf-rs` library.
Every V1 boundary function is exposed as one subcommand. By default each
prints a concise human summary on stdout; `--json` selects the raw JSON
envelope for piping through `jq`; `-q/--quiet` prints nothing and carries
the result in the exit code.

**Exit codes.** They are the machine-readable answer:

| Code | Meaning |
|---|---|
| `0` | Operation succeeded (`status: ok`). Warnings may still be present. |
| `1` | Operation was refused (`status: error`). The diagnostic is on stderr. |
| `2` | Reserved for `axgf validate`: the report contains at least one `error`-severity diagnostic. |

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
cargo install axgf-rs
```

The crate publishes under the name **`axgf-rs`** but the binary it installs
is **`axgf`** (in `~/.cargo/bin/axgf`). The `cli` feature is on by default;
library-only consumers can opt out with `default-features = false, features
= ["gedcom"]` to avoid the `clap` dependency.

### 3. Build from source

```bash
git clone https://github.com/plkarin/axgf-lib
cd axgf-lib
cargo build --release
./target/release/axgf --help
```

Requires Rust ≥ 1.88 (the crate's MSRV).

---

## Output modes

Every subcommand honours three top-level flags:

| Flag | Behavior |
|---|---|
| *(default)* | Concise human summary on stdout. Grouped diagnostic counts on stderr. |
| `--json` | The raw envelope on stdout and nothing else — pipes cleanly into `jq`. |
| `-q`, `--quiet` | Nothing on stdout. Errors still print on stderr. Result in exit code. |

Errors are reported on stderr as `CODE: message`, one per line, and the
process exits `1`.

---

## Bundle inputs and outputs

**Input.** Every subcommand that reads a bundle takes it as a positional
`PATH`. `--input <PATH>` is still accepted as an alias for backward
compatibility within this unreleased 0.2.0. `-` reads bytes from stdin
so pipelines can compose without touching disk. The reader autodetects
the on-disk form by extension: `.axgf` is decoded as a ZIP archive
(via `import_bundle`); anything else is read as flat JSON.

**Output.** Commands that produce a bundle take `-o/--output <PATH>`:

- **`create`** and **`convert-gedcom`** require `-o` (they have no input
  file that could be edited in place).
- **`add`**, **`update`**, **`delete`**, **`dedup`**, **`export`** edit
  the input file *in place* when `-o` is omitted. The write is atomic:
  the new bytes go to a sibling tempfile first, then rename over the
  target, so a mid-write failure never leaves you with a truncated
  bundle.
- Read-only commands (`inspect`, `validate`, `import`) never take `-o`.

The output form is chosen by extension: `.axgf` → ZIP archive (calls
`export_bundle` internally), anything else → flat JSON.

---

## 60-second quickstart

Convert a GEDCOM file, validate the result, inspect it — three commands,
plain bash:

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

---

## Entity kinds

The kind positional argument on `add`, `update`, and `delete` accepts the
singular schema names. The list is fixed by the AXGF specification:

| kind | Collection in the flat bundle | Purpose |
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

Every subcommand accepts `-h`/`--help`. Where an input path is documented
as `PATH`, passing `-` reads from stdin.

### `axgf create`

Create an empty bundle stamped with the current spec version.

```
axgf create [--name <NAME>] -o <PATH>
```

`--name` populates `manifest.family.name`. `-o` chooses `.axgf` (ZIP)
or `.json` (flat) by extension.

```console
$ axgf create --name "Demo" -o /tmp/demo.axgf
created bundle
  persons       0
  families      0
  events        0
  links         0
  occupations   0
  sources       0
  places        0
  documents     0
wrote demo.axgf (4 KiB)
```

Exit codes: `0` always (the library never refuses this operation).

---

### `axgf inspect` *(read-only)*

Return the manifest as-was plus freshly computed stats.

```
axgf inspect <PATH>
```

```console
$ axgf inspect /tmp/demo.axgf
demo.axgf
  axgf          1.0
  family        Demo
  persons       0
  families      0
  events        0
  links         0
  occupations   0
  sources       0
  places        0
  documents     0
```

Useful for detecting manifest drift: compare `.data.manifest.stats`
against `.data.stats` in `--json` mode — if they differ, the bundle's
header is out of sync with its entities.

Exit codes: `0` on ok, `1` on unparseable input or unsupported spec version.

---

### `axgf validate` *(read-only)*

Run structural (JSON Schema) and semantic checks.

```
axgf validate <PATH>
```

Validation is non-blocking: the library returns `Status::Ok` even on
error-severity findings so callers can decide what to do about them. The
CLI escalates to **exit code 2** when any error-severity diagnostic is
present — the report *is* the answer.

```console
$ axgf validate /tmp/t.axgf
validated t.axgf
  errors                     0
  warnings                   3
  SCHEMA_VALIDATION_FAILED   3
```

Exit codes: `0` clean or warnings-only, `1` unparseable / unsupported
spec version, `2` at least one error-severity diagnostic.

---

### `axgf add`

Insert a new entity of the given kind.

```
axgf add <KIND> <PATH> --data <PATH> [-o <PATH>]
```

The bundle path is positional; `--data` (alias `--entity`) points at the
entity JSON. A missing `id` on the entity is filled in with a fresh UUID
v4; the minted id is echoed in the summary and in `data.id` under
`--json`. Without `-o` the bundle is written back to the input path in
place.

```console
$ axgf add person /tmp/demo2.axgf --data /tmp/p.json
added person 4d9d022a-f9ed-494f-93f6-d03285262248
wrote demo2.axgf (4 KiB)
```

Exit codes: `0` on ok (schema warnings do not block the add), `1` on
`ENTITY_ALREADY_EXISTS` or bad input.

---

### `axgf update`

Replace an existing entity in full. The incoming JSON *must* carry the
target `id`.

```
axgf update <KIND> <PATH> --data <PATH> [-o <PATH>]
```

Exit codes: `0` on ok, `1` on `ENTITY_NOT_FOUND` or bad input.

---

### `axgf delete`

Delete an entity by id under a caller-chosen referential-integrity policy.

```
axgf delete <KIND> <PATH> --id <UUID> [--policy <POLICY>] [-o <PATH>]
```

`--policy` is `reject` (default), `cascade`, or `orphan`. Semantics match
[`DeletePolicy`](API.md#delete_entity):

- `reject` — refuse the delete if anything references the target;
- `cascade` — remove the target and physically remove all references;
- `orphan` — remove the target but preserve the shape of referring
  containers (scalar refs become `null`, array items keep their slot).

```console
$ axgf delete person /tmp/family.axgf --id f293... --policy reject
DELETE_BLOCKED_BY_REFERENCE: cannot delete persons/f293... under Reject: still referenced by 1 entities: ["families/a71c..."]
```

The error goes to stderr; the input file is not touched. Exit code `1`.

Exit codes: `0` on ok, `1` on `DELETE_BLOCKED_BY_REFERENCE`,
`ENTITY_NOT_FOUND`, or bad input.

---

### `axgf dedup`

Run the two safe deduplication passes. Ambiguous merges are flagged with
`MANUAL_REVIEW_REQUIRED` diagnostics rather than performed.

```
axgf dedup <PATH> [-o <PATH>]
```

```console
$ axgf dedup /tmp/family.axgf
deduplicated family.axgf
  merged persons    0
  merged families   0
  manual review     0
wrote family.axgf (…)
```

Exit codes: `0` on ok, `1` on bad input.

---

### `axgf import` *(read-only)*

Decode a `.axgf` ZIP archive and print a summary. Useful as the last
step of a pipeline that produced ZIP bytes on stdin; use `--json` to
capture the flat form for a follow-up command.

```
axgf import <PATH>
```

Exit codes: `0` on ok, `1` on `ZIP_READ_ERROR`, `INVALID_JSON`,
`INVALID_BUNDLE_STRUCTURE`, or `UNSUPPORTED_SPEC_VERSION`.

---

### `axgf export`

Rebuild a `.axgf` ZIP (or flat `.json`) from an input bundle. Stats are
recomputed before writing so the artifact is always internally
consistent.

```
axgf export <PATH> -o <PATH>
```

The output form is chosen by extension: `.axgf` → ZIP, anything else →
flat JSON.

Exit codes: `0` on ok, `1` on `ZIP_WRITE_ERROR` or bad input.

---

### `axgf convert-gedcom`

Convert a GEDCOM 5.5.1 byte stream to an AXGF bundle.

```
axgf convert-gedcom <PATH> -o <PATH> [--confidence <FLOAT>] [--place-lang <TAG>]
```

- `--confidence` (default `0.8`) is applied to imported facts that carry
  no explicit confidence signal in the source.
- `--place-lang` (default `en`) is the BCP 47 tag stored on imported place
  names when the GEDCOM has no explicit language.

Feature-gated behind `gedcom` (default-on).

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
```

Exit codes: `0` on ok (unrecognized tags surface as warnings), `1` on
unreadable input.

---

## Scripting patterns

### The `jq` pipeline

`--json` prints the raw envelope; `jq -c .data` extracts the payload:

```bash
axgf create --name "Karin" --json \
  | jq -c .data \
  | axgf add person - --data elise.json --json \
  | jq -c .data.bundle \
  | axgf validate -
```

Because every step is a pure function on JSON, the pipeline can be
resumed at any point by replaying the previous step's output.

### CI gate: refuse to save a bundle with structural errors

Use exit code 2 as the gate. Warnings do not count:

```yaml
- name: Validate genealogy bundle
  run: axgf validate archive/family.axgf
```

`actions/setup-node` etc. treat exit 1 and 2 identically as failures, so
the job goes red for `CYCLE_DETECTED` but stays green for
`SCHEMA_VALIDATION_FAILED` (a warning). If you *want* to fail on warnings
too, add your own predicate against `--json` output.

### Chaining without touching disk

Every subcommand accepts `-` for its input, so bundles can flow through
a script entirely in memory:

```bash
axgf convert-gedcom tree.ged --json \
  | jq -c .data.bundle \
  | axgf dedup - --json \
  | jq -c .data.bundle \
  | axgf export - -o tree.axgf
```

---

## See also

- [`API.md`](API.md) — the same operations from Rust.
- [`../README.md`](../README.md) — project overview and installation matrix.
- [`../SETUP.md`](../SETUP.md) — local dev, feature flags, regression suite.
