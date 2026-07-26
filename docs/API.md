# axgf-lib · V1 API surface

Every public function returns the same envelope. `data` on error is `null`;
diagnostics carry stable machine-readable codes.

```json
{
  "status": "ok" | "error",
  "data": <value | null>,
  "diagnostics": [
    { "code": "STABLE_CODE", "severity": "info|warning|error",
      "message": "...", "entity_ref": "collection/uuid" }
  ]
}
```

Validation is **non-blocking**: an envelope with `status: "ok"` may still
carry `warning` — or even `error`-severity — diagnostics. The status reflects
whether the operation was refused, not whether the bundle is problem-free.

Bundles cross the boundary as flat JSON:

```json
{
  "manifest":     { "axgf": "1.0", "created_at": "...", "stats": {...}, ... },
  "persons":      { "<uuid>": { ... }, ... },
  "families":     { ... },
  "events":       { ... },
  "links":        { ... },
  "occupations":  { ... },
  "sources":      { ... },
  "places":       { ... },
  "documents":    { ... },
  "attachments":  { "documents/files/{uuid}.pdf": "<base64>", ... }   // optional
}
```

The eight collections are always present in serialized output (empty `{}`
when nothing has landed yet). Unknown top-level fields survive a round-trip
untouched — forward-compatibility is a hard requirement.

---

## Lifecycle

### `create_bundle(family_name: Option<&str>) → Envelope`

Return a fresh empty bundle whose manifest declares `axgf: "1.0"`. If
`family_name` is provided, populates `manifest.family.name`.

**data**: the flat bundle JSON.

---

### `import_bundle(zip_bytes: &[u8]) → Envelope`

Unpack a `.axgf` ZIP archive into flat JSON. The manifest's `axgf` field is
checked against `SUPPORTED_SPEC_VERSIONS`; a mismatch yields
`UNSUPPORTED_SPEC_VERSION`. Files under `documents/files/**` and any other
non-entity paths (e.g. `vault/**`) land in `attachments` as base64.

**Diagnostics**: `ZIP_READ_ERROR`, `INVALID_JSON`, `INVALID_BUNDLE_STRUCTURE`,
`UNSUPPORTED_SPEC_VERSION`.

**data on success**: the flat bundle JSON.

---

### `export_bundle(flat_json: &str) → Envelope`

Rebuild a `.axgf` ZIP archive from a flat bundle. Stats are recomputed and
`updated_at` is refreshed. The canonical schema is embedded at
`schema/axgf-1.0.schema.json`. Attachments are written back at their original
paths.

**data on success**: `{ "zip_base64": "...", "size_bytes": u }`.

---

### `inspect(flat_json: &str) → Envelope`

Read-only. Returns the manifest as-was plus a freshly computed stats block —
useful for reconciling a bundle whose manifest has drifted out of sync with
its entities.

**data on success**: `{ "manifest": {...}, "stats": {...} }`.

---

## Validation

### `validate(flat_json: &str) → Envelope`

Runs structural (JSON Schema) and semantic checks. Always returns
`Status::Ok` on parseable input — the report is in `diagnostics`.

Semantic layers:

| Code | Severity | Meaning |
|---|---|---|
| `SCHEMA_VALIDATION_FAILED` | Warning | Entity or manifest violates the embedded JSON Schema. |
| `DANGLING_REFERENCE` | Warning | An `_id` field points to an entity not present in the bundle. |
| `CYCLE_DETECTED` | Error | A parent/child cycle exists in the derived DAG. |
| `CHRONOLOGY_CONFLICT` | Warning | Child's birth year precedes parent's. |
| `DUPLICATE_UNIQUE_REF` | Warning | Two families share the same `union.persons` set. |

**data on success**: `{ "errors": u, "warnings": u, "infos": u, "total": u }`.

---

## CRUD

### `add_entity(flat, kind, entity_json) → Envelope`

- Generates a UUID v4 when the caller omits `id`.
- Fills in `type` and `axgf_version` if missing.
- Structurally validates the entity — schema failures surface as warnings but
  the add still succeeds.
- Refuses duplicates with `ENTITY_ALREADY_EXISTS`.

