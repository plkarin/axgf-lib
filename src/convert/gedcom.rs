// SPDX-License-Identifier: Apache-2.0
//! # gedcom — GEDCOM 5.5.1 → flat AXGF bundle
//!
//! Feature-gated behind `gedcom` (default-on). Converts a GEDCOM 5.5.1
//! byte stream to a flat AXGF bundle. The converter is intentionally
//! forgiving about GEDCOM's many dialects and never fails hard on
//! unknown tags; instead it emits `GEDCOM_UNRECOGNIZED_TAG` warnings
//! and drops the offender.
//!
//! ## What is handled
//!
//! - **Encoding auto-detect**: UTF-8 BOM, UTF-16 LE/BE BOM, plain
//!   UTF-8, latin-1 fallback.
//! - **Line format** `<level> [<xref>] <tag> [<value>]` with `CONC`
//!   / `CONT` continuation merged into the parent value at parse
//!   time.
//! - **INDI → person** with NAME (repeated → alias `names[]`), SEX,
//!   BIRT, DEAT, OCCU (→ standalone Occupation entities), NOTE
//!   (inline and `@ref@`), TITL / FACT (appended to notes).
//! - **FAM → family** (+ MARR event when present) with HUSB, WIFE,
//!   CHIL (respecting `PEDI adopted` for adoption).
//! - **SOUR → source**, **OBJE → document** (both nesting styles:
//!   webtrees `1 OBJE / 2 FILE / 3 FORM / 3 TITL` and the flat
//!   pattern `1 OBJE / 2 FORM / 2 TITL / 2 FILE`).
//! - **PLAC** values become deduplicated `Place` entities keyed by
//!   normalized display string; the imported place carries the
//!   caller's `place_lang` BCP 47 tag.
//! - **Localized date qualifiers** in English, Polish, French, German
//!   (BEF/PRZED/AVANT/VOR, AFT/PO/APRÈS/NACH, ABT/OK/VERS/UM,
//!   BET…AND…/MIĘDZY…I…/ENTRE…ET…/ZWISCHEN…UND…) and localized month
//!   names in all four languages.
//! - **Partial dates**: `12 APR 1923` → exact, `APR 1923` → month,
//!   `1923` → year. Unparseable dates are preserved verbatim in the
//!   `note` field of the date object rather than dropped.
//! - **Xref namespace safety**: every `@X@` cross-reference in this
//!   file is mapped to a fresh UUID v4; two files' xrefs cannot
//!   collide even after later concatenation.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::FlatBundle;
use crate::boundary::lifecycle::{compute_stats, now_iso8601_utc};
use crate::CURRENT_SPEC_VERSION;

/// See [`crate::convert_gedcom`].
pub fn convert(gedcom_bytes: &[u8], default_confidence: f64, place_lang: &str) -> Envelope {
    let text = decode(gedcom_bytes);
    let lines: Vec<Line> = text.lines().filter_map(parse_line).collect();
    let records = build_records(lines);

    let mut bundle = new_bundle();
    let mut ctx = ConvertCtx {
        default_confidence,
        place_lang: place_lang.to_string(),
        xref_map: BTreeMap::new(),
        note_map: BTreeMap::new(),
        place_dedup: BTreeMap::new(),
        diagnostics: Vec::new(),
    };

    // Pass 1: assign a UUID to every xref'd top-level record so that
    // forward references (e.g. INDI → OBJE @M1@ that appears later)
    // resolve on first encounter.
    for r in &records {
        if let Some(x) = &r.xref {
            ctx.xref_map
                .entry(x.clone())
                .or_insert_with(|| Uuid::new_v4().to_string());
        }
    }
    // Pass 2: collect NOTE bodies keyed by @xref@ so INDI/FAM refs can
    // resolve.
    for r in &records {
        if r.tag == "NOTE" {
            if let Some(x) = &r.xref {
                ctx.note_map
                    .insert(x.clone(), collected_text(r));
            }
        }
    }
    // Pass 3: emit entities in a stable order.
    for r in &records {
        match r.tag.as_str() {
            "HEAD" | "TRLR" | "NOTE" | "REPO" => {}
            "INDI" => convert_indi(r, &mut bundle, &mut ctx),
            "FAM" => convert_fam(r, &mut bundle, &mut ctx),
            "SOUR" => convert_sour(r, &mut bundle, &mut ctx),
            "OBJE" => {
                convert_obje_top(r, &mut bundle, &mut ctx);
            }
            other => {
                ctx.diagnostics.push(Diagnostic {
                    code: DiagnosticCode::GedcomUnrecognizedTag,
                    severity: Severity::Warning,
                    message: format!("skipping unrecognized top-level tag {other:?}"),
                    entity_ref: None,
                });
            }
        }
    }

    // Manifest — finalize stats and stamp compatibility.
    let stats = compute_stats(&bundle);
    if let Value::Object(ref mut m) = bundle.manifest {
        m.insert("stats".into(), stats);
        m.insert("updated_at".into(), Value::String(now_iso8601_utc()));
        m.insert(
            "compatibility".into(),
            json!({"gedcom_source": "5.5.1"}),
        );
    }

    let flat = serde_json::to_value(&bundle).unwrap_or(Value::Null);
    Envelope::ok_with(json!({"bundle": flat}), ctx.diagnostics)
}

