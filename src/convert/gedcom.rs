// SPDX-License-Identifier: Apache-2.0
//! # gedcom — GEDCOM 5.5.1 → flat AXGF bundle
//!
//! Feature-gated behind `gedcom` (default-on). Converts a GEDCOM 5.5.1
//! byte stream to a flat AXGF bundle. The converter is intentionally
//! forgiving about GEDCOM's many dialects and never fails hard; what it
//! cannot carry it says so, with a `GEDCOM_UNRECOGNIZED_TAG` warning, so
//! that nothing is left behind without a word.
//!
//! ## What is handled
//!
//! - **Encoding auto-detect**: UTF-8 BOM, UTF-16 LE/BE BOM, plain
//!   UTF-8, latin-1 fallback.
//! - **Line format** `<level> [<xref>] <tag> [<value>]` with `CONC`
//!   / `CONT` continuation merged into the parent value at parse
//!   time. Lines deeper than [`MAX_DEPTH`] are left out, and said to be.
//! - **Records**: every top-level record keeps its own identity. A
//!   cross-reference used twice keeps both records (the second under a
//!   new id) and says so; an INDI with no cross-reference is kept. A
//!   pointer to a record of the wrong kind (a FAM's HUSB naming a family)
//!   or to nothing is not linked, and is reported.
//! - **INDI → person**: NAME (the slashes read as GEDCOM writes them,
//!   unclosed and doubled ones included; a name written as GIVN / SURN /
//!   NICK pieces; a suffix kept in the display, never as a component the
//!   schema refuses; each further NAME typed by its TYPE: birth, married,
//!   aka…), SEX (GEDCOM 7's `X` read as non-binary), BIRT and DEAT (with
//!   the cause of death, and the citation's page as evidence), OCCU
//!   (→ Occupation, with a FROM … TO … period), NOTE (inline and
//!   `@ref@`; a note that only looks like a pointer is kept as text),
//!   descriptive attributes (TITL, FACT, RELI, NATI, …) appended to the
//!   notes. **Every other event** — christening, baptism, burial,
//!   cremation, residence, census, emigration, immigration,
//!   naturalization, graduation, generic `EVEN` with its TYPE, … —
//!   becomes an Event; a burial or cremation means the person has died.
//!   FAMC / FAMS on the person complete the family when its own record
//!   forgot them, and `FAMC.PEDI` gives the child's lineage.
//! - **FAM → family**: HUSB, WIFE, CHIL (each child once), MARR (→ the
//!   union's start and a marriage Event; further MARR → further events),
//!   DIV / ANUL (→ the union's end and an Event), and the other family
//!   events.
//! - **Citations** (`SOUR @S@` with PAGE, QUAY, under an event): the
//!   source, the page and how sure (QUAY 0–3 → confidence) are kept.
//! - **SOUR → source**, **OBJE → document** (both nesting styles:
//!   webtrees `1 OBJE / 2 FILE / 3 FORM / 3 TITL` and the flat
//!   pattern `1 OBJE / 2 FORM / 2 TITL / 2 FILE`); a document is attached
//!   to a person once however often it is referenced; a file named by a
//!   Windows path is named by its last segment.
//! - **PLAC** values become deduplicated `Place` entities, one town
//!   written with other spacing or case being one place; `MAP` / `LATI` /
//!   `LONG` become its coordinates; an empty PLAC is no place. The
//!   imported place carries the caller's `place_lang` BCP 47 tag.
//! - **Dates**: `12 APR 1923` → exact, `APR 1923` → month, `1923` →
//!   year; ABT / CAL / EST → circa, keeping the date's own precision;
//!   BEF / AFT / BET…AND / FROM…TO → ranges; INT keeps its date and the
//!   words it was read from; dual years (`1750/51`); the Julian, French
//!   Republican and Hebrew calendars by their escapes; qualifiers and
//!   month names in English, Polish, French and German. An impossible
//!   day (31 FEB) is not an exact date, and a year outside 0–9999 has no
//!   value: both keep the words in `note`, as an unparseable date does.
//! - **Xref namespace safety**: every `@X@` cross-reference in this
//!   file is mapped to a fresh UUID v4; two files' xrefs cannot
//!   collide even after later concatenation.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::FlatBundle;
use crate::boundary::lifecycle::{compute_stats, now_iso8601_utc, raise_manifest_version};
use crate::CURRENT_SPEC_VERSION;

/// The deepest level a line may sit at. GEDCOM's own structures go to
/// about six; a file nesting thousands of levels is not a family tree,
/// and would otherwise make a tree as deep as itself.
pub const MAX_DEPTH: u32 = 64;

/// See [`crate::convert_gedcom`].
pub fn convert(gedcom_bytes: &[u8], default_confidence: f64, place_lang: &str) -> Envelope {
    let text = decode(gedcom_bytes);
    let mut too_deep = 0usize;
    let lines: Vec<Line> = text
        .lines()
        .filter_map(parse_line)
        .filter(|l| {
            let keep = l.level <= MAX_DEPTH;
            if !keep {
                too_deep += 1;
            }
            keep
        })
        .collect();
    let records = build_records(lines);

    let mut bundle = new_bundle();
    let mut ctx = ConvertCtx {
        default_confidence,
        place_lang: place_lang.to_string(),
        xref_map: BTreeMap::new(),
        kinds: BTreeMap::new(),
        note_map: BTreeMap::new(),
        place_dedup: BTreeMap::new(),
        famc: BTreeMap::new(),
        fams: BTreeMap::new(),
        repos: BTreeMap::new(),
        skipped: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    if too_deep > 0 {
        ctx.warn(format!(
            "left out {too_deep} line(s) nested deeper than level {MAX_DEPTH}"
        ));
    }

    // Pass 1: an id for every top-level record, so that forward
    // references (an INDI pointing at an OBJE further down) resolve on
    // first encounter. A cross-reference keeps the first record that
    // defines it; a second record under the same one is kept under a new
    // id, and said to be.
    let mut ids: Vec<String> = Vec::with_capacity(records.len());
    for r in &records {
        let id = match &r.xref {
            Some(x) if ctx.xref_map.contains_key(x) => {
                ctx.warn(format!(
                    "the cross-reference {x} names two {} records; the second is kept under a new id, and pointers to {x} reach the first",
                    r.tag
                ));
                Uuid::new_v4().to_string()
            }
            Some(x) => {
                let id = Uuid::new_v4().to_string();
                ctx.xref_map.insert(x.clone(), id.clone());
                ctx.kinds.insert(x.clone(), r.tag.clone());
                id
            }
            None => Uuid::new_v4().to_string(),
        };
        ids.push(id);
    }
    // Pass 2: NOTE bodies keyed by @xref@, so INDI/FAM refs can resolve;
    // and the links a person's own record makes to its families (FAMC as
    // a child, FAMS as a spouse), which complete a FAM record that forgot
    // them.
    for (r, id) in records.iter().zip(&ids) {
        if r.tag == "NOTE" {
            if let Some(x) = &r.xref {
                ctx.note_map
                    .entry(x.clone())
                    .or_insert_with(|| collected_text(r));
            }
        }
        if r.tag == "REPO" {
            if let (Some(x), Some(name)) = (&r.xref, child_text(r, "NAME")) {
                ctx.repos.entry(x.clone()).or_insert(name);
            }
        }
        if r.tag == "INDI" {
            for c in &r.children {
                let v = c.value.trim();
                if !is_pointer(v) {
                    continue;
                }
                match c.tag.as_str() {
                    "FAMC" => {
                        let pedi = c
                            .children
                            .iter()
                            .find(|cc| cc.tag == "PEDI")
                            .map(|cc| cc.value.trim().to_string());
                        ctx.famc
                            .entry(v.to_string())
                            .or_default()
                            .push((id.clone(), pedi));
                    }
                    "FAMS" => ctx.fams.entry(v.to_string()).or_default().push(id.clone()),
                    _ => {}
                }
            }
        }
    }
    // Pass 3: emit entities in a stable order.
    for (r, id) in records.iter().zip(&ids) {
        match r.tag.as_str() {
            "HEAD" | "TRLR" | "NOTE" | "REPO" | "SUBM" | "SUBN" => {}
            "INDI" => {
                if r.xref.is_none() {
                    ctx.warn(
                        "an INDI record has no cross-reference; it is kept, though nothing can point at it"
                            .to_string(),
                    );
                }
                convert_indi(r, id, &mut bundle, &mut ctx)
            }
            "FAM" => convert_fam(r, id, &mut bundle, &mut ctx),
            "SOUR" => convert_sour(r, id, &mut bundle, &mut ctx),
            "OBJE" => convert_obje_top(r, id, &mut bundle),
            other => {
                ctx.warn(format!("skipping unrecognized top-level tag {other:?}"));
            }
        }
    }

    // What was left behind, once per tag and place rather than once per
    // line.
    let skipped = std::mem::take(&mut ctx.skipped);
    for ((parent, tag), n) in skipped {
        ctx.warn(format!(
            "left out {n} {parent}.{tag} line(s): not carried by this converter"
        ));
    }

    // Manifest — finalize stats and stamp compatibility.
    let stats = compute_stats(&bundle);
    if let Value::Object(ref mut m) = bundle.manifest {
        m.insert("stats".into(), stats);
        m.insert("updated_at".into(), Value::String(now_iso8601_utc()));
        m.insert("compatibility".into(), json!({"gedcom_source": "5.5.1"}));
    }
    // A child's lineage is AXGF 1.1's: the manifest says what the bundle
    // holds.
    raise_manifest_version(&mut bundle);

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

/// `@X@`: GEDCOM's shape of a pointer. Whether it points at anything is
/// another matter (see [`ConvertCtx::pointer`]).
fn is_pointer(v: &str) -> bool {
    v.len() > 2 && v.starts_with('@') && v.ends_with('@') && !v[1..v.len() - 1].contains('@')
}

fn child<'a>(r: &'a Record, tag: &str) -> Option<&'a Record> {
    r.children.iter().find(|c| c.tag == tag)
}

