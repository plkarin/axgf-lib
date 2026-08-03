# axgf-lib - V1 API surface

Every public function returns the same envelope. Diagnostics carry stable
machine-readable codes.

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

### Accessing `data` from Rust

`Envelope::data` is a `serde_json::Value`, **not** an `Option<Value>`. On error
it holds `Value::Null`, which serializes to JSON `null`. Access it directly -
`.expect()` and `.unwrap()` do not exist on `Value` and will not compile:

```rust
// WRONG - Value has no expect() / unwrap()
let flat = env.data.expect("failed").to_string();

// RIGHT
let flat = env.data.to_string();

// Detecting failure
if env.data.is_null() {
    for d in &env.diagnostics {
        eprintln!("{:?}: {}", d.code, d.message);
    }
}
```

Indexing a `Value` with `[]` is infallible and yields `Value::Null` for missing
keys, so a chain like `env.data["bundle"]` never panics. Converting to a
concrete type is where fallibility appears, and there `Option` methods are
correct:

```rust
let d  = env.data;
let id = d["id"].as_str().expect("id missing");   // as_str() -> Option<&str>
let n  = d["stats"]["persons"].as_u64().unwrap_or(0);
```

Validation is **non-blocking**: an envelope with `status: "ok"` may still
carry `warning` - or even `error`-severity - diagnostics. The status reflects
whether the operation was refused, not whether the bundle is problem-free.

### The flat bundle

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
untouched - forward-compatibility is a hard requirement.

---

## Table of contents