// =========================================================================
// Encoding auto-detect
// =========================================================================

fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(b"\xef\xbb\xbf") {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if bytes.starts_with(b"\xff\xfe") {
        let words: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&words);
    }
    if bytes.starts_with(b"\xfe\xff") {
        let words: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&words);
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // Latin-1 fallback: every byte is its own Unicode code point in U+0000..U+00FF.
    bytes.iter().map(|&b| b as char).collect()
}

// =========================================================================
// Line + record model
// =========================================================================

#[derive(Debug, Clone)]
struct Line {
    level: u32,
    xref: Option<String>,
    tag: String,
    value: String,
}

fn parse_line(raw: &str) -> Option<Line> {
    let s = raw.trim_end_matches(['\r', '\n']).trim_end();
    if s.is_empty() {
        return None;
    }
    let (level_str, rest) = s.split_once(' ')?;
    let level: u32 = level_str.parse().ok()?;
    let (xref, after_xref) = if level == 0 && rest.starts_with('@') {
        // Level-0 line with an xref definition: `@X@ TAG [value]`.
        let end = rest[1..].find('@').map(|i| i + 1);
        match end {
            Some(e) => (Some(rest[..=e].to_string()), rest[e + 1..].trim_start()),
            None => (None, rest),
        }
    } else {
        (None, rest)
    };
    let (tag, value) = match after_xref.split_once(' ') {
        Some((t, v)) => (t.to_string(), v.to_string()),
        None => (after_xref.to_string(), String::new()),
    };
    Some(Line {
        level,
        xref,
        tag,
        value,
    })
}

#[derive(Debug, Default, Clone)]
struct Record {
    tag: String,
    xref: Option<String>,
    value: String,
    children: Vec<Record>,
}

fn build_records(lines: Vec<Line>) -> Vec<Record> {
    let mut roots: Vec<Record> = Vec::new();
    for line in lines {
        if line.tag == "CONC" || line.tag == "CONT" {
            let newline = line.tag == "CONT";
            append_to_deepest(&mut roots, line.level, &line.value, newline);
            continue;
        }
        let node = Record {
            tag: line.tag,
            xref: line.xref,
            value: line.value,
            children: Vec::new(),
        };
        insert_at_level(&mut roots, line.level, node);
    }
    roots
}

fn insert_at_level(roots: &mut Vec<Record>, level: u32, node: Record) {
    if level == 0 {
        roots.push(node);
        return;
    }
    let mut cur = roots;
    for _ in 0..(level - 1) {
        match cur.last_mut() {
            Some(last) => cur = &mut last.children,
            None => return,
        }
    }
    if let Some(last) = cur.last_mut() {
        last.children.push(node);
    }
}

fn append_to_deepest(roots: &mut [Record], level: u32, txt: &str, newline: bool) {
    if level == 0 {
        return;
    }
    let mut cur = roots;
    for _ in 0..(level - 1) {
        match cur.last_mut() {
            Some(last) => cur = last.children.as_mut_slice(),
            None => return,
        }
    }
    if let Some(last) = cur.last_mut() {
        if newline {
            last.value.push('\n');
        }
        last.value.push_str(txt);
    }
}

/// Collected text: `record.value` plus any inline CONC/CONT that was
/// merged into it.
fn collected_text(r: &Record) -> String {
    r.value.clone()
}