fn child_text(r: &Record, tag: &str) -> Option<String> {
    child(r, tag)
        .map(|c| c.value.trim().to_string())
        .filter(|v| !v.is_empty())
}

// =========================================================================
// Convert context
// =========================================================================

struct ConvertCtx {
    default_confidence: f64,
    place_lang: String,
    /// GEDCOM xref (`@I1@`, `@F1@`, `@S1@`, `@M1@`, `@N1@`) → UUID, for
    /// the first record that defines it.
    xref_map: BTreeMap<String, String>,
    /// GEDCOM xref → the tag of the record that defines it.
    kinds: BTreeMap<String, String>,
    /// NOTE record body keyed by xref.
    note_map: BTreeMap<String, String>,
    /// Normalised place → UUID, so PLAC deduplicates across the whole
    /// file however a town is spaced or cased.
    place_dedup: BTreeMap<String, String>,
    /// Family xref → (child id, PEDI) from the children's own FAMC.
    famc: BTreeMap<String, Vec<(String, Option<String>)>>,
    /// Family xref → spouse ids from the spouses' own FAMS.
    fams: BTreeMap<String, Vec<String>>,
    /// Repository xref → its NAME, for a source's REPO.
    repos: BTreeMap<String, String>,
    /// (parent tag, tag) → lines left behind, reported once at the end.
    skipped: BTreeMap<(String, String), usize>,
    diagnostics: Vec<Diagnostic>,
}

impl ConvertCtx {
    fn warn(&mut self, message: String) {
        self.diagnostics.push(Diagnostic {
            code: DiagnosticCode::GedcomUnrecognizedTag,
            severity: Severity::Warning,
            message,
            entity_ref: None,
        });
    }

    fn skip(&mut self, parent: &str, tag: &str) {
        *self
            .skipped
            .entry((parent.to_string(), tag.to_string()))
            .or_default() += 1;
    }

    /// The id of the record `v` points at, when it is a record of the
    /// `want` kind. A pointer to nothing, or to a record of another kind,
    /// is reported and links nothing.
    fn pointer(&mut self, v: &str, want: &str, from: &str) -> Option<String> {
        let v = v.trim();
        if !is_pointer(v) {
            return None;
        }
        match self.kinds.get(v).cloned() {
            Some(kind) if kind == want => self.xref_map.get(v).cloned(),
            Some(kind) => {
                self.warn(format!(
                    "{from} points at {v}, a {kind} record where a {want} belongs; not linked"
                ));
                None
            }
            None => {
                self.warn(format!(
                    "{from} points at {v}, which no record defines; not linked"
                ));
                None
            }
        }
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

/// The person's events GEDCOM writes as their own lines, and what each
/// becomes: an AXGF event category and, where the category is broader
/// than the tag, a subcategory naming it.
fn indi_event(tag: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match tag {
        "CHR" => ("religious", Some("christening")),
        "CHRA" => ("religious", Some("adult christening")),
        "BAPM" => ("religious", Some("baptism")),
        "BLES" => ("religious", Some("blessing")),
        "CONF" => ("religious", Some("confirmation")),
        "FCOM" => ("religious", Some("first communion")),
        "ORDN" => ("religious", Some("ordination")),
        "BARM" => ("religious", Some("bar mitzvah")),
        "BASM" => ("religious", Some("bat mitzvah")),
        "BURI" => ("other", Some("burial")),
        "CREM" => ("other", Some("cremation")),
        "ADOP" => ("adoption", None),
        "EMIG" => ("migration", Some("emigration")),
        "IMMI" => ("migration", Some("immigration")),
        "NATU" => ("naturalization", None),
        "CENS" => ("census", None),
        "RESI" => ("other", Some("residence")),
        "GRAD" => ("social", Some("graduation")),
        "RETI" => ("social", Some("retirement")),
        "PROB" => ("legal", Some("probate")),
        "WILL" => ("legal", Some("will")),
        "EVEN" => ("other", None),
        _ => return None,
    })
}

/// Descriptive attributes: a fact about the person rather than an event,
/// kept in the notes as `TAG: value`.
const INDI_ATTRIBUTES: &[&str] = &[
    "TITL", "FACT", "RELI", "NATI", "DSCR", "EDUC", "PROP", "CAST", "NCHI", "NMR", "IDNO", "SSN",
];

/// What a person's record may hold that this converter reads, or knows to
/// be bookkeeping with nothing to carry.
const INDI_HANDLED: &[&str] = &[
    "NAME", "SEX", "BIRT", "DEAT", "OCCU", "NOTE", "OBJE", "FAMC", "FAMS", "SOUR", "CHAN", "RIN",
    "REFN", "RFN", "AFN", "UID", "_UID", "_UPD", "CREA",
];

fn convert_indi(r: &Record, id: &str, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let who = r
        .xref
        .clone()
        .unwrap_or_else(|| "an INDI without a cross-reference".to_string());
    let mut names_iter = r.children.iter().filter(|c| c.tag == "NAME");
    let primary = names_iter.next();
    let aliases: Vec<&Record> = names_iter.collect();
    // Spec §4.1.1: an unknown identity uses a bracketed placeholder
    // rather than an empty display (schema §3.2 axgf_name.display has
    // minLength 1). Do not invent components — leave the array empty
    // so consumers can still detect that no real name was recorded.
    let primary_name = primary.map(read_name).filter(|n| !n.display.is_empty());
    let mut name = json!({"display": "[Unknown]", "components": []});
    if let Some(n) = &primary_name {
        name = n.to_json();
    }

    // Died, as GEDCOM says it: a death, a burial or a cremation.
    let died = r
        .children
        .iter()
        .any(|c| matches!(c.tag.as_str(), "DEAT" | "BURI" | "CREM"));
    let (sex, sex_note) = pick_sex(r);
    let mut gender = json!({"value": sex});
    if let Some(note) = sex_note {
        gender["note"] = Value::String(note);
    }
    let mut identity = json!({
        "name": name,
        "gender": gender,
        "is_living": !died,
    });
    let names: Vec<Value> = aliases
        .iter()
        .filter_map(|n| {
            let parsed = read_name(n);
            if parsed.display.is_empty() {
                return None;
            }
            let mut v = parsed.to_json();
            v["type"] = Value::String(name_type(child_text(n, "TYPE").as_deref()).to_string());
            Some(v)
        })
        .collect();
    if !names.is_empty() {
        identity["names"] = Value::Array(names);
    }

    let mut person = Map::new();
    person.insert("id".into(), Value::String(id.to_string()));
    person.insert("type".into(), Value::String("person".into()));
    person.insert("axgf_version".into(), Value::String("1.0".into()));
    person.insert("identity".into(), identity);

    let mut seen_birth = false;
    let mut seen_death = false;
    let mut notes = Vec::new();
    let mut docs: Vec<Value> = Vec::new();
    let mut doc_ids: BTreeSet<String> = BTreeSet::new();
    for c in &r.children {
        match c.tag.as_str() {
            "BIRT" if !seen_birth => {
                seen_birth = true;
                let b = vital_block(c, id, "birth", bundle, ctx);
                person.insert("birth".into(), b);
            }
            "DEAT" if !seen_death => {
                seen_death = true;
                let mut d = vital_block(c, id, "death", bundle, ctx);
                if let Some(cause) = child_text(c, "CAUS") {
                    d["cause"] = Value::String(cause);
                }
                person.insert("death".into(), d);
            }
            // A second birth or death record is another account of it:
            // an event of its own, so that it is not lost.
            "BIRT" | "DEAT" => {
                let cat = if c.tag == "BIRT" { "birth" } else { "death" };
                person_event(c, id, cat, Some("another record"), bundle, ctx);
            }
            "OCCU" => convert_occu(c, id, &who, bundle, ctx),
            // Checked here; read by the family they name (pass 2).
            "FAMC" | "FAMS" => {
                let _ = ctx.pointer(&c.value, "FAM", &format!("{who}.{}", c.tag));
            }
            "NOTE" => {
                if let Some(n) = note_text(c, ctx) {
                    notes.push(n);
                }
            }
            "SOUR" => {
                if let Some(cit) = read_citation(c, ctx) {
                    notes.push(cit.as_note());
                }
            }
            "OBJE" => {
                let link = document_link(c, bundle, ctx);
                if let Some(l) = link {
                    let did = l["document_id"].as_str().unwrap_or_default().to_string();
                    if doc_ids.insert(did) {
                        docs.push(l);
                    }
                }
            }
            t if INDI_ATTRIBUTES.contains(&t) => {
                let v = c.value.trim();
                let ty = child_text(c, "TYPE");
                if !v.is_empty() {
                    match ty {
                        Some(ty) => notes.push(format!("{t} ({ty}): {v}")),
                        None => notes.push(format!("{t}: {v}")),
                    }
                }
            }
            t => {
                if let Some((cat, sub)) = indi_event(t) {
                    person_event(c, id, cat, sub, bundle, ctx);
                } else if !INDI_HANDLED.contains(&t) {
                    ctx.skip("INDI", t);
                }
            }
        }
    }
    if !notes.is_empty() {
        person.insert("notes".into(), Value::String(notes.join("\n")));
    }
    if !docs.is_empty() {
        person.insert("documents".into(), Value::Array(docs));
    }
    bundle.persons.insert(id.to_string(), Value::Object(person));
}

/// SEX, read as GEDCOM 5.5.1 and 7 write it: M, F, U, and 7's X for
/// someone who is neither only male nor only female — AXGF's NB, with a
/// note saying which letter it was.
fn pick_sex(r: &Record) -> (String, Option<String>) {
    let raw = child(r, "SEX")
        .map(|c| c.value.trim().to_uppercase())
        .unwrap_or_default();
    match raw.as_str() {
        "M" | "MALE" => ("M".into(), None),
        "F" | "FEMALE" => ("F".into(), None),
        "NB" => ("NB".into(), None),
        "X" => ("NB".into(), Some("GEDCOM SEX X".into())),
        _ => ("U".into(), None),
    }
}

/// NAME.TYPE → the type of a further name.
fn name_type(t: Option<&str>) -> &'static str {
    match t.map(|t| t.trim().to_lowercase()).as_deref() {
        Some("birth") | Some("maiden") => "birth",
        Some("married") => "married",
        Some("aka") | None | Some("") => "alias",
        Some("nickname") | Some("nick") => "nickname",
        Some("religious") => "religious",
        Some("pen") | Some("pen_name") | Some("professional") => "pen_name",
        Some(_) => "other",
    }
}

