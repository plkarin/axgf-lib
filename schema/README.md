# AXGF JSON Schema — Vendored Copy

This directory contains a **vendored copy** of the AXGF 1.0 JSON Schema.

## Source of authority

The single, canonical source is the `axgf-spec` repository:

<https://github.com/plkarin/axgf-spec/blob/main/schema/axgf-1.0.schema.json>

Never edit `axgf-1.0.schema.json` in this directory by hand. Any change to
the schema must be made in `axgf-spec` first, then synced here.

## Why a copy exists

`axgf-rs` embeds this schema at compile time via `include_str!`. This is
deliberate:

- **Offline validation.** Consumers of the library can validate bundles
  without network access.
- **WASM targets.** In a browser or a sandboxed WASM runtime there is no
  filesystem and no way to fetch a remote resource at library-init time.
- **Reproducibility.** Every build of a given `axgf-rs` version validates
  against exactly the schema that shipped with it, not a moving target.

The tradeoff is drift risk, which is handled below.

## How to update

Run:

```sh
./scripts/sync-schema.sh
```

The script downloads the current schema from `axgf-spec` main and overwrites
this file. Review the diff. If it is more than a trivial change, the library
code likely needs corresponding updates.

## How drift is prevented

`.github/workflows/schema-drift.yml` compares the vendored copy against
`axgf-spec` main on every push, pull request, and weekly. Comparison is
**canonical** (parsed JSON, sorted keys) rather than byte-for-byte, so
insignificant formatting differences do not cause spurious failures.

The workflow deliberately does **not** auto-update this file. A schema
change may require corresponding changes to the library code (parsers,
validators, generated types) and therefore requires human review.