// =========================================================================
// Convert context
// =========================================================================

struct ConvertCtx {
    default_confidence: f64,
    place_lang: String,
    /// GEDCOM xref (`@I1@`, `@F1@`, `@S1@`, `@M1@`, `@N1@`) → UUID.
    xref_map: BTreeMap<String, String>,
    /// NOTE record body keyed by xref.
    note_map: BTreeMap<String, String>,
    /// Place display string → UUID, so PLAC deduplicates across the
    /// whole file.
    place_dedup: BTreeMap<String, String>,
    diagnostics: Vec<Diagnostic>,
}

impl ConvertCtx {
    fn uuid_for(&mut self, xref: &str) -> String {
        self.xref_map
            .entry(xref.to_string())
            .or_insert_with(|| Uuid::new_v4().to_string())
            .clone()
    }
}

fn new_bundle() -> FlatBundle {
    let now = now_iso8601_utc();
    FlatBundle {
        manifest: json!({
            "axgf": CURRENT_SPEC_VERSION,
            "created_at": now.clone(),
            "updated_at": now,
            "stats": {
                "persons": 0, "families": 0, "events": 0, "links": 0,
                "occupations": 0, "sources": 0, "places": 0, "documents": 0
            },
            "generator": {"name": "axgf-rs/convert_gedcom", "version": env!("CARGO_PKG_VERSION")}
        }),
        ..Default::default()
    }
}

// =========================================================================
// Entity conversion — Person
// =========================================================================

fn convert_indi(r: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let Some(xref) = &r.xref else {
        return;
    };
    let id = ctx.uuid_for(xref);

    let mut names_iter = r.children.iter().filter(|c| c.tag == "NAME");
    let primary = names_iter.next();
    let aliases: Vec<&Record> = names_iter.collect();
    let display = primary.map(|n| gedcom_name_display(&n.value)).unwrap_or_default();
    let components = primary
        .map(|n| gedcom_name_components(&n.value))
        .unwrap_or_default();

    let mut identity = json!({
        "name": {"display": display, "components": components},
        "gender": {"value": pick_sex(r)},
        "is_living": pick_child_value(r, "DEAT").is_none(),
    });
    if !aliases.is_empty() {
        let names: Vec<Value> = aliases
            .iter()
            .map(|n| {
                json!({
                    "type": "alias",
                    "display": gedcom_name_display(&n.value),
                    "components": gedcom_name_components(&n.value)
                })
            })
            .collect();
        identity
            .as_object_mut()
            .map(|m| m.insert("names".to_string(), Value::Array(names)));
    }

    let mut person = Map::new();
    person.insert("id".into(), Value::String(id.clone()));
    person.insert("type".into(), Value::String("person".into()));
    person.insert("axgf_version".into(), Value::String("1.0".into()));
    person.insert("identity".into(), identity);

    if let Some(birt) = r.children.iter().find(|c| c.tag == "BIRT") {
        person.insert("birth".into(), event_block(birt, bundle, ctx));
    }
    if let Some(deat) = r.children.iter().find(|c| c.tag == "DEAT") {
        person.insert("death".into(), event_block(deat, bundle, ctx));
    }

    // Notes: NOTE children (inline or @ref@) + TITL + FACT
    let notes = collect_notes(r, ctx);
    if !notes.is_empty() {
        person.insert("notes".into(), Value::String(notes));
    }

    // Attached documents: children OBJE either inline or by @xref@.
    let docs = collect_person_documents(r, bundle, ctx);
    if !docs.is_empty() {
        person.insert("documents".into(), Value::Array(docs));
    }

    // OCCU children → standalone Occupation entities.
    for occu in r.children.iter().filter(|c| c.tag == "OCCU") {
        let occ_id = Uuid::new_v4().to_string();
        let mut occ = Map::new();
        occ.insert("id".into(), Value::String(occ_id.clone()));
        occ.insert("type".into(), Value::String("occupation".into()));
        occ.insert("axgf_version".into(), Value::String("1.0".into()));
        occ.insert("person_id".into(), Value::String(id.clone()));
        occ.insert("title".into(), Value::String(occu.value.clone()));
        occ.insert("confidence".into(), json!(ctx.default_confidence));
        // DATE nested → valid_from/valid_until (single DATE → valid_from)
        if let Some(date) = occu.children.iter().find(|c| c.tag == "DATE") {
            let d = parse_gedcom_date(&date.value);
            occ.insert("valid_from".into(), json!({"date": d}));
        }
        if let Some(place) = occu.children.iter().find(|c| c.tag == "PLAC") {
            let pid = ensure_place(&place.value, bundle, ctx);
            occ.insert("place_id".into(), Value::String(pid));
        }
        bundle.occupations.insert(occ_id, Value::Object(occ));
    }

    bundle.persons.insert(id, Value::Object(person));
}