/// A name as GEDCOM writes it: the display, its components, and whether
/// the family name came first.
struct ParsedName {
    display: String,
    components: Vec<Value>,
    family_first: bool,
}

impl ParsedName {
    fn to_json(&self) -> Value {
        let mut v = json!({"display": self.display, "components": self.components});
        if self.family_first {
            v["display_order"] = Value::String("family_first".into());
        }
        v
    }
}

/// What may follow the surname and is not a name: a generational suffix.
fn is_suffix(s: &str) -> bool {
    let t = s.trim().trim_end_matches('.').to_lowercase();
    matches!(
        t.as_str(),
        "jr" | "sr"
            | "jun"
            | "sen"
            | "junior"
            | "senior"
            | "ii"
            | "iii"
            | "iv"
            | "v"
            | "vi"
            | "vii"
            | "viii"
            | "fils"
            | "père"
            | "pere"
            | "aîné"
            | "aine"
            | "cadet"
            | "the elder"
            | "the younger"
    )
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Read a NAME line and its pieces. The slashes mark the surname, as
/// GEDCOM writes it: `Jean /Dupont/`, an unclosed `Jean /Dupont`, two
/// surnames `Jean /Dupont/ /Martin/`, a family-first `/Dupont/ Jean`, a
/// suffix `Jean /Dupont/ Jr.` (kept in the display: the schema has no
/// suffix component). With no value, the GIVN / SURN / NICK / NPFX /
/// SPFX / NSFX pieces make the name.
fn read_name(r: &Record) -> ParsedName {
    let raw = r.value.trim();
    let givn = child_text(r, "GIVN");
    let surn = child_text(r, "SURN");
    let nick = child_text(r, "NICK");
    let spfx = child_text(r, "SPFX");
    let mut components: Vec<Value> = Vec::new();
    let mut push = |kind: &str, v: &str| {
        let v = collapse(&v.replace('/', " "));
        if !v.is_empty() {
            let order = components.len() + 1;
            components.push(json!({"type": kind, "value": v, "order": order}));
        }
    };
    if raw.is_empty() {
        // Pieces only.
        let mut words: Vec<String> = Vec::new();
        for t in ["NPFX", "GIVN"] {
            if let Some(v) = child_text(r, t) {
                words.push(v);
            }
        }
        let family = match (&spfx, &surn) {
            (Some(p), Some(s)) => Some(format!("{p} {s}")),
            (None, Some(s)) => Some(s.clone()),
            _ => None,
        };
        if let Some(f) = &family {
            words.push(f.clone());
        }
        if let Some(v) = child_text(r, "NSFX") {
            words.push(v);
        }
        if let Some(g) = &givn {
            push("given_name", g);
        }
        if let Some(f) = &family {
            push("family_name", f);
        }
        if let Some(n) = &nick {
            push("nickname", n);
        }
        return ParsedName {
            display: collapse(&words.join(" ").replace('/', " ")),
            components,
            family_first: false,
        };
    }
    let display = collapse(&raw.replace('/', " "));
    // Outside / inside the slashes, in order. An odd count of slashes
    // leaves the last surname open to the end of the line.
    let parts: Vec<&str> = raw.split('/').collect();
    let mut before = String::new();
    let mut after = String::new();
    let mut surnames: Vec<String> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        if i % 2 == 1 {
            let s = p.trim();
            if !s.is_empty() {
                surnames.push(s.to_string());
            }
        } else if surnames.is_empty() && i == 0 {
            before = p.trim().to_string();
        } else {
            let t = p.trim();
            if !t.is_empty() {
                if !after.is_empty() {
                    after.push(' ');
                }
                after.push_str(t);
            }
        }
    }
    if parts.len() == 1 {
        // No slash: the whole is the given name.
        push("given_name", raw);
        if let Some(n) = &nick {
            push("nickname", n);
        }
        return ParsedName {
            display,
            components,
            family_first: false,
        };
    }
    let mut family_first = false;
    let given = if !before.is_empty() {
        Some(before.clone())
    } else if !after.is_empty() && !is_suffix(&after) {
        // `/Dupont/ Jean`: the family name first, then the given name.
        family_first = true;
        Some(after.clone())
    } else {
        givn.clone()
    };
    if family_first {
        for s in &surnames {
            push("family_name", s);
        }
        if let Some(g) = &given {
            push("given_name", g);
        }
    } else {
        if let Some(g) = &given {
            push("given_name", g);
        }
        for s in &surnames {
            let s = match &spfx {
                Some(p) if !s.to_lowercase().starts_with(&p.to_lowercase()) => format!("{p} {s}"),
                _ => s.clone(),
            };
            push("family_name", &s);
        }
    }
    if surnames.is_empty() {
        if let Some(s) = &surn {
            push("family_name", s);
        }
    }
    if let Some(n) = &nick {
        push("nickname", n);
    }
    ParsedName {
        display,
        components,
        family_first,
    }
}