`kind` is [`EntityKind`](../src/logic/crud.rs): one of `Person`, `Family`,
`Event`, `Link`, `Occupation`, `Source`, `Place`, `Document`.

**data on success**: `{ "id": "<uuid>", "bundle": <flat> }`.

---

### `update_entity(flat, kind, entity_json) → Envelope`

Replaces an existing entity in full. `entity.id` is required; a missing entity
yields `ENTITY_NOT_FOUND`. Callers send the whole entity, not a patch.

**data on success**: `{ "id": "<uuid>", "bundle": <flat> }`.

---

### `delete_entity(flat, kind, id, policy) → Envelope`

`policy` is [`DeletePolicy`](../src/logic/crud.rs):

- **`Reject`** — scan every other entity for a reference to `id`; if any
  exists, refuse with `DELETE_BLOCKED_BY_REFERENCE` (bundle unchanged).
- **`Cascade`** — remove the entity; physically remove references
  (array items dropped, scalar `_id` fields removed from their object).
- **`Orphan`** — remove the entity; preserve the shape of referring
  containers (scalar `_id` fields set to `null`, array items kept with
  `_id` inside them nulled).

V1 does *not* recursively delete referring entities — callers can chain
deletes if they want a deeper cascade.

**data on success**: `{ "id": "<uuid>", "bundle": <flat> }`.

---

## Cleanup

### `deduplicate(flat) → Envelope`

Two passes, in order:

1. **Person merge.** Groups persons by
   `(normalized_display_name, birth_year, death_year)`. Never merges
   father/son homonyms or same-name siblings/cousins — flags them with
   `MANUAL_REVIEW_REQUIRED` instead.
2. **Family merge.** Groups families by their sorted spouse set. Never
   merges families disagreeing on `union.type` or start-date year
   (>1 year apart).

When merged, the lowest-UUID member wins; references across the bundle are
rewritten from victims to keeper.

**data on success**: `{ "bundle": <flat>, "merged_persons": u,
"merged_families": u, "manual_review": u }`.

---

## Conversion

### `convert_gedcom(bytes, default_confidence, place_lang) → Envelope`

Feature-gated behind `gedcom` (default-on). Converts a GEDCOM 5.5.1 byte
stream to a flat AXGF bundle. Never fails hard on unknown tags — emits
`GEDCOM_UNRECOGNIZED_TAG` warnings and drops the offender.

Handles: encoding auto-detect (UTF-8 BOM / UTF-16 / UTF-8 / latin-1),
localised date qualifiers in EN/PL/FR/DE, both webtrees `OBJE` layouts,
partial dates, `NOTE @xref@` resolution, `PEDI adopted`, and per-file xref
namespace isolation.

**data on success**: `{ "bundle": <flat> }`.

---

## Diagnostic codes (stable public contract)

| Code | Emitted by |
|---|---|
| `UNSUPPORTED_SPEC_VERSION` | Every operation that reads a manifest |
| `INVALID_JSON` | Every operation |
| `INVALID_BUNDLE_STRUCTURE` | Import, CRUD |
| `SCHEMA_VALIDATION_FAILED` | Validate, CRUD (add/update) |
| `DANGLING_REFERENCE` | Validate |
| `DUPLICATE_ENTITY_ID` | CRUD |
| `DUPLICATE_UNIQUE_REF` | Validate |
| `CYCLE_DETECTED` | Validate |
| `CHRONOLOGY_CONFLICT` | Validate |
| `ENTITY_NOT_FOUND` | CRUD (update/delete) |
| `ENTITY_ALREADY_EXISTS` | CRUD (add) |
| `UNKNOWN_ENTITY_KIND` | Adapters |
| `DELETE_BLOCKED_BY_REFERENCE` | CRUD (delete under `Reject`) |
| `MANUAL_REVIEW_REQUIRED` | Deduplicate |
| `ZIP_READ_ERROR` | Import |
| `ZIP_WRITE_ERROR` | Export |
| `GEDCOM_PARSE_ERROR` | Convert (reserved) |
| `GEDCOM_UNRECOGNIZED_TAG` | Convert |
| `INTERNAL` | Fallback |

New codes may be added in any minor version. Existing codes never change
spelling or meaning.
