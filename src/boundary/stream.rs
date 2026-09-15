// SPDX-License-Identifier: Apache-2.0
//! # stream — payload-at-a-time import and export
//!
//! [`crate::import_bundle`] and [`crate::export_bundle`] both hold every
//! binary payload in memory at once, base64-encoded inside the flat JSON.
//! Base64 inflates by a third, and encoded and decoded forms coexist while
//! the map is built, so a bundle with 417 MB of media costs upward of 1.5 GB
//! to open and considerably more to save. That is fine for a bundle of
//! scanned certificates and untenable for one carrying photographs.
//!
//! This module adds a second path that never materialises more than one
//! payload at a time. It does not replace the simple path: a caller who wants
//! the whole bundle as one JSON value still gets exactly what they got
//! before.
//!
//! # The shape of it
//!
//! On import, the textual half — manifest, the eight entity collections,
//! document *metadata* — is small and bounded by the size of the tree, so it
//! is returned whole as usual. Payloads are handed to a caller-supplied
//! closure one at a time as a [`Payload`], which is a [`Read`]: the caller
//! copies it wherever it belongs and the library forgets it before opening
//! the next.
//!
//! On export the direction reverses. The library walks the payloads the
//! bundle declares, hands the caller a [`PayloadSlot`] — a [`Write`] — for
//! each one in turn, and streams whatever is written straight into the ZIP.
//!
//! Peak memory on both sides is bounded by the largest single payload if the
//! caller buffers one whole (via [`Payload::read_to_end`]), and by the copy
//! buffer if they do not (via [`Payload::copy_to`]). It is never bounded by
//! the sum.
//!
//! # Why closures rather than an iterator
//!
//! A [`zip::ZipArchive`] lends each entry from `&mut self`, so successive
//! entries cannot coexist. An `Iterator` yielding payloads would therefore
//! need either a lending-iterator abstraction or self-referential borrows.
//! Inverting the control flow avoids the problem entirely and keeps the
//! library free of hidden state: nothing is retained between calls, and there
//! are no handles for a caller to leak.
//!
//! # Losing media is made impossible, not merely unlikely
//!
//! A caller who streams payloads out and then reaches for the familiar
//! [`crate::export_bundle`] would otherwise write a structurally valid bundle
//! with every photograph silently missing. To stop that, a streaming import
//! records what it streamed in
//! [`crate::boundary::flat::FlatBundle::external_payloads`], and
//! [`crate::export_bundle`] refuses any bundle carrying that marker with a
//! `PAYLOADS_EXTERNAL` diagnostic. The failure is loud, it names the
//! function to use instead, and it happens before anything is written.

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, Write};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde_json::{json, Value};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::boundary::envelope::{Diagnostic, DiagnosticCode, Envelope, Severity};
use crate::boundary::flat::{ExternalPayload, FlatBundle};
use crate::boundary::lifecycle::{
    check_manifest_version, compute_stats, now_iso8601_utc, parse_flat, raise_manifest_version,
    read_json, schema_entry, split_entity_path,
};

/// Size of the buffer used by [`Payload::copy_to`] and
/// [`PayloadSlot::write_all_from`]. Large enough that syscall overhead is
/// irrelevant next to decompression, small enough to be invisible in a
/// memory budget.
const COPY_BUF: usize = 64 * 1024;

// -------------------------------------------------------------------------
// Payload — one incoming payload, borrowed for the duration of one callback
// -------------------------------------------------------------------------

/// A single payload being read out of a bundle, handed to the closure passed
/// to [`import_bundle_streaming`].
///
/// The bytes are **not** buffered: this is a live reader positioned at the
/// start of the entry. Read it however suits — [`Payload::copy_to`] to send
/// it straight to a file without ever holding it whole,
/// [`Payload::read_to_end`] to take it as a `Vec`, or the [`Read`] impl for
/// anything else. Whatever is not read is skipped when the callback returns.
pub struct Payload<'a> {
    path: String,
    size: u64,
    crc32: u32,
    reader: &'a mut dyn Read,
}