fn pick_sex(r: &Record) -> String {
    r.children
        .iter()
        .find(|c| c.tag == "SEX")
        .map(|c| c.value.trim().to_string())
        .filter(|s| matches!(s.as_str(), "M" | "F" | "NB" | "U"))
        .unwrap_or_else(|| "U".to_string())
}

fn pick_child_value<'a>(r: &'a Record, tag: &str) -> Option<&'a Record> {
    r.children.iter().find(|c| c.tag == tag)
}

/// Turn a GEDCOM NAME value like `Jean /Pierre-Léonard/` into a
/// display string with slashes stripped.
fn gedcom_name_display(raw: &str) -> String {
    raw.replace('/', "").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract given / family name components from a GEDCOM NAME value.
fn gedcom_name_components(raw: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut order = 1;
    if let (Some(a), Some(b)) = (raw.find('/'), raw.rfind('/')) {
        if b > a {
            let given = raw[..a].trim();
            let family = raw[a + 1..b].trim();
            let suffix = raw[b + 1..].trim();
            if !given.is_empty() {
                out.push(json!({"type": "given_name", "value": given, "order": order}));
                order += 1;
            }
            if !family.is_empty() {
                out.push(json!({"type": "family_name", "value": family, "order": order}));
                order += 1;
            }
            if !suffix.is_empty() {
                out.push(json!({"type": "suffix", "value": suffix, "order": order}));
            }
            return out;
        }
    }
    // No slashes → whole thing is a given name.
    if !raw.trim().is_empty() {
        out.push(json!({"type": "given_name", "value": raw.trim(), "order": 1}));
    }
    out
}

/// Build a birth/death sub-object (`date`, `place_id`, `confidence`)
/// from a BIRT / DEAT record.
fn event_block(rec: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) -> Value {
    let mut out = Map::new();
    if let Some(date) = rec.children.iter().find(|c| c.tag == "DATE") {
        out.insert("date".into(), parse_gedcom_date(&date.value));
    }
    if let Some(place) = rec.children.iter().find(|c| c.tag == "PLAC") {
        let pid = ensure_place(&place.value, bundle, ctx);
        out.insert("place_id".into(), Value::String(pid));
    }
    out.insert("confidence".into(), json!(ctx.default_confidence));
    Value::Object(out)
}

/// Concatenate NOTE (inline + @ref@), TITL and FACT children into a
/// single notes string. Order is stable.
fn collect_notes(r: &Record, ctx: &ConvertCtx) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in &r.children {
        match c.tag.as_str() {
            "NOTE" => {
                let v = c.value.trim();
                if v.starts_with('@') && v.ends_with('@') {
                    if let Some(body) = ctx.note_map.get(v) {
                        parts.push(body.clone());
                    }
                } else if !v.is_empty() {
                    parts.push(v.to_string());
                }
            }
            "TITL" | "FACT" => {
                let v = c.value.trim();
                if !v.is_empty() {
                    parts.push(format!("{}: {v}", c.tag));
                }
            }
            _ => {}
        }
    }
    parts.join("\n")
}

// =========================================================================
// Documents attached to INDI (both nesting styles)
// =========================================================================

fn collect_person_documents(
    r: &Record,
    bundle: &mut FlatBundle,
    ctx: &mut ConvertCtx,
) -> Vec<Value> {
    let mut out = Vec::new();
    for obje in r.children.iter().filter(|c| c.tag == "OBJE") {
        let v = obje.value.trim();
        if v.starts_with('@') && v.ends_with('@') {
            // Reference to a top-level OBJE — assume it exists (pass 1
            // assigned a UUID). Just link.
            let uuid = ctx.uuid_for(v);
            out.push(json!({"document_id": uuid, "role": "attachment"}));
        } else {
            // Inline OBJE — materialize a Document entity and link.
            let doc_id = Uuid::new_v4().to_string();
            let doc = obje_to_document(&doc_id, obje);
            bundle.documents.insert(doc_id.clone(), doc);
            out.push(json!({"document_id": doc_id, "role": "attachment"}));
        }
    }
    out
}

