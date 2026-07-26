# Contributing to axgf-lib

Thanks for your interest. This library is the reference implementation of the
[Axiom Genealogy Format](https://github.com/plkarin/axgf-spec) — correctness
and clarity outrank speed. Please read the [design contract in `README.md`](./README.md#design-contract)
before proposing changes; the five rules are non-negotiable in V1.

## Spec first

If your change touches format semantics, the spec is the authority. If the code
and the spec disagree, **the spec wins and the library is the bug**. Before
proposing a format change, open an issue at
[plkarin/axgf-spec](https://github.com/plkarin/axgf-spec) — the format is CC0
and evolves independently of any implementation.

## What lives where

- `src/model/` — typed structs for the eight entity kinds. Internal only;
  never crosses the boundary. Every field mirrors the schema.
- `src/logic/` — the value core: validation, CRUD, deduplication. Operates on
  typed model or on raw JSON with helper accessors; never on ZIPs or filesystem.
- `src/boundary/` — the only layer that speaks JSON, ZIP and bytes. Owns the
  uniform `Envelope`, the `FlatBundle`, and the lifecycle operations.
- `src/convert/` — foreign-format converters (GEDCOM 5.5.1 → AXGF).
- `src/adapters/` — thin per-target wrappers behind feature flags. Logic-free.
- `tests/` — integration tests, one file per public concern. Fixtures live in
  `tests/fixtures/`.

## Development workflow

```bash
cargo test                          # 82+ tests must all pass
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo doc --no-deps
```

Each of those must be green before you open a PR. Warnings are treated as
errors — please fix them locally, don't `#[allow]`.

## Style

- **Zero `unwrap()` / `expect()` / `panic!()` in library code.** Return
  `Result` or an error envelope. Tests may use `unwrap` freely.
- **Every public item has rustdoc.** Every module has `//!` docs explaining
  its role.
- **BTreeMap over HashMap** whenever the output is user-visible. Bundles must
  diff cleanly across runs.
- **Diagnostic codes are a public contract.** SCREAMING_SNAKE_CASE, never
  renamed. Add new codes freely; do not repurpose old ones.
- **Comments explain _why_.** Well-named identifiers explain _what_.

## Testing

- One integration test file per public concern (`tests/lifecycle.rs`,
  `tests/validation.rs`, `tests/crud.rs`, `tests/dedup.rs`,
  `tests/gedcom_convert.rs`, `tests/model_roundtrip.rs`).
- Tests describe behaviour, not internals. If the test name doesn't sound
  like a promise the library makes, rename it.
- GEDCOM fixtures live in `tests/fixtures/`. Keep them small — cover exactly
  one edge case per file.

## Commit messages

Conventional style, imperative mood. Recent history:

```
feat: entity CRUD with referential-integrity policies
feat: structural and semantic validation
feat: bundle lifecycle (create/import/export/inspect) with spec-version gating
```

The subject line is a promise. The body explains _why_ and, when useful, the
non-obvious trade-off. One logical change per commit.

## License

By contributing, you agree that your contributions are licensed under
Apache-2.0 (the crate's licence) and CC0 where they change format examples in
this repo. SPDX headers on every new file:

```rust
// SPDX-License-Identifier: Apache-2.0
```
