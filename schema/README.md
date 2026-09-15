# AXGF JSON Schemas — Vendored Copies

This directory contains **vendored copies** of the AXGF JSON Schemas: `axgf-1.0.schema.json`
and `axgf-1.1.schema.json`. 1.1 is a superset of 1.0; a bundle is validated
against, and exported with, the one its manifest declares.

## Source of authority

The single, canonical source is the `axgf-spec` repository:

<https://github.com/plkarin/axgf-spec/blob/main/schema/axgf-1.0.schema.json>  
<https://github.com/plkarin/axgf-spec/blob/main/schema/axgf-1.1.schema.json>

Never edit either file in this directory by hand. Any change to a schema
must be made in `axgf-spec` first, then synced here.

## Why a copy exists

`axgf-rs` embeds these schemas at compile time via `include_str!`. This is
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

The script downloads the current schemas from `axgf-spec` main and
overwrites these files. Review the diff. If it is more than a trivial change, the library
code likely needs corresponding updates.

## How drift is prevented

`.github/workflows/schema-drift.yml` compares each vendored copy against
`axgf-spec` main on every push, pull request, and weekly. Comparison is
**canonical** (parsed JSON, sorted keys) rather than byte-for-byte, so
insignificant formatting differences do not cause spurious failures.

The workflow deliberately does **not** auto-update these files. A schema
change may require corresponding changes to the library code (parsers,
validators, generated types) and therefore requires human review.

## The 1.1 registry

`src/model/profile/` restates the 1.1 schema's attributes and vocabularies
as Rust data. `tests/profile_registry.rs` holds the two against each other in
both directions, so a schema sync that adds an attribute or a term fails the
build until the registry has it too.