/// A NOTE line: its text, or the text of the NOTE record it points at. A
/// value that only looks like a pointer (`@home@`, naming no NOTE record)
/// is the note's own text.
fn note_text(c: &Record, ctx: &ConvertCtx) -> Option<String> {
    let v = c.value.trim();
    if is_pointer(v) {
        if let Some(body) = ctx.note_map.get(v) {
            return Some(body.clone());
        }
    }
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

// =========================================================================
// Citations
// =========================================================================

/// A citation: which source, which page, how sure, and its words.
struct Citation {
    source_id: Option<String>,
    title: Option<String>,
    page: Option<String>,
    text: Option<String>,
    confidence: Option<f64>,
}

impl Citation {
    /// The citation's own words, for an event's description or a
    /// person's notes.
    fn words(&self) -> String {
        let mut out = String::new();
        if let Some(t) = &self.title {
            out.push_str(t);
        }
        if let Some(p) = &self.page {
            if !out.is_empty() {
                out.push_str(", ");
            }
            out.push_str(p);
        }
        if let Some(t) = &self.text {
            if !out.is_empty() {
                out.push_str(" — ");
            }
            out.push_str(t);
        }
        out
    }
    fn as_note(&self) -> String {
        format!("SOUR: {}", self.words())
    }
}

/// QUAY: 0 unreliable, 1 questionable, 2 secondary, 3 direct and primary.
fn quay_confidence(v: &str) -> Option<f64> {
    match v.trim() {
        "0" => Some(0.25),
        "1" => Some(0.5),
        "2" => Some(0.75),
        "3" => Some(0.95),
        _ => None,
    }
}

fn read_citation(c: &Record, ctx: &mut ConvertCtx) -> Option<Citation> {
    let v = c.value.trim();
    let (source_id, title) = if is_pointer(v) {
        let sid = ctx.pointer(v, "SOUR", "a citation");
        (sid, None)
    } else if v.is_empty() {
        (None, None)
    } else {
        // A citation written out, with no SOUR record behind it.
        (None, Some(v.to_string()))
    };
    let page = child_text(c, "PAGE");
    let text = child(c, "DATA")
        .and_then(|d| child_text(d, "TEXT"))
        .or_else(|| child_text(c, "TEXT"));
    let confidence = child_text(c, "QUAY").and_then(|q| quay_confidence(&q));
    if source_id.is_none() && title.is_none() && page.is_none() && text.is_none() {
        return None;
    }
    Some(Citation {
        source_id,
        title,
        page,
        text,
        confidence,
    })
}

fn citations_of(rec: &Record, ctx: &mut ConvertCtx) -> Vec<Citation> {
    rec.children
        .iter()
        .filter(|c| c.tag == "SOUR")
        .filter_map(|c| read_citation(c, ctx))
        .collect()
}

// =========================================================================
// Events
// =========================================================================

/// The date, place and evidence an event line carries, in AXGF's terms.
struct EventParts {
    date: Option<Value>,
    place_id: Option<String>,
    source_id: Option<String>,
    confidence: f64,
    /// What the line says besides: its value, TYPE, AGE, CAUS, notes and
    /// the citations' pages.
    words: Vec<String>,
}

fn event_parts(rec: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) -> EventParts {
    let date = child(rec, "DATE").map(|d| parse_gedcom_date(&d.value));
    let place_id = child(rec, "PLAC").and_then(|p| ensure_place(p, bundle, ctx));
    let cits = citations_of(rec, ctx);
    let source_id = cits.iter().find_map(|c| c.source_id.clone());
    let confidence = cits
        .iter()
        .find_map(|c| c.confidence)
        .unwrap_or(ctx.default_confidence);
    let mut words = Vec::new();
    let v = rec.value.trim();
    if !v.is_empty() && v != "Y" && v != "y" {
        words.push(v.to_string());
    }
    if let Some(a) = child_text(rec, "AGE") {
        words.push(format!("AGE: {a}"));
    }
    if let Some(a) = child_text(rec, "AGNC") {
        words.push(a);
    }
    for c in rec.children.iter().filter(|c| c.tag == "NOTE") {
        if let Some(n) = note_text(c, ctx) {
            words.push(n);
        }
    }
    for c in &cits {
        let w = c.words();
        if !w.is_empty() {
            words.push(format!("SOUR: {w}"));
        }
    }
    EventParts {
        date,
        place_id,
        source_id,
        confidence,
        words,
    }
}

/// A standalone Event for a line of a person's or a family's record.
#[allow(clippy::too_many_arguments)]
fn emit_event(
    category: &str,
    subcategory: Option<String>,
    parts: &EventParts,
    participants: Vec<Value>,
    bundle: &mut FlatBundle,
    ctx: &ConvertCtx,
) -> String {
    let _ = ctx;
    let eid = Uuid::new_v4().to_string();
    let mut ev = Map::new();
    ev.insert("id".into(), Value::String(eid.clone()));
    ev.insert("type".into(), Value::String("event".into()));
    ev.insert("axgf_version".into(), Value::String("1.0".into()));
    ev.insert("category".into(), Value::String(category.to_string()));
    if let Some(s) = subcategory {
        ev.insert("subcategory".into(), Value::String(s));
    }
    // event.date is required by the schema, but axgf_date.value is not —
    // an event with no date is one whose date is unknown.
    ev.insert(
        "date".into(),
        parts
            .date
            .clone()
            .unwrap_or_else(|| json!({"precision": "unknown"})),
    );
    if let Some(p) = &parts.place_id {
        ev.insert("place_id".into(), Value::String(p.clone()));
    }
    ev.insert("participants".into(), Value::Array(participants));
    if !parts.words.is_empty() {
        ev.insert("description".into(), Value::String(parts.words.join("\n")));
    }
    if let Some(s) = &parts.source_id {
        ev.insert("source_id".into(), Value::String(s.clone()));
    }
    ev.insert("confidence".into(), json!(parts.confidence));
    bundle.events.insert(eid.clone(), Value::Object(ev));
    eid
}

fn person_event(
    rec: &Record,
    person_id: &str,
    category: &str,
    sub: Option<&str>,
    bundle: &mut FlatBundle,
    ctx: &mut ConvertCtx,
) {
    let parts = event_parts(rec, bundle, ctx);
    // EVEN's TYPE names it; another event's TYPE says more about it.
    let mut subcategory = sub.map(str::to_string);
    let ty = child_text(rec, "TYPE");
    if rec.tag == "EVEN" {
        subcategory = ty.or(subcategory);
    } else if let Some(t) = ty {
        subcategory = Some(match subcategory {
            Some(s) => format!("{s}: {t}"),
            None => t,
        });
    }
    let who = json!([{"entity_type": "person", "entity_id": person_id, "role": "principal"}]);
    let participants = who.as_array().cloned().unwrap_or_default();
    emit_event(category, subcategory, &parts, participants, bundle, ctx);
}

/// A birth or a death, as the person's own block (`date`, `place_id`,
/// `confidence`, `source_id`). Its evidence — the citation's page and
/// words — needs somewhere to stand: when there is any, an Event of the
/// same category carries it, and the block points at it.
fn vital_block(
    rec: &Record,
    person_id: &str,
    category: &str,
    bundle: &mut FlatBundle,
    ctx: &mut ConvertCtx,
) -> Value {
    let parts = event_parts(rec, bundle, ctx);
    let mut out = Map::new();
    if let Some(d) = &parts.date {
        out.insert("date".into(), d.clone());
    }
    if let Some(p) = &parts.place_id {
        out.insert("place_id".into(), Value::String(p.clone()));
    }
    out.insert("confidence".into(), json!(parts.confidence));
    if let Some(s) = &parts.source_id {
        out.insert("source_id".into(), Value::String(s.clone()));
    }
    if !parts.words.is_empty() {
        let who =
            vec![json!({"entity_type": "person", "entity_id": person_id, "role": "principal"})];
        let eid = emit_event(category, None, &parts, who, bundle, ctx);
        out.insert("event_id".into(), Value::String(eid));
    }
    Value::Object(out)
}

// =========================================================================
// Occupations
// =========================================================================

/// An OCCU line → an Occupation. A bare `1 OCCU` with no value carries no
/// information (the schema's `occupation.title` requires minLength 1):
/// skipped, and said to be. Its DATE is when it was held: a period gives
/// both ends, a single date the start.
fn convert_occu(
    occu: &Record,
    person_id: &str,
    who: &str,
    bundle: &mut FlatBundle,
    ctx: &mut ConvertCtx,
) {
    let title = occu.value.trim();
    if title.is_empty() {
        ctx.warn(format!("skipping empty OCCU tag on person {who}"));
        return;
    }
    let occ_id = Uuid::new_v4().to_string();
    let mut occ = Map::new();
    occ.insert("id".into(), Value::String(occ_id.clone()));
    occ.insert("type".into(), Value::String("occupation".into()));
    occ.insert("axgf_version".into(), Value::String("1.0".into()));
    occ.insert("person_id".into(), Value::String(person_id.to_string()));
    occ.insert("title".into(), Value::String(title.to_string()));
    let cits = citations_of(occu, ctx);
    let confidence = cits
        .iter()
        .find_map(|c| c.confidence)
        .unwrap_or(ctx.default_confidence);
    occ.insert("confidence".into(), json!(confidence));
    if let Some(sid) = cits.iter().find_map(|c| c.source_id.clone()) {
        occ.insert("source_id".into(), Value::String(sid));
    }
    if let Some(date) = child(occu, "DATE") {
        let d = parse_gedcom_date(&date.value);
        // FROM … TO …: when it was held. Any other date (BET, BEF, a
        // single date) says when it was known to be held: its start.
        let period = d
            .get("range")
            .cloned()
            .filter(|r| r.get("note").and_then(Value::as_str) == Some("period"));
        match period {
            Some(range) if range.get("earliest").is_some() || range.get("latest").is_some() => {
                if let Some(e) = range.get("earliest") {
                    occ.insert("valid_from".into(), json!({"date": e}));
                }
                if let Some(l) = range.get("latest") {
                    occ.insert("valid_until".into(), json!({"date": l}));
                }
            }
            _ => {
                occ.insert("valid_from".into(), json!({"date": d}));
            }
        }
    }
    if let Some(place) = child(occu, "PLAC") {
        if let Some(pid) = ensure_place(place, bundle, ctx) {
            occ.insert("place_id".into(), Value::String(pid));
        }
    }
    let mut notes = Vec::new();
    for c in occu.children.iter().filter(|c| c.tag == "NOTE") {
        if let Some(n) = note_text(c, ctx) {
            notes.push(n);
        }
    }
    for c in &cits {
        let w = c.words();
        if !w.is_empty() {
            notes.push(format!("SOUR: {w}"));
        }
    }
    if !notes.is_empty() {
        occ.insert("note".into(), Value::String(notes.join("\n")));
    }
    bundle.occupations.insert(occ_id, Value::Object(occ));
}

// =========================================================================
// Entity conversion — Family
// =========================================================================

/// The family's events besides the marriage, the divorce and the
/// annulment, which shape the union itself.
fn fam_event(tag: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match tag {
        "ENGA" => ("marriage", Some("engagement")),
        "MARB" => ("marriage", Some("banns")),
        "MARC" => ("marriage", Some("contract")),
        "MARL" => ("marriage", Some("licence")),
        "MARS" => ("marriage", Some("settlement")),
        "DIVF" => ("divorce", Some("filing")),
        "CENS" => ("census", None),
        "RESI" => ("other", Some("residence")),
        "EVEN" => ("other", None),
        _ => return None,
    })
}