- [Lifecycle](#lifecycle) - `create_bundle`, `import_bundle`, `export_bundle`, `inspect`
- [Validation](#validation) - `validate`
- [CRUD](#crud) - `add_entity`, `update_entity`, `delete_entity`
- [Cleanup](#cleanup) - `deduplicate`
- [Conversion](#conversion) - `convert_gedcom`
- [Worked scenarios](#worked-scenarios) - multi-call recipes
- [Diagnostic codes](#diagnostic-codes-stable-public-contract)

---

## Lifecycle

### `create_bundle(family_name: Option<&str>) -> Envelope`

Return a fresh empty bundle whose manifest declares `axgf: "1.0"`. If
`family_name` is provided, populates `manifest.family.name`.

**data**: the flat bundle JSON.

#### Demo A - start a new family archive

*Reproduces: a user clicks "New family tree" in a client application. The
library hands back an empty but structurally complete bundle, ready to
receive entities.*

```rust
use axgf_rs::create_bundle;

let env = create_bundle(Some("Pierre-Leonard Family"));
println!("{}", env.to_json());
```

```json
{
  "status": "ok",
  "data": {
    "manifest": {
      "axgf": "1.0",
      "created_at": "2026-08-01T21:02:20Z",
      "updated_at": "2026-08-01T21:02:20Z",
      "stats": { "persons": 0, "families": 0, "events": 0, "links": 0,
                 "occupations": 0, "sources": 0, "places": 0, "documents": 0 },
      "family": { "name": "Pierre-Leonard Family" }
    },
    "persons": {}, "families": {}, "events": {}, "links": {},
    "occupations": {}, "sources": {}, "places": {}, "documents": {}
  },
  "diagnostics": []
}
```

#### Demo B - anonymous scratch bundle

*Reproduces: a conversion pipeline that builds a bundle in memory before it
knows what the family should be called. `manifest.family` is simply absent.*

```rust
let flat = create_bundle(None).data.to_string();
// manifest.family is omitted entirely - not null, not empty string
```

---

### `import_bundle(zip_bytes: &[u8]) -> Envelope`

Unpack a `.axgf` ZIP archive into flat JSON. The manifest's `axgf` field is
checked against `SUPPORTED_SPEC_VERSIONS`; a mismatch yields
`UNSUPPORTED_SPEC_VERSION`. Files under `documents/files/**` and any other
non-entity paths (e.g. `vault/**`) land in `attachments` as base64.

**Diagnostics**: `ZIP_READ_ERROR`, `INVALID_JSON`, `INVALID_BUNDLE_STRUCTURE`,
`UNSUPPORTED_SPEC_VERSION`.

**data on success**: the flat bundle JSON.

#### Demo A - open a file from disk (caller handles I/O)

*Reproduces: a desktop client opening a `.axgf` a user double-clicked. Note
the library never touches the filesystem - the caller reads the bytes.*

```rust
use axgf_rs::import_bundle;

let bytes = std::fs::read("family.axgf")?;      // caller's responsibility
let env   = import_bundle(&bytes);
if env.data.is_null() {
    return Err("import failed - see diagnostics");
}
let flat = env.data.to_string();
```

#### Demo B - a bundle carrying scanned certificates

*Reproduces: importing an archive that embeds binary documents. The entity
metadata lands in `documents`, the actual bytes in `attachments`, keyed by
their original in-archive path.*

```json
{
  "status": "ok",
  "data": {
    "manifest": { "axgf": "1.0", "stats": { "documents": 2, ... } },
    "documents": {
      "d1f0...": {
        "type": "document",
        "filename": "birth-cert-1923.pdf",
        "mime_type": "application/pdf",
        "document_type": "birth_certificate",
        "status": "present",
        "file": { "path": "documents/files/d1f0....pdf",
                  "size_bytes": 184320, "sha256": "a3f8..." }
      }
    },
    "attachments": {
      "documents/files/d1f0....pdf": "JVBERi0xLjQKJc...",
      "vault/wiki/persons/550e....md": "IyBKZWFuIFBpZXJyZS1MZW9uYXJk..."
    }
  },
  "diagnostics": []
}
```

#### Demo C - rejecting a future-spec bundle

*Reproduces: a V1 build receiving an AXGF 2.0 archive. The library refuses
rather than silently mangling data it does not understand. This gating is
why a stale client can never corrupt a newer file.*

```rust
let env = import_bundle(&bytes_from_axgf_2_0);
assert!(env.data.is_null());
```

```json
{
  "status": "error",
  "data": null,
  "diagnostics": [
    { "code": "UNSUPPORTED_SPEC_VERSION", "severity": "error",
      "message": "bundle declares axgf 2.0; this build supports 1.0",
      "entity_ref": "manifest" }
  ]
}
```

#### Demo D - garbage input

*Reproduces: a user renaming a JPEG to `.axgf`, or a truncated download.*

```rust
let env = import_bundle(b"not a zip at all");
// status: error, code: ZIP_READ_ERROR, data: null
```

---

### `export_bundle(flat_json: &str) -> Envelope`

Rebuild a `.axgf` ZIP archive from a flat bundle. Stats are recomputed and
`updated_at` is refreshed. The canonical schema is embedded at
`schema/axgf-1.0.schema.json`. Attachments are written back at their original
paths.

**data on success**: `{ "zip_base64": "...", "size_bytes": u }`.

#### Demo A - save to disk

*Reproduces: the "Save" button. The library produces bytes; the caller
decides where they go - a file, an S3 bucket, a database blob, a download
response.*

```rust
use axgf_rs::export_bundle;
use base64::Engine as _;

let d   = export_bundle(&flat_json).data;
let b64 = d["zip_base64"].as_str().ok_or("export failed")?;
let zip = base64::engine::general_purpose::STANDARD.decode(b64)?;
std::fs::write("family.axgf", zip)?;             // caller's responsibility
```

```json
{
  "status": "ok",
  "data": { "zip_base64": "UEsDBBQAAAAIA...", "size_bytes": 48216 },
  "diagnostics": []
}
```

#### Demo B - stats reconciliation happens on export

*Reproduces: a client that hand-edited the flat JSON and left `stats` stale.
Export recomputes them from the actual entities, so the written archive is
always internally consistent.*

```rust
// flat_json.manifest.stats.persons says 3, but persons has 5 keys
let env = export_bundle(&flat_json);
// the produced ZIP's manifest.json reports persons: 5
```

#### Demo C - serving a download from a web backend

*Reproduces: a SaaS endpoint returning the archive to a browser. The base64
crosses the library boundary; the HTTP layer decodes once.*

```rust
let d = export_bundle(&flat_json).data;
// HTTP 200, Content-Type: application/vnd.axgf+zip
// Content-Length: d["size_bytes"]
// body: base64-decoded d["zip_base64"]
```

---

### `inspect(flat_json: &str) -> Envelope`

Read-only. Returns the manifest as-was plus a freshly computed stats block -
useful for reconciling a bundle whose manifest has drifted out of sync with
its entities.

**data on success**: `{ "manifest": {...}, "stats": {...} }`.

#### Demo A - a dashboard header

*Reproduces: a client rendering "767 persons, 295 families" without
deserializing every entity. Cheap enough to call on every page load.*

```rust
use axgf_rs::inspect;

let d = inspect(&flat_json).data;
println!("{} persons in {}",
         d["stats"]["persons"],
         d["manifest"]["family"]["name"]);
```

#### Demo B - detecting manifest drift

*Reproduces: an integrity check. If the manifest's own stats disagree with
the recomputed ones, something wrote entities without updating the header -
worth surfacing to the operator.*

```rust
let d = inspect(&flat_json).data;
if d["manifest"]["stats"] != d["stats"] {
    eprintln!("manifest drift: declared {}, actual {}",
              d["manifest"]["stats"], d["stats"]);
}
```

---

## Validation

### `validate(flat_json: &str) -> Envelope`

Runs structural (JSON Schema) and semantic checks. Always returns
`Status::Ok` on parseable input - the report is in `diagnostics`.

Semantic layers:

| Code | Severity | Meaning |
|---|---|---|
| `SCHEMA_VALIDATION_FAILED` | Warning | Entity or manifest violates the embedded JSON Schema. |
| `DANGLING_REFERENCE` | Warning | An `_id` field points to an entity not present in the bundle. |
| `CYCLE_DETECTED` | Error | A parent/child cycle exists in the derived DAG. |
| `CHRONOLOGY_CONFLICT` | Warning | Child's birth year precedes parent's. |
| `DUPLICATE_UNIQUE_REF` | Warning | Two families share the same `union.persons` set. |

**data on success**: `{ "errors": u, "warnings": u, "infos": u, "total": u }`.

#### Demo A - a clean bundle

*Reproduces: the happy path. Zero diagnostics means the bundle is
structurally sound and semantically coherent.*

```json
{
  "status": "ok",
  "data": { "errors": 0, "warnings": 0, "infos": 0, "total": 0 },
  "diagnostics": []
}
```

#### Demo B - a research-grade bundle with open questions

*Reproduces: real genealogical work in progress. A source was referenced
before it was entered, and a transcription error put a child before its
parent. Neither blocks the operation - the bundle is usable, the operator
sees a punch list.*

```json
{
  "status": "ok",
  "data": { "errors": 0, "warnings": 3, "infos": 0, "total": 3 },
  "diagnostics": [
    { "code": "DANGLING_REFERENCE", "severity": "warning",
      "message": "persons/550e... birth.source_id references missing sources/src-9f2a...",
      "entity_ref": "persons/550e8400-e29b-41d4-a716-446655440001" },
    { "code": "CHRONOLOGY_CONFLICT", "severity": "warning",
      "message": "child born 1918 precedes parent born 1923",
      "entity_ref": "persons/7c1d..." },
    { "code": "DUPLICATE_UNIQUE_REF", "severity": "warning",
      "message": "families/f1a2... and families/f9b3... share the same spouse set",
      "entity_ref": "families/f9b3..." }
  ]
}
```

#### Demo C - a genuine data corruption

*Reproduces: a bad import or a buggy client that made someone their own
ancestor. `CYCLE_DETECTED` is error-severity because no downstream consumer
can safely walk this graph.*

```json
{
  "status": "ok",
  "data": { "errors": 1, "warnings": 0, "infos": 0, "total": 1 },
  "diagnostics": [
    { "code": "CYCLE_DETECTED", "severity": "error",
      "message": "parent/child cycle: 550e... -> 7c1d... -> 550e...",
      "entity_ref": "persons/550e8400-e29b-41d4-a716-446655440001" }
  ]
}
```

#### Demo D - gating a save on validation

*Reproduces: a client policy that refuses to write a file containing
error-severity findings, while tolerating warnings.*

```rust
use axgf_rs::{validate, export_bundle};

let errors = validate(&flat_json).data["errors"].as_u64().unwrap_or(0);
if errors > 0 {
    return Err("refusing to save a bundle with structural errors");
}
let zip = export_bundle(&flat_json).data;
```

---

## CRUD

### `add_entity(flat, kind, entity_json) -> Envelope`

- Generates a UUID v4 when the caller omits `id`.
- Fills in `type` and `axgf_version` if missing.
- Structurally validates the entity - schema failures surface as warnings but
  the add still succeeds.
- Refuses duplicates with `ENTITY_ALREADY_EXISTS`.

`kind` is [`EntityKind`](../src/logic/crud.rs): one of `Person`, `Family`,
`Event`, `Link`, `Occupation`, `Source`, `Place`, `Document`.

**data on success**: `{ "id": "<uuid>", "bundle": <flat> }`.

#### Demo A - add a person, let the library assign the id

*Reproduces: a user filling a "New person" form. The client sends no `id`;
the library returns the one it minted so the client can select the new row.*

```rust
use axgf_rs::{add_entity, EntityKind};
use serde_json::json;

let person = json!({
  "identity": {
    "name": { "display": "Jean Pierre-Leonard", "components": [
      { "type": "given_name",  "value": "Jean",           "order": 1 },
      { "type": "family_name", "value": "Pierre-Leonard", "order": 2 }
    ]},
    "gender": { "value": "M" },
    "is_living": false,
    "visibility": "members"
  },
  "birth": { "date": { "value": "1923-04-12", "calendar": "gregorian",
                       "precision": "exact", "circa": false, "confidence": 0.98 } }
});

let d      = add_entity(&flat, EntityKind::Person, &person.to_string()).data;
let new_id = d["id"].as_str().expect("id missing").to_string();
let flat   = d["bundle"].to_string();     // carry forward
```

#### Demo B - add a person with a caller-chosen id

*Reproduces: an importer that must preserve stable identifiers from a source
system so re-running the import is idempotent.*

```rust
let person = json!({
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "identity": { "name": { "display": "Elise Bernard", "components": [] },
                "gender": { "value": "F" },
                "is_living": false, "visibility": "members" }
});
let env = add_entity(&flat, EntityKind::Person, &person.to_string());
// re-running the same call yields ENTITY_ALREADY_EXISTS and data: null
```

#### Demo C - a family binding two existing persons

*Reproduces: recording a marriage. The family references persons by id; it
does not embed them. The two spouses must already exist or validation will
later report `DANGLING_REFERENCE`.*

```rust
let family = json!({
  "name": "Family Jean x Elise",
  "union": {
    "type": "marriage",
    "status": "ended_by_death",
    "persons": [
      { "person_id": jean_id,  "role": "spouse" },
      { "person_id": elise_id, "role": "spouse" }
    ],
    "start": { "date": { "value": "1948-06-15", "precision": "exact" } },
    "confidence": 0.99
  },
  "children": []
});
let d    = add_entity(&flat, EntityKind::Family, &family.to_string()).data;
let flat = d["bundle"].to_string();
```

#### Demo D - an event touching three entities at once

*Reproduces: the same marriage as a first-class historical fact. The event
references both spouses, a witness, and the family it created - the case
that motivates Events being independent rather than owned by a person.*

```rust
let event = json!({
  "category": "marriage",
  "subcategory": "civil",
  "date": { "value": "1948-06-15", "precision": "exact", "confidence": 0.99 },
  "place_id": paris_14_id,
  "participants": [
    { "entity_type": "person", "entity_id": jean_id,   "role": "spouse_1", "confidence": 0.99 },
    { "entity_type": "person", "entity_id": elise_id,  "role": "spouse_2", "confidence": 0.99 },
    { "entity_type": "person", "entity_id": andre_id,  "role": "witness",  "confidence": 0.90 },
    { "entity_type": "family", "entity_id": family_id, "role": "created",  "confidence": 0.99 }
  ],
  "description": "Civil marriage, Paris 14th"
});
let flat = add_entity(&flat, EntityKind::Event, &event.to_string())
    .data["bundle"].to_string();
```

#### Demo E - a non-family relationship

*Reproduces: recording that Jean was Jules' godfather - a bond GEDCOM cannot
express. The link is dated, sourced and carries its own confidence,
independent of both persons.*

```rust
let link = json!({
  "from":  { "entity_type": "person", "entity_id": jean_id },
  "to":    { "entity_type": "person", "entity_id": jules_id },
  "label": "godfather",
  "label_reverse": "godchild",
  "category": "spiritual",
  "bidirectional": false,
  "valid_from": { "date": { "value": "1950-03-15", "precision": "exact" } },
  "confidence": 0.85,
  "source_id": letter_source_id
});
let flat = add_entity(&flat, EntityKind::Link, &link.to_string())
    .data["bundle"].to_string();
```

#### Demo F - a career as a state, not an event

*Reproduces: "schoolteacher from 1948 to 1978". Modelled as an Occupation
because it is a span, not a point in time - which is why statistics like
"most common trade per generation" are computable.*

```rust
let occ = json!({
  "person_id": jean_id,
  "title": "Schoolteacher",
  "title_latin": "Primary school teacher",
  "employer": { "name": "Public school of Saint-Denis", "place_id": st_denis_id },
  "valid_from":  { "date": { "value": "1948", "precision": "year" } },
  "valid_until": { "date": { "value": "1978", "precision": "year" } },
  "confidence": 0.90,
  "source_id": municipal_archive_id
});
let flat = add_entity(&flat, EntityKind::Occupation, &occ.to_string())
    .data["bundle"].to_string();
```

#### Demo G - a place reused by many entities

*Reproduces: entering Saint-Denis once and referencing it from births,
deaths, marriages and occupations. Note `country_history` - the same
coordinates can belong to different states over time.*

```rust
let place = json!({
  "names": [ { "lang": "fr", "value": "Saint-Denis", "is_primary": true },
             { "lang": "en", "value": "Saint-Denis, Reunion" } ],
  "place_type": "city",
  "country_current": "FR",
  "coordinates": { "lat": -20.8823, "lon": 55.4504, "precision": "city_center" },
  "country_history": [ { "country": "FR", "from": null, "until": null } ],
  "identifiers": { "wikidata": "Q47045" }
});
let d       = add_entity(&flat, EntityKind::Place, &place.to_string()).data;
let place_id = d["id"].as_str().expect("id missing").to_string();
let flat    = d["bundle"].to_string();
```

#### Demo H - a source, then a document that evidences it

*Reproduces: the two-step every serious researcher performs - register the
evidence, then attach the scan. Source carries the claim about reliability;
Document carries the bytes.*

```rust
let source = json!({
  "title": "Birth certificate no.47 - Jean Pierre-Leonard 1923",
  "source_type": "birth_certificate",
  "reliability": "primary",
  "confidence": 0.98,
  "status": "verified",
  "repository": { "name": "Departmental Archives of Reunion",
                  "reference": "5MI/47/1923/0047" }
});
let d      = add_entity(&flat, EntityKind::Source, &source.to_string()).data;
let src_id = d["id"].as_str().expect("id missing").to_string();
let flat   = d["bundle"].to_string();

let document = json!({
  "filename": "birth-cert-1923.pdf",
  "mime_type": "application/pdf",
  "document_type": "birth_certificate",
  "status": "present",
  "linked_to": [
    { "entity_type": "person", "entity_id": jean_id, "role": "subject" },
    { "entity_type": "source", "entity_id": src_id,  "role": "evidence" }
  ]
});
let flat = add_entity(&flat, EntityKind::Document, &document.to_string())
    .data["bundle"].to_string();
```

#### Demo I - a schema-imperfect entity still lands

*Reproduces: a partial record captured in the field. The add succeeds so the
user does not lose their input; the warning tells them what to complete
later. This is the non-blocking philosophy applied to writes.*

```json
{
  "status": "ok",
  "data": { "id": "9a2f...", "bundle": { "...": "..." } },
  "diagnostics": [
    { "code": "SCHEMA_VALIDATION_FAILED", "severity": "warning",
      "message": "identity.gender is required",
      "entity_ref": "persons/9a2f..." }
  ]
}
```

---

### `update_entity(flat, kind, entity_json) -> Envelope`

Replaces an existing entity in full. `entity.id` is required; a missing entity
yields `ENTITY_NOT_FOUND`. Callers send the whole entity, not a patch.

**data on success**: `{ "id": "<uuid>", "bundle": <flat> }`.

#### Demo A - read-modify-write

*Reproduces: editing one field in a UI. Because update is a full replace, the
client must fetch the current entity, mutate it, and send it back whole -
which also means an update can never silently drop fields the client did not
know about.*

```rust
use axgf_rs::{update_entity, EntityKind};

let bundle: serde_json::Value = serde_json::from_str(&flat)?;
let mut jean = bundle["persons"][jean_id].clone();
jean["id"]  = json!(jean_id);                    // id is mandatory on update
jean["bio"] = json!("Schoolteacher. Order of Merit, 1972.");

let flat = update_entity(&flat, EntityKind::Person, &jean.to_string())
    .data["bundle"].to_string();
```

#### Demo B - correcting a date after finding the certificate

*Reproduces: research progress. The birth was recorded as "circa 1923" from
family memory; the civil register now gives an exact date, so precision and
confidence both rise.*

```rust
jean["birth"]["date"] = json!({
  "value": "1923-04-12", "calendar": "gregorian",
  "precision": "exact", "circa": false, "confidence": 0.98
});
jean["birth"]["source_id"] = json!(birth_cert_source_id);

let flat = update_entity(&flat, EntityKind::Person, &jean.to_string())
    .data["bundle"].to_string();
```

#### Demo C - adding a child to an existing family

*Reproduces: a birth. The family entity owns the children list, so the update
targets the family, not the child.*

```rust
let mut family = bundle["families"][family_id].clone();
family["id"] = json!(family_id);
family["children"].as_array_mut().expect("children must be an array").push(json!({
    "person_id": robert_id, "birth_order": 1, "confidence": 0.99
}));

let flat = update_entity(&flat, EntityKind::Family, &family.to_string())
    .data["bundle"].to_string();
```

#### Demo D - update on an unknown id

*Reproduces: a stale client sending an edit for a row someone else already
deleted.*

```json
{
  "status": "error",
  "data": null,
  "diagnostics": [
    { "code": "ENTITY_NOT_FOUND", "severity": "error",
      "message": "no person with id 00000000-0000-4000-8000-000000000000",
      "entity_ref": "persons/00000000-0000-4000-8000-000000000000" }
  ]
}
```

#### Demo E - a sibling group gains its parents

*Reproduces: an imported branch cut off at the top, four siblings whose
parents are unknown. Spec §4.2.3 permits a family with `children` and no
`union`. A researcher later discovers the parents and needs to add a `union`
block **without losing the children or their ids**.*

*Why this demo exists*: `update_entity` is a **full replace**, not a patch.
The caller MUST read the existing family, add the new `union` block, and
send the merged object back. Constructing a fresh family with only the
newly-discovered union would silently drop the four children — a failure
mode that would corrupt the archive quietly, with no error. This is why
the read-modify-write pattern from Demo A is non-negotiable when growing
an entity: fetch first, mutate, then write.*

```rust
use axgf_rs::{update_entity, EntityKind};
use serde_json::json;

// The sibling group was recorded as a union-less family earlier.
let bundle: serde_json::Value = serde_json::from_str(&flat)?;

// STEP 1 — read the existing family. Do NOT rebuild it from scratch.
let mut family = bundle["families"][family_id].clone();

// STEP 2 — add the newly discovered union. Children stay exactly as
// they were: same ids, same birth_order, same array length.
family["union"] = json!({
    "type": "marriage",
    "persons": [
        { "person_id": father_id, "role": "spouse" },
        { "person_id": mother_id, "role": "spouse" },
    ]
});

// STEP 3 — write the merged object back.
let flat = update_entity(&flat, EntityKind::Family, &family.to_string())
    .data["bundle"].to_string();
```

**Anti-pattern — do not do this.** The children silently vanish because
update is a full replace and the fresh object never carried them:

```rust
// WRONG: constructs a fresh family object with only the union.
// The four children are gone from the archive after this call.
let fresh = json!({
    "id": family_id, "type": "family", "axgf_version": "1.0",
    "union": { "type": "marriage", "persons": [ /* ... */ ] }
});
update_entity(&flat, EntityKind::Family, &fresh.to_string());
```

Covered by `tests/crud.rs::family_gains_union_later_without_losing_children`.

---

### `delete_entity(flat, kind, id, policy) -> Envelope`

`policy` is [`DeletePolicy`](../src/logic/crud.rs):

- **`Reject`** - scan every other entity for a reference to `id`; if any
  exists, refuse with `DELETE_BLOCKED_BY_REFERENCE` (bundle unchanged).
- **`Cascade`** - remove the entity; physically remove references
  (array items dropped, scalar `_id` fields removed from their object).
- **`Orphan`** - remove the entity; preserve the shape of referring
  containers (scalar `_id` fields set to `null`, array items kept with
  `_id` inside them nulled).

V1 does *not* recursively delete referring entities - callers can chain
deletes if they want a deeper cascade.

**data on success**: `{ "id": "<uuid>", "bundle": <flat> }`.

#### Demo A - Reject as a safety net

*Reproduces: the default "Delete" button in a careful client. The person is
a spouse in a family, so the deletion is refused and the user is told why
before anything is lost.*

```rust
use axgf_rs::{delete_entity, EntityKind, DeletePolicy};

let env = delete_entity(&flat, EntityKind::Person, jean_id, DeletePolicy::Reject);
if env.data.is_null() {
    // bundle untouched; show env.diagnostics to the user
}
```

```json
{
  "status": "error",
  "data": null,
  "diagnostics": [
    { "code": "DELETE_BLOCKED_BY_REFERENCE", "severity": "error",
      "message": "persons/f293... is referenced by families/a71c....union.persons",
      "entity_ref": "families/a71c..." }
  ]
}
```

#### Demo B - Reject on an unreferenced entity succeeds

*Reproduces: removing a person entered by mistake before any relationship was
built. Nothing points at them, so Reject has nothing to object to.*

```rust
let flat = delete_entity(&flat, EntityKind::Person, orphan_id, DeletePolicy::Reject)
    .data["bundle"].to_string();
```

#### Demo C - Cascade to genuinely erase someone

*Reproduces: a GDPR erasure request, or undoing a bad merge. The person
disappears and every reference to them is physically removed - the family's
`union.persons` array shrinks, an event's participant entry is dropped.*

```rust
let flat = delete_entity(&flat, EntityKind::Person, jean_id, DeletePolicy::Cascade)
    .data["bundle"].to_string();
```

Before and after, on the referring family:

```json
// before
"union": { "persons": [ { "person_id": "f293...", "role": "spouse" },
                        { "person_id": "8b11...", "role": "spouse" } ] }
// after Cascade
"union": { "persons": [ { "person_id": "8b11...", "role": "spouse" } ] }
```

#### Demo D - Orphan to keep the record of an unknown

*Reproduces: "we know this child had a father, but the person record we
created for him was wrong". Orphan preserves the structure - the slot stays,
its occupant becomes unknown - which is often historically truer than
pretending the relationship never existed.*

```rust
let flat = delete_entity(&flat, EntityKind::Person, father_id, DeletePolicy::Orphan)
    .data["bundle"].to_string();
```

```json
// before
"union": { "persons": [ { "person_id": "f293...", "role": "spouse" },
                        { "person_id": "8b11...", "role": "spouse" } ] }
// after Orphan - array length preserved, identity nulled
"union": { "persons": [ { "person_id": null,     "role": "spouse" },
                        { "person_id": "8b11...", "role": "spouse" } ] }
```

And on a scalar reference:

```json
// before                             // after Orphan
"birth": { "place_id": "p4d2..." }    "birth": { "place_id": null }
// (under Cascade the place_id key would be removed entirely)
```

#### Demo E - deleting a place used across the whole tree

*Reproduces: merging two duplicate place records. Cascade strips the dead
place from every birth, death and event that referenced it; the caller then
points them at the surviving place with a series of updates.*

```rust
let flat = delete_entity(&flat, EntityKind::Place, dup_place_id, DeletePolicy::Cascade)
    .data["bundle"].to_string();
// every "place_id": "<dup>" key is now gone from persons, events, occupations
```

---

## Cleanup

### `deduplicate(flat) -> Envelope`

Two passes, in order:

1. **Person merge.** Groups persons by
   `(normalized_display_name, birth_year, death_year)`. Never merges
   father/son homonyms or same-name siblings/cousins - flags them with
   `MANUAL_REVIEW_REQUIRED` instead.
2. **Family merge.** Groups families by their sorted spouse set. Never
   merges families disagreeing on `union.type` or start-date year
   (>1 year apart).

When merged, the lowest-UUID member wins; references across the bundle are
rewritten from victims to keeper.

**data on success**: `{ "bundle": <flat>, "merged_persons": u,
"merged_families": u, "manual_review": u }`.

#### Demo A - the same couple entered twice

*Reproduces: the classic import artifact - two branches of a tree were
merged and the shared ancestors got duplicated. Both persons and the
resulting duplicate family collapse in one call.*

```rust
use axgf_rs::deduplicate;

let d    = deduplicate(&flat).data;
let flat = d["bundle"].to_string();
println!("merged {} persons, {} families", d["merged_persons"], d["merged_families"]);
```

```json
{
  "status": "ok",
  "data": {
    "bundle": { "...": "..." },
    "merged_persons": 2,
    "merged_families": 1,
    "manual_review": 0
  },
  "diagnostics": []
}
```

#### Demo B - homonyms that must NOT be merged

*Reproduces: the trap that makes naive deduplication destructive. Two men
named Henryk Frick exist - one is the spouse in family F, the other is a
child of that same family. They are father and son. The library detects the
generational relationship and refuses.*

```json
{
  "status": "ok",
  "data": { "bundle": { "...": "..." },
            "merged_persons": 0, "merged_families": 0, "manual_review": 2 },
  "diagnostics": [
    { "code": "MANUAL_REVIEW_REQUIRED", "severity": "info",
      "message": "persons/a11c... and persons/b22d... share a name but one is a child of a family the other is spouse in (father/son homonym)",
      "entity_ref": "persons/b22d..." },
    { "code": "MANUAL_REVIEW_REQUIRED", "severity": "info",
      "message": "persons/c33e... and persons/d44f... share a name but have different parents (cousins)",
      "entity_ref": "persons/d44f..." }
  ]
}
```

#### Demo C - families that disagree on the facts

*Reproduces: two records for the same couple, one saying "marriage 1948",
the other "cohabitation 1955". The library will not decide which is right -
it flags them and leaves both intact.*

```json
{
  "status": "ok",
  "data": { "bundle": { "...": "..." },
            "merged_persons": 0, "merged_families": 0, "manual_review": 1 },
  "diagnostics": [
    { "code": "MANUAL_REVIEW_REQUIRED", "severity": "info",
      "message": "families/f1a2... and families/f9b3... share a spouse set but union.type differs (marriage vs cohabitation)",
      "entity_ref": "families/f9b3..." }
  ]
}
```

#### Demo D - post-import hygiene pass

*Reproduces: the recommended pipeline after converting a legacy file -
convert, deduplicate, validate, then export.*

```rust
let bundle = convert_gedcom(&bytes, 0.8, "pl").data["bundle"].to_string();
let clean  = deduplicate(&bundle).data;
println!("merged {} persons, {} families, {} need review",
         clean["merged_persons"], clean["merged_families"], clean["manual_review"]);

let flat   = clean["bundle"].to_string();
let report = validate(&flat).data;
```

---

## Conversion

### `convert_gedcom(bytes, default_confidence, place_lang) -> Envelope`

Feature-gated behind `gedcom` (default-on). Converts a GEDCOM 5.5.1 byte
stream to a flat AXGF bundle. Never fails hard on unknown tags - emits
`GEDCOM_UNRECOGNIZED_TAG` warnings and drops the offender.

Handles: encoding auto-detect (UTF-8 BOM / UTF-16 / UTF-8 / latin-1),
localised date qualifiers in EN/PL/FR/DE, both webtrees `OBJE` layouts,
partial dates, `NOTE @xref@` resolution, `PEDI adopted`, and per-file xref
namespace isolation.

**data on success**: `{ "bundle": <flat> }`.

#### Demo A - migrating off Webtrees or Ancestry

*Reproduces: the entry point for most new users. `default_confidence` is the
trust level assigned to every imported fact, since GEDCOM has no way to
express uncertainty; 0.8 says "probably right, but not verified by me".*

```rust
use axgf_rs::convert_gedcom;

let bytes = std::fs::read("export-from-webtrees.ged")?;
let flat  = convert_gedcom(&bytes, 0.8, "en").data["bundle"].to_string();
```

#### Demo B - a Polish archive with localised dates

*Reproduces: the reason the converter is multilingual. Real webtrees exports
localise date qualifiers, and a parser that only knows `ABT`/`BEF`/`AFT`
silently loses them.*

Input fragment:

```
1 BIRT
2 DATE OK 1500
1 DEAT
2 DATE PRZED 1430 R
```

Output:

```json
"birth": { "date": { "value": "1500", "calendar": "gregorian",
                     "precision": "year", "circa": true, "confidence": 0.8 } },
"death": { "date": { "calendar": "gregorian", "precision": "unknown",
                     "circa": false,
                     "range": { "latest": { "value": "1430", "precision": "year" } } } }
```

*`OK` (approximately) becomes `circa: true`; `PRZED` (before) becomes a
range with only an upper bound. `place_lang: "pl"` tags every imported place
name as Polish.*

#### Demo C - text that is not a date

*Reproduces: free text typed into a date field - extremely common in real
files. The value is preserved as a note rather than discarded, so no
information is lost even though it cannot be parsed.*

Input: `2 DATE PO II WOJNIE` (after the Second World War)

```json
"death": { "date": { "calendar": "gregorian", "precision": "unknown",
                     "circa": false, "note": "PO II WOJNIE" } }
```

#### Demo D - occupations and notes become first-class

*Reproduces: GEDCOM's `OCCU` and `NOTE` tags being promoted. The occupation
leaves the person record and becomes its own dated entity; the note text
follows `@xref@` indirection to its target record.*

Input:

```
0 @N1@ NOTE Village school founder.
1 CONT Order of Merit, 1972.
0 @I1@ INDI
1 OCCU Schoolteacher
2 DATE FROM 1948 TO 1978
1 NOTE @N1@
```

Output: one `occupations/{uuid}` entity linked to the person, and the
person's `notes` field carrying both lines of the resolved note.

#### Demo E - a document reference from webtrees

*Reproduces: the OBJE nesting quirk. GEDCOM 5.5.1 puts `FORM` and `TITL`
underneath `FILE`, while older 5.5 files put them at `OBJE` level. Reading
only one layout produces `application/octet-stream` for every media file.*

```
0 @X1285@ OBJE
1 FILE photo-1955.jpg
2 FORM jpg
2 TITL Family photo, 1955
```

```json
"documents": { "e8a1...": {
  "filename": "photo-1955.jpg",
  "mime_type": "image/jpeg",
  "document_type": "photo",
  "status": "referenced",
  "caption": "Family photo, 1955"
} }
```

*`status` is `referenced` rather than `present` because the byte payload is
not alongside the `.ged` - the metadata is kept so the link can be repaired
later.*

#### Demo F - full migration pipeline

*Reproduces: end-to-end onboarding of a legacy file into a clean, validated,
saved archive.*

```rust
use base64::Engine as _;

let bytes  = std::fs::read("tree.ged")?;
let flat   = convert_gedcom(&bytes, 0.8, "pl").data["bundle"].to_string();
let clean  = deduplicate(&flat).data["bundle"].to_string();
let report = validate(&clean).data;
println!("validation: {} errors, {} warnings",
         report["errors"], report["warnings"]);

let zip = export_bundle(&clean).data;
let b64 = zip["zip_base64"].as_str().ok_or("export failed")?;
std::fs::write("family.axgf",
    base64::engine::general_purpose::STANDARD.decode(b64)?)?;
```

---

## Worked scenarios

### Scenario 1 - build a three-generation family from nothing

*Reproduces: a user starting from a blank slate and entering what they know.
Note the pattern: every call returns a new bundle that must be carried
forward, because the library is stateless.*

```rust
use axgf_rs::*;
use serde_json::json;

// helper: add an entity and carry the new bundle forward
fn push(flat: &str, kind: EntityKind, e: serde_json::Value) -> (String, String) {
    let d = add_entity(flat, kind, &e.to_string()).data;
    (d["id"].as_str().expect("id missing").to_string(),
     d["bundle"].to_string())
}

// 1. empty archive
let mut flat = create_bundle(Some("Pierre-Leonard Family")).data.to_string();

// 2. a place both generations will reference
let (paris, f) = push(&flat, EntityKind::Place, json!({
    "names": [{ "lang": "fr", "value": "Paris", "is_primary": true }],
    "place_type": "city", "country_current": "FR",
    "coordinates": { "lat": 48.8566, "lon": 2.3522 }
}));
flat = f;

// 3. generation 1
let (jean, f) = push(&flat, EntityKind::Person, json!({
    "identity": { "name": { "display": "Jean Pierre-Leonard", "components": [] },
                  "gender": { "value": "M" }, "is_living": false,
                  "visibility": "members" },
    "birth": { "date": { "value": "1923-04-12", "precision": "exact" },
               "place_id": paris }
}));
flat = f;

let (elise, f) = push(&flat, EntityKind::Person, json!({
    "identity": { "name": { "display": "Elise Bernard", "components": [] },
                  "gender": { "value": "F" }, "is_living": false,
                  "visibility": "members" }
}));
flat = f;

// 4. generation 2
let (robert, f) = push(&flat, EntityKind::Person, json!({
    "identity": { "name": { "display": "Robert Pierre-Leonard", "components": [] },
                  "gender": { "value": "M" }, "is_living": true,
                  "visibility": "members" },
    "birth": { "date": { "value": "1949-06", "precision": "month" },
               "place_id": paris }
}));
flat = f;

// 5. the family binding them
let (_family, f) = push(&flat, EntityKind::Family, json!({
    "union": { "type": "marriage",
               "persons": [ { "person_id": jean,  "role": "spouse" },
                            { "person_id": elise, "role": "spouse" } ],
               "start": { "date": { "value": "1948-06-15", "precision": "exact" } },
               "confidence": 0.99 },
    "children": [ { "person_id": robert, "birth_order": 1, "confidence": 0.99 } ]
}));
flat = f;

// 6. check coherence, then save
let errors = validate(&flat).data["errors"].as_u64().unwrap_or(0);
assert_eq!(errors, 0);
let zip = export_bundle(&flat).data;
```

### Scenario 2 - merge two archives from different relatives

*Reproduces: two cousins each maintaining a branch, now combining them. The
overlap is the shared ancestors, which deduplicate resolves.*

```rust
let a = import_bundle(&std::fs::read("cousin-a.axgf")?).data;
let b = import_bundle(&std::fs::read("cousin-b.axgf")?).data;

// merge collections client-side: the library does not join bundles in V1
let mut merged = a.clone();
for coll in ["persons","families","events","links",
             "occupations","sources","places","documents"] {
    let target = merged[coll].as_object_mut().expect("collection must be an object");
    if let Some(src) = b[coll].as_object() {
        for (id, entity) in src {
            target.insert(id.clone(), entity.clone());
        }
    }
}

// let the library resolve the overlap
let clean = deduplicate(&merged.to_string()).data;
println!("merged {} duplicate persons, {} families; {} need a human",
         clean["merged_persons"], clean["merged_families"], clean["manual_review"]);
```

### Scenario 3 - a validation dashboard

*Reproduces: a client surfacing research quality. Diagnostics group naturally
into a to-do list for the genealogist. Working from the serialized envelope
keeps this identical across every language binding.*

```rust
use std::collections::BTreeMap;

let env: serde_json::Value =
    serde_json::from_str(&validate(&flat).to_json())?;

let mut by_code: BTreeMap<String, Vec<String>> = BTreeMap::new();
if let Some(diags) = env["diagnostics"].as_array() {
    for d in diags {
        let code = d["code"].as_str().unwrap_or("UNKNOWN").to_string();
        let who  = d["entity_ref"].as_str().unwrap_or("-").to_string();
        by_code.entry(code).or_default().push(who);
    }
}
for (code, refs) in &by_code {
    println!("{code}: {} affected", refs.len());
}
```

```
CHRONOLOGY_CONFLICT: 2 affected
DANGLING_REFERENCE: 14 affected
DUPLICATE_UNIQUE_REF: 3 affected
```

### Scenario 4 - a stateless HTTP service

*Reproduces: exposing the library as a web API. Because every function is
pure, no session state is needed - the bundle travels with each request,
which is exactly what makes horizontal scaling trivial.*

```
POST   /api/bundle                    -> create_bundle
POST   /api/bundle/import  (ZIP body) -> import_bundle
POST   /api/bundle/export  (flat)     -> export_bundle
POST   /api/bundle/validate (flat)    -> validate
POST   /api/entity/:kind   (flat+e)   -> add_entity
PUT    /api/entity/:kind   (flat+e)   -> update_entity
DELETE /api/entity/:kind/:id?policy   -> delete_entity
POST   /api/bundle/dedup   (flat)     -> deduplicate
POST   /api/convert/gedcom (bytes)    -> convert_gedcom
```

Every handler is a one-liner: parse the request body, call the function,
return `envelope.to_json()` verbatim. The envelope already carries status and
diagnostics, so no error-mapping layer is needed.

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