fn convert_obje_top(r: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let Some(x) = &r.xref else {
        return;
    };
    let id = ctx.uuid_for(x);
    let doc = obje_to_document(&id, r);
    bundle.documents.insert(id, doc);
}

/// Build a Document entity from an OBJE record. Handles the two
/// common GEDCOM layouts:
///
///   1 OBJE                          1 OBJE
///   2 FORM jpg                      2 FILE documents/file.jpg
///   2 TITL Photo                    3 FORM jpg
///   2 FILE documents/file.jpg       3 TITL Photo
fn obje_to_document(id: &str, r: &Record) -> Value {
    let mut form: Option<String> = None;
    let mut title: Option<String> = None;
    let mut file_path: Option<String> = None;

    for c in &r.children {
        match c.tag.as_str() {
            "FORM" => form = Some(c.value.clone()),
            "TITL" => title = Some(c.value.clone()),
            "FILE" => {
                file_path = Some(c.value.clone());
                // Webtrees: FORM / TITL nested UNDER FILE.
                for cc in &c.children {
                    match cc.tag.as_str() {
                        "FORM" => form = form.or(Some(cc.value.clone())),
                        "TITL" => title = title.or(Some(cc.value.clone())),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let path = file_path.unwrap_or_default();
    let filename = path.rsplit('/').next().unwrap_or(&path).to_string();
    let ext = filename
        .rsplit('.')
        .next()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let mime = mime_for_ext(&ext, form.as_deref());
    let doc_type = document_type_for_ext(&ext);

    let mut doc = Map::new();
    doc.insert("id".into(), Value::String(id.to_string()));
    doc.insert("type".into(), Value::String("document".into()));
    doc.insert("axgf_version".into(), Value::String("1.0".into()));
    doc.insert("filename".into(), Value::String(filename));
    doc.insert("mime_type".into(), Value::String(mime));
    doc.insert("document_type".into(), Value::String(doc_type));
    doc.insert("status".into(), Value::String("referenced".into()));
    if !path.is_empty() {
        doc.insert("file".into(), json!({"path": path}));
    }
    if let Some(t) = title {
        doc.insert("caption".into(), Value::String(t));
    }
    Value::Object(doc)
}

fn mime_for_ext(ext: &str, form: Option<&str>) -> String {
    // FORM takes precedence when present and non-trivial (webtrees puts
    // things like "jpg", "image/jpeg" or full mime strings there).
    if let Some(f) = form {
        let f = f.trim();
        if f.contains('/') {
            return f.to_string();
        }
    }
    match ext {
        "jpg" | "jpeg" => "image/jpeg".into(),
        "png" => "image/png".into(),
        "gif" => "image/gif".into(),
        "tif" | "tiff" => "image/tiff".into(),
        "webp" => "image/webp".into(),
        "pdf" => "application/pdf".into(),
        "txt" => "text/plain".into(),
        "html" | "htm" => "text/html".into(),
        "mp3" => "audio/mpeg".into(),
        "wav" => "audio/wav".into(),
        "mp4" => "video/mp4".into(),
        _ => "application/octet-stream".into(),
    }
}

fn document_type_for_ext(ext: &str) -> String {
    match ext {
        "jpg" | "jpeg" | "png" | "gif" | "tif" | "tiff" | "webp" => "photo".into(),
        "mp3" | "wav" => "audio".into(),
        "mp4" => "video".into(),
        _ => "other".into(),
    }
}

// =========================================================================
// Entity conversion — Family
// =========================================================================

fn convert_fam(r: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let Some(xref) = &r.xref else {
        return;
    };
    let id = ctx.uuid_for(xref);

    let mut persons: Vec<Value> = Vec::new();
    for c in &r.children {
        if c.tag == "HUSB" || c.tag == "WIFE" {
            let v = c.value.trim();
            if v.starts_with('@') && v.ends_with('@') {
                let pid = ctx.uuid_for(v);
                persons.push(json!({"person_id": pid, "role": "spouse"}));
            }
        }
    }
    let mut children: Vec<Value> = Vec::new();
    let mut order = 1;
    for c in r.children.iter().filter(|c| c.tag == "CHIL") {
        let v = c.value.trim();
        if v.starts_with('@') && v.ends_with('@') {
            let pid = ctx.uuid_for(v);
            let mut entry = json!({"person_id": pid, "birth_order": order});
            // Optional PEDI adopted etc. (kept as note field).
            if let Some(pedi) = c.children.iter().find(|cc| cc.tag == "PEDI") {
                entry["note"] = Value::String(format!("pedigree: {}", pedi.value));
            }
            children.push(entry);
            order += 1;
        }
    }

    let mut union = Map::new();
    union.insert("type".into(), Value::String("marriage".into()));
    union.insert("persons".into(), Value::Array(persons.clone()));

    let mut event_id: Option<String> = None;
    if let Some(marr) = r.children.iter().find(|c| c.tag == "MARR") {
        let mut start = Map::new();
        if let Some(d) = marr.children.iter().find(|c| c.tag == "DATE") {
            start.insert("date".into(), parse_gedcom_date(&d.value));
        }
        if let Some(pl) = marr.children.iter().find(|c| c.tag == "PLAC") {
            let pid = ensure_place(&pl.value, bundle, ctx);
            start.insert("place_id".into(), Value::String(pid));
        }
        // Create standalone marriage event.
        let eid = Uuid::new_v4().to_string();
        let mut ev = Map::new();
        ev.insert("id".into(), Value::String(eid.clone()));
        ev.insert("type".into(), Value::String("event".into()));
        ev.insert("axgf_version".into(), Value::String("1.0".into()));
        ev.insert("category".into(), Value::String("marriage".into()));
        if let Some(d) = start.get("date") {
            ev.insert("date".into(), d.clone());
        } else {
            ev.insert("date".into(), json!({"value": "", "precision": "unknown"}));
        }
        if let Some(p) = start.get("place_id") {
            ev.insert("place_id".into(), p.clone());
        }
        let mut parts: Vec<Value> = Vec::new();
        for p in &persons {
            if let Some(pid) = p.get("person_id").and_then(Value::as_str) {
                parts.push(json!({"entity_type":"person","entity_id":pid,"role":"spouse"}));
            }
        }
        parts.push(json!({"entity_type": "family", "entity_id": &id, "role": "created"}));
        ev.insert("participants".into(), Value::Array(parts));
        ev.insert("confidence".into(), json!(ctx.default_confidence));
        bundle.events.insert(eid.clone(), Value::Object(ev));
        event_id = Some(eid.clone());
        start.insert("event_id".into(), Value::String(eid));
        union.insert("start".into(), Value::Object(start));
    }

    let mut fam = Map::new();
    fam.insert("id".into(), Value::String(id.clone()));
    fam.insert("type".into(), Value::String("family".into()));
    fam.insert("axgf_version".into(), Value::String("1.0".into()));
    fam.insert("union".into(), Value::Object(union));
    if !children.is_empty() {
        fam.insert("children".into(), Value::Array(children));
    }
    let notes = collect_notes(r, ctx);
    if !notes.is_empty() {
        fam.insert("notes".into(), Value::String(notes));
    }
    // Silence unused_variables warning; event_id is captured inside union.start already.
    let _ = event_id;

    bundle.families.insert(id, Value::Object(fam));
}

// =========================================================================
// Entity conversion — Source
// =========================================================================

fn convert_sour(r: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let Some(xref) = &r.xref else {
        return;
    };
    let id = ctx.uuid_for(xref);
    let title = r
        .children
        .iter()
        .find(|c| c.tag == "TITL")
        .map(|c| c.value.clone())
        .unwrap_or_else(|| "Untitled source".to_string());

    let mut src = Map::new();
    src.insert("id".into(), Value::String(id.clone()));
    src.insert("type".into(), Value::String("source".into()));
    src.insert("axgf_version".into(), Value::String("1.0".into()));
    src.insert("title".into(), Value::String(title));
    src.insert("source_type".into(), Value::String("other".into()));
    src.insert("reliability".into(), Value::String("unknown".into()));
    src.insert("confidence".into(), json!(ctx.default_confidence));
    bundle.sources.insert(id, Value::Object(src));
}

// =========================================================================
// Place dedup
// =========================================================================

fn ensure_place(raw: &str, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) -> String {
    let key = raw.trim().to_string();
    if let Some(existing) = ctx.place_dedup.get(&key) {
        return existing.clone();
    }
    let id = Uuid::new_v4().to_string();
    let mut place = Map::new();
    place.insert("id".into(), Value::String(id.clone()));
    place.insert("type".into(), Value::String("place".into()));
    place.insert("axgf_version".into(), Value::String("1.0".into()));
    place.insert(
        "names".into(),
        json!([{"lang": ctx.place_lang, "value": key.clone(), "is_primary": true}]),
    );
    bundle.places.insert(id.clone(), Value::Object(place));
    ctx.place_dedup.insert(key, id.clone());
    id
}

// =========================================================================
// Date parsing — multi-language qualifiers
// =========================================================================

/// Parse a GEDCOM DATE value into an AXGF `axgf_date` object. Never
/// fails: unparseable dates are preserved verbatim in the `note`
/// field with `precision: "unknown"`.
fn parse_gedcom_date(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({"value": null, "precision": "unknown"});
    }
    let upper = normalize_qualifier(trimmed);

    // Range qualifiers first.
    if let Some(rest) = strip_prefix_ci(&upper, "BET ") {
        // BET <a> AND <b>  (may also appear in other languages, but all normalise
        // to BET…AND at this point via normalize_qualifier).
        if let Some((a, b)) = split_range(rest) {
            let (av, ay) = try_parse_ymd(&a);
            let (bv, by) = try_parse_ymd(&b);
            return json!({
                "value": av.clone().unwrap_or_default(),
                "precision": "range",
                "range": {
                    "start": av.or(Some(a.clone())).unwrap_or(a),
                    "end":   bv.or(Some(b.clone())).unwrap_or(b),
                    "start_year": ay,
                    "end_year":   by
                }
            });
        }
    }
    if let Some(rest) = strip_prefix_ci(&upper, "BEF ") {
        let (v, y) = try_parse_ymd(rest);
        return json!({
            "value": v.clone().unwrap_or(rest.to_string()),
            "precision": "range",
            "range": {"end": v.unwrap_or(rest.to_string()), "end_year": y}
        });
    }
    if let Some(rest) = strip_prefix_ci(&upper, "AFT ") {
        let (v, y) = try_parse_ymd(rest);
        return json!({
            "value": v.clone().unwrap_or(rest.to_string()),
            "precision": "range",
            "range": {"start": v.unwrap_or(rest.to_string()), "start_year": y}
        });
    }
    if let Some(rest) = strip_prefix_ci(&upper, "ABT ") {
        let (v, _y) = try_parse_ymd(rest);
        return json!({"value": v.unwrap_or(rest.to_string()), "circa": true, "precision": "year"});
    }

    // Plain date (possibly with month name).
    let (v, _y) = try_parse_ymd(&upper);
    if let Some(val) = v {
        // Precision by number of dashes present.
        let dashes = val.matches('-').count();
        let precision = match dashes {
            2 => "exact",
            1 => "month",
            _ => "year",
        };
        return json!({"value": val, "precision": precision});
    }
    // Unparseable — preserve as a note per the module contract.
    json!({"value": null, "precision": "unknown", "note": raw.to_string()})
}

/// Uppercase + English-alias any localised qualifier. Preserves the
/// rest of the string (month names are handled by `try_parse_ymd`).
fn normalize_qualifier(s: &str) -> String {
    let up = s.to_uppercase();
    // Prefix aliases → BEF/AFT/ABT/BET/AND.
    let aliases: &[(&str, &str)] = &[
        // Polish
        ("PRZED ", "BEF "),
        ("OK ", "ABT "),
        ("OKOŁO ", "ABT "),
        ("PO ", "AFT "),
        ("MIĘDZY ", "BET "),
        (" I ", " AND "),
        // French
        ("AVANT ", "BEF "),
        ("AV ", "BEF "),
        ("APRÈS ", "AFT "),
        ("APRES ", "AFT "),
        ("AP ", "AFT "),
        ("VERS ", "ABT "),
        ("ENV ", "ABT "),
        ("ENTRE ", "BET "),
        (" ET ", " AND "),
        // German
        ("VOR ", "BEF "),
        ("NACH ", "AFT "),
        ("UM ", "ABT "),
        ("ZWISCHEN ", "BET "),
        (" UND ", " AND "),
    ];
    let mut out = up;
    for (from, to) in aliases {
        if out.contains(from) {
            out = out.replace(from, to);
        }
    }
    // Polish year suffix "R" / "ROKU" — strip.
    out = out.replace(" ROKU", "").replace(" R.", "").trim().to_string();
    // If it ends with a stray " R", drop it.
    if let Some(stripped) = out.strip_suffix(" R") {
        out = stripped.to_string();
    }
    out
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn split_range(s: &str) -> Option<(String, String)> {
    // Split on AND with surrounding whitespace tolerance.
    let up = s.to_uppercase();
    let idx = up.find(" AND ")?;
    let a = s[..idx].trim().to_string();
    let b = s[idx + 5..].trim().to_string();
    if a.is_empty() || b.is_empty() {
        None
    } else {
        Some((a, b))
    }
}

/// Try to interpret `s` as `DD MON YYYY` / `MON YYYY` / `YYYY` where
/// MON can be an English or localised abbreviation/full month name.
/// Returns the ISO 8601 rendering and the year (if parseable).
fn try_parse_ymd(s: &str) -> (Option<String>, Option<i32>) {
    let s = s.trim();
    // Just a year?
    if s.chars().all(|c| c.is_ascii_digit()) && (s.len() == 4) {
        return (Some(s.to_string()), s.parse().ok());
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    match parts.len() {
        3 => {
            let day: Option<u32> = parts[0].parse().ok();
            let month = month_number(parts[1]);
            let year: Option<i32> = parts[2].parse().ok();
            if let (Some(d), Some(m), Some(y)) = (day, month, year) {
                return (
                    Some(format!("{y:04}-{m:02}-{d:02}")),
                    Some(y),
                );
            }
        }
        2 => {
            let month = month_number(parts[0]);
            let year: Option<i32> = parts[1].parse().ok();
            if let (Some(m), Some(y)) = (month, year) {
                return (Some(format!("{y:04}-{m:02}")), Some(y));
            }
        }
        1 => {
            let year: Option<i32> = parts[0].parse().ok();
            if let Some(y) = year {
                return (Some(format!("{y:04}")), Some(y));
            }
        }
        _ => {}
    }
    (None, None)
}

/// English + Polish + French + German month names / abbreviations.
fn month_number(m: &str) -> Option<u32> {
    let key = m.to_uppercase();
    let key = key.trim_end_matches('.').trim();
    match key {
        // English 3-letter (GEDCOM canonical).
        "JAN" | "JANUARY" | "STYCZEŃ" | "STYCZNIA" | "JANVIER" | "JANUAR" => Some(1),
        "FEB" | "FEBRUARY" | "LUTY" | "LUTEGO" | "FÉVRIER" | "FEVRIER" | "FEBRUAR" => Some(2),
        "MAR" | "MARCH" | "MARZEC" | "MARCA" | "MARS" | "MÄRZ" | "MAERZ" => Some(3),
        "APR" | "APRIL" | "KWIECIEŃ" | "KWIETNIA" | "AVRIL" => Some(4),
        "MAY" | "MAJ" | "MAJA" | "MAI" => Some(5),
        "JUN" | "JUNE" | "CZERWIEC" | "CZERWCA" | "JUIN" | "JUNI" => Some(6),
        "JUL" | "JULY" | "LIPIEC" | "LIPCA" | "JUILLET" | "JULI" => Some(7),
        "AUG" | "AUGUST" | "SIERPIEŃ" | "SIERPNIA" | "AOÛT" | "AOUT" => Some(8),
        "SEP" | "SEPT" | "SEPTEMBER" | "WRZESIEŃ" | "WRZEŚNIA" | "SEPTEMBRE" => Some(9),
        "OCT" | "OCTOBER" | "PAŹDZIERNIK" | "PAŹDZIERNIKA" | "OCTOBRE" | "OKTOBER" => Some(10),
        "NOV" | "NOVEMBER" | "LISTOPAD" | "LISTOPADA" | "NOVEMBRE" => Some(11),
        "DEC" | "DECEMBER" | "GRUDZIEŃ" | "GRUDNIA" | "DÉCEMBRE" | "DECEMBRE" | "DEZEMBER" => {
            Some(12)
        }
        _ => None,
    }
}