const FAM_HANDLED: &[&str] = &[
    "HUSB", "WIFE", "CHIL", "MARR", "DIV", "ANUL", "NOTE", "OBJE", "SOUR", "NCHI", "CHAN", "RIN",
    "REFN", "UID", "_UID", "_UPD", "CREA", "SUBM",
];

/// GEDCOM's PEDI → AXGF's lineage. `sealing` and `other` say nothing
/// AXGF can name.
fn lineage(pedi: &str) -> &'static str {
    match pedi.trim().to_lowercase().as_str() {
        "birth" => "biological",
        "adopted" => "adoptive",
        "foster" => "foster",
        _ => "unknown",
    }
}

fn convert_fam(r: &Record, id: &str, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let who = r
        .xref
        .clone()
        .unwrap_or_else(|| "a FAM without a cross-reference".to_string());

    // The spouses: HUSB and WIFE when they name a person, and whoever
    // names this family in their own FAMS.
    let mut spouses: Vec<String> = Vec::new();
    for c in r
        .children
        .iter()
        .filter(|c| c.tag == "HUSB" || c.tag == "WIFE")
    {
        if let Some(pid) = ctx.pointer(&c.value, "INDI", &format!("{who}.{}", c.tag)) {
            if !spouses.contains(&pid) {
                spouses.push(pid);
            }
        }
    }
    let fams = r
        .xref
        .as_ref()
        .and_then(|x| ctx.fams.get(x))
        .cloned()
        .unwrap_or_default();
    for pid in fams {
        if !spouses.contains(&pid) {
            spouses.push(pid);
        }
    }

    // The children, each once: CHIL when it names a person, and whoever
    // names this family in their own FAMC. GEDCOM 5.5.1 puts the
    // pedigree on the child's FAMC; some files put it under CHIL.
    let mut kids: Vec<(String, Option<String>)> = Vec::new();
    for c in r.children.iter().filter(|c| c.tag == "CHIL") {
        let Some(pid) = ctx.pointer(&c.value, "INDI", &format!("{who}.CHIL")) else {
            continue;
        };
        let pedi = child_text(c, "PEDI");
        match kids.iter_mut().find(|(k, _)| *k == pid) {
            Some(k) => k.1 = k.1.take().or(pedi),
            None => kids.push((pid, pedi)),
        }
    }
    let famc = r
        .xref
        .as_ref()
        .and_then(|x| ctx.famc.get(x))
        .cloned()
        .unwrap_or_default();
    for (pid, pedi) in famc {
        match kids.iter_mut().find(|(k, _)| *k == pid) {
            Some(k) => {
                if pedi.is_some() {
                    k.1 = pedi;
                }
            }
            None => kids.push((pid, pedi)),
        }
    }

    // A FAM record with neither spouses nor children carries no
    // family relationship at all (some sources leave stray FAM stubs
    // after deletions). Skip it — the schema requires at least one
    // of `union` or `children` — and say so.
    if spouses.is_empty() && kids.is_empty() {
        ctx.warn(format!(
            "skipping empty FAM record {who} with no spouses and no children"
        ));
        return;
    }

    let mut fam = Map::new();
    fam.insert("id".into(), Value::String(id.to_string()));
    fam.insert("type".into(), Value::String("family".into()));
    fam.insert("axgf_version".into(), Value::String("1.0".into()));

    // Who takes part in the family's events: the spouses, and the family.
    let mut participants: Vec<Value> = spouses
        .iter()
        .map(|p| json!({"entity_type": "person", "entity_id": p, "role": "spouse"}))
        .collect();
    participants.push(json!({"entity_type": "family", "entity_id": id, "role": "created"}));

    let mut union = Map::new();
    let mut notes = Vec::new();
    let mut docs: Vec<Value> = Vec::new();
    let mut doc_ids: BTreeSet<String> = BTreeSet::new();
    for c in &r.children {
        match c.tag.as_str() {
            "MARR" => {
                let parts = event_parts(c, bundle, ctx);
                let sub = child_text(c, "TYPE");
                let eid = emit_event("marriage", sub, &parts, participants.clone(), bundle, ctx);
                // The first marriage is the union's start; a further one
                // is another account of it, or a renewal: an event.
                if !union.contains_key("start") {
                    let mut start = Map::new();
                    if let Some(d) = &parts.date {
                        start.insert("date".into(), d.clone());
                    }
                    if let Some(p) = &parts.place_id {
                        start.insert("place_id".into(), Value::String(p.clone()));
                    }
                    start.insert("event_id".into(), Value::String(eid));
                    union.insert("start".into(), Value::Object(start));
                    if let Some(s) = &parts.source_id {
                        union.insert("source_id".into(), Value::String(s.clone()));
                    }
                }
            }
            "DIV" | "ANUL" => {
                let parts = event_parts(c, bundle, ctx);
                let (category, sub, reason, status) = if c.tag == "DIV" {
                    ("divorce", None, "divorce", "ended_by_divorce")
                } else {
                    (
                        "legal",
                        Some("annulment".to_string()),
                        "annulment",
                        "annulled",
                    )
                };
                emit_event(category, sub, &parts, participants.clone(), bundle, ctx);
                if !union.contains_key("end") {
                    let mut end = Map::new();
                    if let Some(d) = &parts.date {
                        end.insert("date".into(), d.clone());
                    }
                    end.insert("reason".into(), Value::String(reason.into()));
                    union.insert("end".into(), Value::Object(end));
                    union.insert("status".into(), Value::String(status.into()));
                }
            }
            "NOTE" => {
                if let Some(n) = note_text(c, ctx) {
                    notes.push(n);
                }
            }
            "SOUR" => {
                if let Some(cit) = read_citation(c, ctx) {
                    notes.push(cit.as_note());
                }
            }
            "NCHI" => {
                let v = c.value.trim();
                if !v.is_empty() {
                    notes.push(format!("NCHI: {v}"));
                }
            }
            "OBJE" => {
                if let Some(l) = document_link(c, bundle, ctx) {
                    let did = l["document_id"].as_str().unwrap_or_default().to_string();
                    if doc_ids.insert(did) {
                        docs.push(l);
                    }
                }
            }
            t => {
                if let Some((cat, sub)) = fam_event(t) {
                    let parts = event_parts(c, bundle, ctx);
                    let ty = child_text(c, "TYPE");
                    let sub = if t == "EVEN" {
                        ty.or(sub.map(str::to_string))
                    } else {
                        match (sub, ty) {
                            (Some(s), Some(t)) => Some(format!("{s}: {t}")),
                            (s, t) => t.or(s.map(str::to_string)),
                        }
                    };
                    emit_event(cat, sub, &parts, participants.clone(), bundle, ctx);
                } else if !FAM_HANDLED.contains(&t) {
                    ctx.skip("FAM", t);
                }
            }
        }
    }

    // Spec §4.2.3: a FAM with only CHIL entries is a sibling group —
    // parents unknown. Emit the family without a union rather than
    // "a marriage of zero persons". A researcher adds the union later
    // via read-modify-write on update_entity (see docs/API.md Demo E).
    if !spouses.is_empty() {
        union.insert("type".into(), Value::String("marriage".into()));
        let persons: Vec<Value> = spouses
            .iter()
            .map(|p| json!({"person_id": p, "role": "spouse"}))
            .collect();
        union.insert("persons".into(), Value::Array(persons));
        fam.insert("union".into(), Value::Object(union));
    } else if union.contains_key("end") || union.contains_key("start") {
        ctx.warn(format!(
            "{who} records a marriage or its end with no spouse: kept as events only"
        ));
    }

    if !kids.is_empty() {
        let mut lineage_used = false;
        let children: Vec<Value> = kids
            .iter()
            .enumerate()
            .map(|(i, (pid, pedi))| {
                let mut entry = json!({"person_id": pid, "birth_order": i + 1});
                if let Some(p) = pedi {
                    entry["lineage"] = Value::String(lineage(p).to_string());
                    entry["note"] = Value::String(format!("pedigree: {p}"));
                    lineage_used = true;
                }
                entry
            })
            .collect();
        fam.insert("children".into(), Value::Array(children));
        // `lineage` is AXGF 1.1's: the family says so.
        if lineage_used {
            fam.insert("axgf_version".into(), Value::String("1.1".into()));
        }
    }
    if !notes.is_empty() {
        fam.insert("notes".into(), Value::String(notes.join("\n")));
    }
    if !docs.is_empty() {
        fam.insert("documents".into(), Value::Array(docs));
    }
    bundle.families.insert(id.to_string(), Value::Object(fam));
}