impl Payload<'_> {
    /// The payload's path inside the archive, e.g.
    /// `documents/files/<uuid>.jpg`. This is the same key the payload has in
    /// [`crate::boundary::flat::FlatBundle::attachments`] on the
    /// non-streaming path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Uncompressed length in bytes, from the archive's central directory —
    /// known before a single byte is decompressed, so a caller can decide
    /// what to do with a payload based on its size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// CRC-32 of the uncompressed bytes, from the archive's central
    /// directory.
    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    /// Copy the payload into `dest`, using a fixed 64 KiB buffer.
    ///
    /// This is the bounded-memory path: peak cost is the buffer, not the
    /// payload, however large the payload is. Returns the number of bytes
    /// copied.
    pub fn copy_to<W: Write>(&mut self, mut dest: W) -> io::Result<u64> {
        let mut buf = vec![0u8; COPY_BUF];
        let mut total = 0u64;
        loop {
            let n = self.reader.read(&mut buf)?;
            if n == 0 {
                return Ok(total);
            }
            dest.write_all(&buf[..n])?;
            total += n as u64;
        }
    }

    /// Read the whole payload into memory.
    ///
    /// Convenient, and bounded by this one payload rather than by all of
    /// them — but [`Payload::copy_to`] is better when the destination is a
    /// file, because it never holds the payload whole at all.
    pub fn read_to_end(&mut self) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.size as usize);
        self.reader.read_to_end(&mut out)?;
        Ok(out)
    }
}

impl Read for Payload<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

// -------------------------------------------------------------------------
// PayloadSlot — one outgoing payload, borrowed for the duration of one callback
// -------------------------------------------------------------------------

/// A slot in the archive being written, handed to the closure passed to
/// [`export_bundle_streaming`].
///
/// The ZIP entry is already open: everything written here is compressed and
/// appended as it arrives, so the payload never exists whole in memory unless
/// the caller chooses to build it that way.
pub struct PayloadSlot<'a> {
    path: String,
    expected_size: u64,
    expected_crc32: u32,
    written: u64,
    sink: &'a mut dyn Write,
}

impl PayloadSlot<'_> {
    /// The path the library is asking for, e.g.
    /// `documents/files/<uuid>.jpg`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The size recorded for this payload when the bundle was imported, or
    /// `0` if unknown. Advisory: the caller may legitimately supply
    /// different bytes, which is reported as a warning rather than refused.
    pub fn expected_size(&self) -> u64 {
        self.expected_size
    }

    /// The CRC-32 recorded for this payload when the bundle was imported, or
    /// `0` if unknown.
    pub fn expected_crc32(&self) -> u32 {
        self.expected_crc32
    }

    /// Bytes written into this slot so far.
    pub fn written(&self) -> u64 {
        self.written
    }

    /// Copy everything from `src` into the slot with a fixed 64 KiB buffer
    /// — the bounded-memory counterpart to [`Payload::copy_to`].
    pub fn write_all_from<R: Read>(&mut self, mut src: R) -> io::Result<u64> {
        let mut buf = vec![0u8; COPY_BUF];
        let mut total = 0u64;
        loop {
            let n = src.read(&mut buf)?;
            if n == 0 {
                return Ok(total);
            }
            self.write_all(&buf[..n])?;
            total += n as u64;
        }
    }
}

impl Write for PayloadSlot<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.sink.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }
}

// -------------------------------------------------------------------------
// import
// -------------------------------------------------------------------------

/// See [`crate::import_bundle_textual`].
pub fn import_bundle_textual<R: Read + Seek>(src: R) -> Envelope {
    import_inner(src, |_| Ok(()))
}