// =========================================================================
// Entity conversion — Source
// =========================================================================

const SOUR_HANDLED: &[&str] = &[
    "TITL", "ABBR", "AUTH", "PUBL", "TEXT", "NOTE", "REPO", "DATA", "CHAN", "RIN", "REFN", "UID",
    "_UID", "_UPD", "CREA", "OBJE",
];

fn convert_sour(r: &Record, id: &str, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) {
    let title = child_text(r, "TITL")
        .or_else(|| child_text(r, "ABBR"))
        .or_else(|| Some(r.value.trim().to_string()).filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "Untitled source".to_string());

    let mut src = Map::new();
    src.insert("id".into(), Value::String(id.to_string()));
    src.insert("type".into(), Value::String("source".into()));
    src.insert("axgf_version".into(), Value::String("1.0".into()));
    src.insert("title".into(), Value::String(title));
    src.insert("source_type".into(), Value::String("other".into()));
    src.insert("reliability".into(), Value::String("unknown".into()));
    src.insert("confidence".into(), json!(ctx.default_confidence));
    let mut notes = Vec::new();
    if let Some(a) = child_text(r, "AUTH") {
        notes.push(format!("AUTH: {a}"));
    }
    if let Some(p) = child_text(r, "PUBL") {
        notes.push(format!("PUBL: {p}"));
    }
    for c in r.children.iter().filter(|c| c.tag == "NOTE") {
        if let Some(n) = note_text(c, ctx) {
            notes.push(n);
        }
    }
    if let Some(t) = child_text(r, "TEXT") {
        src.insert("transcription".into(), Value::String(t));
    }
    if let Some(repo) = child(r, "REPO") {
        let v = repo.value.trim();
        let name = if is_pointer(v) {
            ctx.repos.get(v).cloned()
        } else {
            Some(v.to_string()).filter(|v| !v.is_empty())
        };
        if let Some(name) = name {
            let mut rep = json!({"name": name});
            if let Some(call) = child_text(repo, "CALN") {
                rep["reference"] = Value::String(call);
            }
            src.insert("repository".into(), rep);
        }
    }
    if let Some(o) = r.children.iter().find(|c| c.tag == "OBJE") {
        if let Some(l) = document_link(o, bundle, ctx) {
            if let Some(d) = l.get("document_id") {
                src.insert("document_id".into(), d.clone());
            }
        }
    }
    for c in &r.children {
        if !SOUR_HANDLED.contains(&c.tag.as_str()) {
            ctx.skip("SOUR", &c.tag);
        }
    }
    if !notes.is_empty() {
        src.insert("note".into(), Value::String(notes.join("\n")));
    }
    bundle.sources.insert(id.to_string(), Value::Object(src));
}

// =========================================================================
// Documents
// =========================================================================

/// An OBJE line under a person, a family or a source: a link to a
/// top-level OBJE record, or an inline one made a Document. Nothing when
/// it points at nothing or holds nothing.
fn document_link(c: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) -> Option<Value> {
    let v = c.value.trim();
    if is_pointer(v) {
        let id = ctx.pointer(v, "OBJE", "an OBJE link")?;
        return Some(json!({"document_id": id, "role": "attachment"}));
    }
    if c.children.is_empty() {
        return None;
    }
    let doc_id = Uuid::new_v4().to_string();
    let doc = obje_to_document(&doc_id, c);
    bundle.documents.insert(doc_id.clone(), doc);
    Some(json!({"document_id": doc_id, "role": "attachment"}))
}

fn convert_obje_top(r: &Record, id: &str, bundle: &mut FlatBundle) {
    let doc = obje_to_document(id, r);
    bundle.documents.insert(id.to_string(), doc);
}