/// See [`crate::import_bundle_streaming`].
pub fn import_bundle_streaming<R, F>(src: R, on_payload: F) -> Envelope
where
    R: Read + Seek,
    F: FnMut(&mut Payload<'_>) -> io::Result<()>,
{
    import_inner(src, on_payload)
}

/// The one implementation behind both import entry points. `on_payload` is
/// invoked once per payload; passing a closure that reads nothing yields the
/// textual-only behaviour, because an unread entry is simply skipped.
fn import_inner<R, F>(src: R, mut on_payload: F) -> Envelope
where
    R: Read + Seek,
    F: FnMut(&mut Payload<'_>) -> io::Result<()>,
{
    let mut archive = match ZipArchive::new(src) {
        Ok(a) => a,
        Err(e) => {
            return Envelope::error(
                DiagnosticCode::ZipReadError,
                format!("cannot open ZIP: {e}"),
            );
        }
    };

    let mut bundle = FlatBundle::default();
    let mut manifest_seen = false;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                return Envelope::error(
                    DiagnosticCode::ZipReadError,
                    format!("cannot read ZIP entry #{i}: {e}"),
                );
            }
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();

        if name == "manifest.json" {
            match read_json(&mut entry) {
                Ok(v) => {
                    bundle.manifest = v;
                    manifest_seen = true;
                }
                Err(e) => return e,
            }
            continue;
        }

        // schema/** is dropped on import; export writes a fresh embedded copy.
        if name.starts_with("schema/") {
            continue;
        }

        if let Some((collection, id)) = split_entity_path(&name) {
            let target = match collection {
                "persons" => &mut bundle.persons,
                "families" => &mut bundle.families,
                "events" => &mut bundle.events,
                "links" => &mut bundle.links,
                "occupations" => &mut bundle.occupations,
                "sources" => &mut bundle.sources,
                "places" => &mut bundle.places,
                _ => unreachable!("split_entity_path only yields the seven per-file dirs"),
            };
            let id = id.to_string();
            match read_json(&mut entry) {
                Ok(v) => {
                    target.insert(id, v);
                    continue;
                }
                Err(e) => return e,
            }
        }

        if name == "documents/index.json" {
            match read_json(&mut entry) {
                Ok(Value::Object(map)) => {
                    for (k, v) in map {
                        bundle.documents.insert(k, v);
                    }
                    continue;
                }
                Ok(_) => {
                    return Envelope::error(
                        DiagnosticCode::InvalidBundleStructure,
                        "documents/index.json is not a JSON object",
                    );
                }
                Err(e) => return e,
            }
        }

        // Anything else is a payload: document files, vault pages, and any
        // path this build does not recognise. Record it, then let the caller
        // take the bytes. Size and CRC come from the central directory, so
        // they are known without decompressing.
        let size = entry.size();
        let crc32 = entry.crc32();
        bundle.external_payloads.insert(
            name.clone(),
            ExternalPayload {
                size_bytes: size,
                crc32,
                extra: BTreeMap::new(),
            },
        );
        let mut payload = Payload {
            path: name,
            size,
            crc32,
            reader: &mut entry,
        };
        if let Err(e) = on_payload(&mut payload) {
            let path = payload.path.clone();
            return Envelope::error(
                DiagnosticCode::PayloadSinkFailed,
                format!("caller could not accept payload {path:?}: {e}"),
            );
        }
    }

    if !manifest_seen {
        return Envelope::error(
            DiagnosticCode::InvalidBundleStructure,
            "manifest.axgf is missing or not a string",
        );
    }
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }

    let payload_count = bundle.external_payloads.len();
    let payload_bytes: u64 = bundle
        .external_payloads
        .values()
        .map(|p| p.size_bytes)
        .sum();
    let value = serde_json::to_value(&bundle).unwrap_or(Value::Null);

    // An informational diagnostic, not a warning: streaming is a deliberate
    // choice, but a caller reading the envelope should be able to see that
    // this bundle's media is not in the JSON they are holding.
    let note = Diagnostic {
        code: DiagnosticCode::PayloadsExternal,
        severity: Severity::Info,
        message: format!(
            "{payload_count} payload(s), {payload_bytes} byte(s), were streamed out and are \
             recorded in external_payloads; use export_bundle_streaming to write this bundle"
        ),
        entity_ref: None,
    };
    if payload_count == 0 {
        Envelope::ok(value)
    } else {
        Envelope::ok_with(value, vec![note])
    }
}

// -------------------------------------------------------------------------
// export
// -------------------------------------------------------------------------

/// Where a given payload's bytes are to come from during a streaming export.
enum Source {
    /// Declared in `external_payloads`; the caller supplies the bytes.
    External(ExternalPayload),
    /// Carried inline in `attachments` as base64; the library decodes it.
    Inline(String),
}