/// Build a Document entity from an OBJE record. Handles the two
/// common GEDCOM layouts:
///
///   1 OBJE                          1 OBJE
///   2 FORM jpg                      2 FILE documents/file.jpg
///   2 TITL Photo                    3 FORM jpg
///   2 FILE documents/file.jpg       3 TITL Photo
///
/// A file is named by the last segment of its path, whichever way its
/// separators lean (`C:\Photos\portrait.jpg` names `portrait.jpg`).
fn obje_to_document(id: &str, r: &Record) -> Value {
    let mut form: Option<String> = None;
    let mut title: Option<String> = None;
    let mut file_path: Option<String> = None;

    for c in &r.children {
        match c.tag.as_str() {
            "FORM" => form = Some(c.value.clone()),
            "TITL" => title = Some(c.value.clone()),
            "FILE" => {
                file_path = Some(c.value.trim().to_string());
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
    let filename = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
    let ext = filename
        .rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
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
    if let Some(t) = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
    {
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
// Places
// =========================================================================

/// A place's jurisdictions as written, each trimmed and its spacing
/// collapsed, the empty ones (GEDCOM's positional `, , Paris`) dropped.
fn place_parts(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(collapse)
        .filter(|p| !p.is_empty())
        .collect()
}

/// `N47.9960` / `S…` / `E…` / `W…`, or a signed number.
fn coordinate(v: &str, pos: char, neg: char) -> Option<f64> {
    let v = v.trim();
    let first = v.chars().next()?.to_ascii_uppercase();
    let (sign, rest) = if first == pos {
        (1.0, &v[1..])
    } else if first == neg {
        (-1.0, &v[1..])
    } else {
        (1.0, v)
    };
    rest.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .map(|x| x * sign)
        .filter(|x| x.is_finite())
}

/// The place a PLAC line names, made once however it is spaced or cased.
/// An empty PLAC names no place. `MAP` / `LATI` / `LONG` give its
/// coordinates.
fn ensure_place(plac: &Record, bundle: &mut FlatBundle, ctx: &mut ConvertCtx) -> Option<String> {
    let parts = place_parts(&plac.value);
    if parts.is_empty() {
        return None;
    }
    let display = parts.join(", ");
    let key = parts
        .iter()
        .map(|p| p.to_lowercase())
        .collect::<Vec<_>>()
        .join(",");
    let coords = child(plac, "MAP").and_then(|m| {
        let lat = child_text(m, "LATI").and_then(|v| coordinate(&v, 'N', 'S'));
        let lon = child_text(m, "LONG").and_then(|v| coordinate(&v, 'E', 'W'));
        match (lat, lon) {
            (Some(lat), Some(lon)) if lat.abs() <= 90.0 && lon.abs() <= 180.0 => {
                Some(json!({"lat": lat, "lon": lon}))
            }
            _ => None,
        }
    });
    if child(plac, "MAP").is_some() && coords.is_none() {
        ctx.warn(format!(
            "the coordinates of {display:?} cannot be read; left out"
        ));
    }
    if let Some(existing) = ctx.place_dedup.get(&key).cloned() {
        // The same place met again: its coordinates, when the first
        // mention had none.
        if let (Some(c), Some(Value::Object(p))) = (coords, bundle.places.get_mut(&existing)) {
            p.entry("coordinates").or_insert(c);
        }
        return Some(existing);
    }
    let id = Uuid::new_v4().to_string();
    let mut place = Map::new();
    place.insert("id".into(), Value::String(id.clone()));
    place.insert("type".into(), Value::String("place".into()));
    place.insert("axgf_version".into(), Value::String("1.0".into()));
    place.insert(
        "names".into(),
        json!([{"lang": ctx.place_lang, "value": display, "is_primary": true}]),
    );
    if let Some(c) = coords {
        place.insert("coordinates".into(), c);
    }
    bundle.places.insert(id.clone(), Value::Object(place));
    ctx.place_dedup.insert(key, id.clone());
    Some(id)
}

// =========================================================================
// Date parsing — calendars, qualifiers in four languages
// =========================================================================

/// Parse a GEDCOM DATE value into an AXGF `axgf_date` object. Never
/// fails: unparseable dates are preserved verbatim in the `note`
/// field with `precision: "unknown"` and no `value`.
///
/// Range shapes (BEF/AFT/BET…AND/FROM…TO) follow schema
/// `$defs.axgf_date`: the outer date carries `precision: "unknown"` and
/// no `value`; the `range` object holds `earliest` and/or `latest` as
/// nested full `axgf_date` objects (§5.2.3).
///
/// A date in another calendar keeps that calendar's reckoning in `value`
/// (`@#DHEBREW@ 1 TSH 5600` → `5600-01-01`, `calendar: hebrew`): AXGF's
/// `calendar` names the calendar the value is written in, and nothing is
/// converted to Gregorian.
fn parse_gedcom_date(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({"precision": "unknown"});
    }
    // INT <date> (<phrase>): the date, and the words it was read from.
    if let Some(rest) = strip_prefix_ci(trimmed, "INT ") {
        let (date_part, phrase) = match rest.find('(') {
            Some(i) => (
                &rest[..i],
                Some(rest[i + 1..].trim_end().trim_end_matches(')').trim()),
            ),
            None => (rest, None),
        };
        let mut d = parse_gedcom_date(date_part);
        if let Some(p) = phrase.filter(|p| !p.is_empty()) {
            add_note(&mut d, p);
        }
        return d;
    }
    // A date phrase alone: words, no date.
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        if inner.is_empty() {
            return json!({"precision": "unknown"});
        }
        return json!({"precision": "unknown", "note": inner});
    }
    let upper = normalize_qualifier(&canon_escapes(trimmed));

    // Ranges.
    if let Some(rest) = strip_prefix_ci(&upper, "BET ") {
        if let Some((a, b)) = split_on(rest, " AND ") {
            return json!({
                "precision": "unknown",
                "range": {"earliest": bound_date(&a), "latest": bound_date(&b)}
            });
        }
    }
    if let Some(rest) = strip_prefix_ci(&upper, "FROM ") {
        return match split_on(rest, " TO ") {
            Some((a, b)) => json!({
                "precision": "unknown",
                "range": {"earliest": bound_date(&a), "latest": bound_date(&b), "note": "period"}
            }),
            None => json!({
                "precision": "unknown",
                "range": {"earliest": bound_date(rest), "note": "period"}
            }),
        };
    }
    if let Some(rest) = strip_prefix_ci(&upper, "TO ") {
        return json!({
            "precision": "unknown",
            "range": {"latest": bound_date(rest), "note": "period"}
        });
    }
    if let Some(rest) = strip_prefix_ci(&upper, "BEF ") {
        return json!({"precision": "unknown", "range": {"latest": bound_date(rest)}});
    }
    if let Some(rest) = strip_prefix_ci(&upper, "AFT ") {
        return json!({"precision": "unknown", "range": {"earliest": bound_date(rest)}});
    }
    // Approximations: about, calculated, estimated. The date keeps its
    // own precision; `circa` says it is approximate.
    for (q, how) in [
        ("ABT ", None),
        ("CAL ", Some("calculated")),
        ("EST ", Some("estimated")),
    ] {
        if let Some(rest) = strip_prefix_ci(&upper, q) {
            let mut d = bound_date(rest);
            if d.get("value").is_some() {
                d["circa"] = Value::Bool(true);
                if let Some(h) = how {
                    add_note(&mut d, h);
                }
            }
            return d;
        }
    }
    let mut d = bound_date(&upper);
    if d.get("note").and_then(Value::as_str) == Some(upper.trim()) {
        // Unparseable — preserve the words as written.
        d["note"] = Value::String(trimmed.to_string());
    }
    d
}

fn add_note(d: &mut Value, note: &str) {
    let joined = match d.get("note").and_then(Value::as_str) {
        Some(n) if !n.is_empty() => format!("{n}; {note}"),
        _ => note.to_string(),
    };
    d["note"] = Value::String(joined);
}

/// `@#DFRENCH R@` → `@#DFRENCHR@`: a calendar escape as one word, so the
/// qualifiers' reading cannot take it apart.
fn canon_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("@#") {
        out.push_str(&rest[..i]);
        match rest[i + 2..].find('@') {
            Some(j) => {
                let name: String = rest[i + 2..i + 2 + j]
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                out.push_str("@#");
                out.push_str(&name.to_uppercase());
                out.push('@');
                rest = &rest[i + 2 + j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Uppercase, and read a qualifier written in Polish, French or German
/// as GEDCOM's English one. Word by word: a qualifier is a word of its
/// own, never part of a month's name (`BRUM` is not `BR` + `UM`).
fn normalize_qualifier(s: &str) -> String {
    let words: Vec<String> = s.split_whitespace().map(str::to_uppercase).collect();
    let mut out: Vec<&str> = Vec::with_capacity(words.len());
    let last = words.len().saturating_sub(1);
    for (i, w) in words.iter().enumerate() {
        let first = i == 0;
        let w = w.as_str();
        let mapped = match w {
            "PRZED" | "AVANT" | "AV" | "AV." | "VOR" if first => "BEF",
            "OK" | "OK." | "OKOŁO" | "VERS" | "ENV" | "ENV." | "UM" | "CA" | "CA." | "CIRCA"
                if first =>
            {
                "ABT"
            }
            "PO" | "APRÈS" | "APRES" | "AP" | "AP." | "NACH" if first => "AFT",
            "MIĘDZY" | "ENTRE" | "ZWISCHEN" if first => "BET",
            "I" | "ET" | "UND" if !first => "AND",
            // Polish year suffix: "1920 r." / "1920 roku".
            "R" | "R." | "ROKU" if !first && i == last => continue,
            _ => w,
        };
        out.push(mapped);
    }
    out.join(" ")
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix)
    {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn split_on(s: &str, sep: &str) -> Option<(String, String)> {
    let up = s.to_uppercase();
    let idx = up.find(sep)?;
    if !s.is_char_boundary(idx) || !s.is_char_boundary(idx + sep.len()) {
        return None;
    }
    let a = s[..idx].trim().to_string();
    let b = s[idx + sep.len()..].trim().to_string();
    if a.is_empty() || b.is_empty() {
        None
    } else {
        Some((a, b))
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Calendar {
    Gregorian,
    Julian,
    Hebrew,
    FrenchRepublican,
    Roman,
}

impl Calendar {
    fn name(self) -> &'static str {
        match self {
            Calendar::Gregorian => "gregorian",
            Calendar::Julian => "julian",
            Calendar::Hebrew => "hebrew",
            Calendar::FrenchRepublican => "republican_french",
            Calendar::Roman => "roman",
        }
    }

    /// A calendar escape (`@#DJULIAN@`) or GEDCOM 7's calendar word.
    fn from_word(w: &str) -> Option<Option<Calendar>> {
        let w = w
            .trim_start_matches("@#D")
            .trim_start_matches("@#")
            .trim_end_matches('@');
        Some(match w {
            "GREGORIAN" => Some(Calendar::Gregorian),
            "JULIAN" => Some(Calendar::Julian),
            "HEBREW" => Some(Calendar::Hebrew),
            "FRENCHR" | "FRENCH_R" => Some(Calendar::FrenchRepublican),
            "ROMAN" => Some(Calendar::Roman),
            "UNKNOWN" => None,
            _ => return None,
        })
    }

    fn month(self, m: &str) -> Option<u32> {
        match self {
            Calendar::Hebrew => hebrew_month(m),
            Calendar::FrenchRepublican => french_republican_month(m),
            _ => month_number(m),
        }
    }

    /// The days a month has, or the most it may have when that depends
    /// on the year in ways this converter does not reckon.
    fn days_in_month(self, year: i64, month: u32) -> u32 {
        match self {
            Calendar::Hebrew => 30,
            Calendar::FrenchRepublican => {
                if month == 13 {
                    6
                } else {
                    30
                }
            }
            Calendar::Gregorian | Calendar::Julian | Calendar::Roman => {
                let leap = if self == Calendar::Gregorian {
                    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
                } else {
                    year % 4 == 0
                };
                match month {
                    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                    4 | 6 | 9 | 11 => 30,
                    _ if leap => 29,
                    _ => 28,
                }
            }
        }
    }

    fn months(self) -> u32 {
        match self {
            Calendar::Hebrew | Calendar::FrenchRepublican => 13,
            _ => 12,
        }
    }
}

/// A year as GEDCOM writes it, with its dual form (`1750/51`, before the
/// year began in January): the later year, and the words to say so.
fn read_year(s: &str, cal: Calendar) -> Option<(i64, Option<String>)> {
    if let Some((y, alt)) = s.split_once('/') {
        if cal != Calendar::Gregorian || alt.is_empty() || alt.len() > y.len() {
            return None;
        }
        let year: i64 = y.parse().ok()?;
        if !alt.chars().all(|c| c.is_ascii_digit()) || !y.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let modulus = 10i64.pow(alt.len() as u32);
        let mut later = year - year.rem_euclid(modulus) + alt.parse::<i64>().ok()?;
        if later < year {
            later += modulus;
        }
        return Some((later, Some(format!("dual year {s}"))));
    }
    let digits = s.strip_prefix('-').unwrap_or(s);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((s.parse().ok()?, None))
}

/// One date: `[calendar] [[day] month] year [B.C.]`.
fn bound_date(raw: &str) -> Value {
    let unknown = || json!({"precision": "unknown", "note": raw.trim().to_string()});
    let mut words: Vec<&str> = raw.split_whitespace().collect();
    let mut cal = Calendar::Gregorian;
    if let Some(first) = words.first() {
        if first.starts_with("@#")
            || matches!(*first, "GREGORIAN" | "JULIAN" | "HEBREW" | "FRENCH_R")
        {
            match Calendar::from_word(first) {
                Some(Some(c)) => cal = c,
                // A calendar AXGF has no name for, or one GEDCOM calls
                // unknown: the words, as written.
                _ => return unknown(),
            }
            words.remove(0);
        }
    }
    if matches!(words.last(), Some(&"B.C." | &"BC" | &"BCE")) {
        // Before the common era: no ISO 8601 year the schema accepts.
        return unknown();
    }
    let (day, month, year_word) = match words.as_slice() {
        [d, m, y] => (Some(*d), Some(*m), *y),
        [m, y] => (None, Some(*m), *y),
        [y] => (None, None, *y),
        _ => return unknown(),
    };
    let Some((year, dual)) = read_year(year_word, cal) else {
        return unknown();
    };
    if !(0..=9999).contains(&year) {
        let mut d = unknown();
        add_note(&mut d, "a year outside 0–9999");
        return d;
    }
    let month = match month {
        Some(m) => match cal.month(m) {
            Some(n) if n <= cal.months() => Some(n),
            _ => return unknown(),
        },
        None => None,
    };
    let mut out = Map::new();
    let mut notes: Vec<String> = Vec::new();
    match (day, month) {
        (Some(d), Some(m)) => {
            let day: Option<u32> = d.parse().ok();
            match day {
                Some(day) if day >= 1 && day <= cal.days_in_month(year, m) => {
                    out.insert("value".into(), json!(format!("{year:04}-{m:02}-{day:02}")));
                    out.insert("precision".into(), json!("exact"));
                }
                _ => {
                    // No such day: the month stands, the day is said.
                    out.insert("value".into(), json!(format!("{year:04}-{m:02}")));
                    out.insert("precision".into(), json!("month"));
                    notes.push(format!("no day {d} in that month ({})", raw.trim()));
                }
            }
        }
        (None, Some(m)) => {
            out.insert("value".into(), json!(format!("{year:04}-{m:02}")));
            out.insert("precision".into(), json!("month"));
        }
        _ => {
            out.insert("value".into(), json!(format!("{year:04}")));
            out.insert("precision".into(), json!("year"));
        }
    }
    if cal != Calendar::Gregorian {
        out.insert("calendar".into(), json!(cal.name()));
    }
    if let Some(d) = dual {
        notes.push(d);
    }
    if !notes.is_empty() {
        out.insert("note".into(), json!(notes.join("; ")));
    }
    Value::Object(out)
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

/// GEDCOM's Hebrew months, Tishri first; Adar Sheni (ADS) is the
/// thirteenth of a leap year.
fn hebrew_month(m: &str) -> Option<u32> {
    Some(match m {
        "TSH" => 1,
        "CSH" => 2,
        "KSL" => 3,
        "TVT" => 4,
        "SHV" => 5,
        "ADR" => 6,
        "ADS" => 7,
        "NSN" => 8,
        "IYR" => 9,
        "SVN" => 10,
        "TMZ" => 11,
        "AAV" => 12,
        "ELL" => 13,
        _ => return None,
    })
}

/// GEDCOM's French Republican months, Vendémiaire first; COMP is the
/// complementary days.
fn french_republican_month(m: &str) -> Option<u32> {
    Some(match m {
        "VEND" => 1,
        "BRUM" => 2,
        "FRIM" => 3,
        "NIVO" => 4,
        "PLUV" => 5,
        "VENT" => 6,
        "GERM" => 7,
        "FLOR" => 8,
        "PRAI" => 9,
        "MESS" => 10,
        "THER" => 11,
        "FRUC" => 12,
        "COMP" => 13,
        _ => return None,
    })
}

#[cfg(test)]
mod date_shape_tests {
    use super::parse_gedcom_date;
    use serde_json::json;

    // Schema contract (§5.2.1, §5.2.3): ranged dates have
    // `precision: "unknown"`, no top-level `value`, and use
    // `earliest` / `latest` inside `range` (each a full axgf_date).

    #[test]
    fn bet_and_produces_earliest_and_latest() {
        let d = parse_gedcom_date("BET 1920 AND 1925");
        assert_eq!(
            d,
            json!({
                "precision": "unknown",
                "range": {
                    "earliest": {"value": "1920", "precision": "year"},
                    "latest":   {"value": "1925", "precision": "year"}
                }
            })
        );
    }

    #[test]
    fn bef_produces_latest_only() {
        let d = parse_gedcom_date("BEF 1990");
        assert_eq!(
            d,
            json!({
                "precision": "unknown",
                "range": {"latest": {"value": "1990", "precision": "year"}}
            })
        );
    }

    #[test]
    fn aft_produces_earliest_only() {
        let d = parse_gedcom_date("AFT 2000");
        assert_eq!(
            d,
            json!({
                "precision": "unknown",
                "range": {"earliest": {"value": "2000", "precision": "year"}}
            })
        );
    }

    #[test]
    fn polish_between_maps_to_earliest_latest() {
        // MIĘDZY 1920 I 1925 → BET 1920 AND 1925
        let d = parse_gedcom_date("MIĘDZY 1920 I 1925");
        assert_eq!(d["precision"], "unknown");
        assert_eq!(d["range"]["earliest"]["value"], "1920");
        assert_eq!(d["range"]["latest"]["value"], "1925");
    }

    #[test]
    fn unparseable_omits_value_but_keeps_note() {
        let d = parse_gedcom_date("bogus-date-value");
        assert_eq!(d["precision"], "unknown");
        assert!(
            d.get("value").is_none(),
            "unparseable dates must omit `value` (schema types it as string)"
        );
        assert_eq!(d["note"], "bogus-date-value");
    }

    #[test]
    fn empty_input_omits_value() {
        let d = parse_gedcom_date("");
        assert_eq!(d["precision"], "unknown");
        assert!(d.get("value").is_none());
        assert!(d.get("note").is_none());
    }

    #[test]
    fn month_bounds_carry_finer_precision() {
        // Ensure `bound_date` picks up richer precision when parseable.
        let d = parse_gedcom_date("BET APR 1920 AND 12 MAY 1925");
        assert_eq!(d["range"]["earliest"]["value"], "1920-04");
        assert_eq!(d["range"]["earliest"]["precision"], "month");
        assert_eq!(d["range"]["latest"]["value"], "1925-05-12");
        assert_eq!(d["range"]["latest"]["precision"], "exact");
    }
}