/// See [`crate::export_bundle_streaming`].
pub fn export_bundle_streaming<W, F>(flat_json: &str, dest: W, mut supply: F) -> Envelope
where
    W: Write + Seek,
    F: FnMut(&mut PayloadSlot<'_>) -> io::Result<()>,
{
    let mut bundle = match parse_flat(flat_json) {
        Ok(b) => b,
        Err(env) => return env,
    };
    if let Err(env) = check_manifest_version(&bundle.manifest) {
        return env;
    }

    // Merge the two possible payload origins into one ordered set. Sorting by
    // path means the entries come out in the same order the non-streaming
    // export writes them, so the two paths produce equivalent archives.
    let mut sources: BTreeMap<String, Source> = BTreeMap::new();
    for (path, meta) in &bundle.external_payloads {
        sources.insert(path.clone(), Source::External(meta.clone()));
    }
    for (path, b64) in &bundle.attachments {
        if sources.contains_key(path) {
            return Envelope::error(
                DiagnosticCode::InvalidBundleStructure,
                format!(
                    "payload {path:?} appears in both attachments and external_payloads; \
                     it must be carried one way or the other, not both"
                ),
            );
        }
        sources.insert(path.clone(), Source::Inline(b64.clone()));
    }

    raise_manifest_version(&mut bundle);
    let fresh_stats = compute_stats(&bundle);
    let now = now_iso8601_utc();
    if let Value::Object(ref mut m) = bundle.manifest {
        m.insert("stats".into(), fresh_stats);
        m.insert("updated_at".into(), Value::String(now));
    }

    let mut zip = ZipWriter::new(dest);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    macro_rules! zip_err {
        ($e:expr) => {
            return Envelope::error(DiagnosticCode::ZipWriteError, $e)
        };
    }

    // manifest.json
    if let Err(e) = write_json_entry(&mut zip, "manifest.json", &bundle.manifest, opts) {
        zip_err!(e);
    }
    // The canonical schema for the version the manifest declares, verbatim.
    let (schema_path, schema_text) = schema_entry(&bundle.manifest);
    if let Err(e) = start_and_write(&mut zip, schema_path, schema_text.as_bytes(), opts) {
        zip_err!(e);
    }

    let per_entity: [(&str, &BTreeMap<String, Value>); 7] = [
        ("persons", &bundle.persons),
        ("families", &bundle.families),
        ("events", &bundle.events),
        ("links", &bundle.links),
        ("occupations", &bundle.occupations),
        ("sources", &bundle.sources),
        ("places", &bundle.places),
    ];
    for (dir, map) in per_entity {
        for (id, value) in map {
            let path = format!("{dir}/{id}.json");
            if let Err(e) = write_json_entry(&mut zip, &path, value, opts) {
                zip_err!(e);
            }
        }
    }

    let doc_index: Value = bundle
        .documents
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<serde_json::Map<_, _>>()
        .into();
    if let Err(e) = write_json_entry(&mut zip, "documents/index.json", &doc_index, opts) {
        zip_err!(e);
    }

    // Payloads, one at a time, in path order.
    let mut payload_bytes: u64 = 0;
    for (path, source) in &sources {
        if let Err(e) = zip.start_file(path, opts) {
            zip_err!(format!("zip start {path}: {e}"));
        }
        match source {
            Source::Inline(b64) => {
                // The inline path still costs one payload's memory — that is
                // what carrying it inline means — but only one at a time.
                let bytes = match BASE64.decode(b64.as_bytes()) {
                    Ok(b) => b,
                    Err(e) => {
                        return Envelope::error(
                            DiagnosticCode::InvalidBundleStructure,
                            format!("attachment {path:?} is not valid base64: {e}"),
                        );
                    }
                };
                if let Err(e) = zip.write_all(&bytes) {
                    zip_err!(format!("zip write {path}: {e}"));
                }
                payload_bytes += bytes.len() as u64;
            }
            Source::External(meta) => {
                let mut slot = PayloadSlot {
                    path: path.clone(),
                    expected_size: meta.size_bytes,
                    expected_crc32: meta.crc32,
                    written: 0,
                    sink: &mut zip,
                };
                if let Err(e) = supply(&mut slot) {
                    return Envelope::error(
                        DiagnosticCode::PayloadSourceFailed,
                        format!("caller could not supply payload {path:?}: {e}"),
                    );
                }
                let written = slot.written;
                // Nothing at all for a payload the bundle declares is the
                // silent-empty-media failure this whole module exists to
                // prevent. Refuse rather than write a hollow archive.
                if written == 0 && meta.size_bytes > 0 {
                    return Envelope::error(
                        DiagnosticCode::PayloadSourceFailed,
                        format!(
                            "payload {path:?} is declared as {} byte(s) but the caller wrote \
                             nothing; refusing to write a bundle with missing media",
                            meta.size_bytes
                        ),
                    );
                }
                // A different length is legitimate — the caller may have
                // replaced the file — but it is worth saying out loud.
                if meta.size_bytes > 0 && written != meta.size_bytes {
                    diagnostics.push(Diagnostic {
                        code: DiagnosticCode::PayloadSourceFailed,
                        severity: Severity::Warning,
                        message: format!(
                            "payload {path:?} was declared as {} byte(s) but {written} were \
                             supplied; the bundle was written with the supplied bytes",
                            meta.size_bytes
                        ),
                        entity_ref: None,
                    });
                }
                payload_bytes += written;
            }
        }
    }

    let mut out = match zip.finish() {
        Ok(w) => w,
        Err(e) => zip_err!(format!("zip finish: {e}")),
    };
    let size_bytes = out.stream_position().unwrap_or(0);

    let data = json!({
        "size_bytes": size_bytes,
        "payloads_written": sources.len(),
        "payload_bytes": payload_bytes,
    });
    if diagnostics.is_empty() {
        Envelope::ok(data)
    } else {
        Envelope::ok_with(data, diagnostics)
    }
}

fn write_json_entry<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &str,
    value: &Value,
    opts: FileOptions,
) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("serialize {path}: {e}"))?;
    start_and_write(zip, path, text.as_bytes(), opts)
}

fn start_and_write<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &str,
    bytes: &[u8],
    opts: FileOptions,
) -> Result<(), String> {
    zip.start_file(path, opts)
        .map_err(|e| format!("zip start {path}: {e}"))?;
    zip.write_all(bytes)
        .map_err(|e| format!("zip write {path}: {e}"))?;
    Ok(())
}
