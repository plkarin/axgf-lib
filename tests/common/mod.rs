// SPDX-License-Identifier: Apache-2.0
//! Shared fixture generation for the streaming tests.
//!
//! The point of the streaming API is bundles too big to keep in memory, so
//! the fixtures that exercise it are too big to commit. They are generated
//! here instead — deterministically, from a seeded PRNG, so a failure is
//! reproducible — and thrown away when the test ends.

#![allow(dead_code)]

use std::io::{Cursor, Write};

use sha2::{Digest, Sha256};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

/// A tiny xorshift64* PRNG. Payload bytes must be incompressible, or deflate
/// shrinks a 64 MiB fixture to nothing and the memory comparison measures
/// the wrong thing entirely.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Fill `buf` with pseudo-random bytes.
    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            let n = chunk.len();
            chunk.copy_from_slice(&v[..n]);
        }
    }
}

/// Hex-encoded SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// One payload in a generated fixture.
pub struct Spec {
    /// In-archive path, e.g. `documents/files/<uuid>.bin`.
    pub path: String,
    /// Uncompressed length in bytes.
    pub size: usize,
}

/// Build a valid `.axgf` archive in memory carrying `specs` as payloads.
///
/// The textual half is deliberately small — a handful of persons and one
/// document metadata record per payload — so that any memory the streaming
/// path uses is visibly attributable to payloads rather than to entities.
///
/// Returns the archive bytes and, for each payload, its digest so a
/// round-trip can be checked payload by payload.
pub fn synthetic_axgf(specs: &[Spec], compress: bool) -> (Vec<u8>, Vec<(String, String)>) {
    let mut rng = Rng::new(0x5EED_1234_ABCD_0001);
    let total: usize = specs.iter().map(|s| s.size).sum();
    let mut buf: Vec<u8> = Vec::with_capacity(total + (1 << 20));
    let mut digests: Vec<(String, String)> = Vec::new();

    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let method = if compress {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        };
        let opts = FileOptions::default().compression_method(method);

        let manifest = serde_json::json!({
            "axgf": "1.0",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "stats": {
                "persons": 2, "families": 0, "events": 0, "links": 0,
                "occupations": 0, "sources": 0, "places": 0,
                "documents": specs.len()
            }
        });
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
            .unwrap();

        for (i, id) in [
            "11111111-1111-4111-8111-111111111111",
            "22222222-2222-4222-8222-222222222222",
        ]
        .iter()
        .enumerate()
        {
            let person = serde_json::json!({
                "id": id,
                "type": "person",
                "axgf_version": "1.0",
                "identity": {
                    "name": { "display": format!("Test Person {i}"), "components": [] },
                    "is_living": false
                }
            });
            zip.start_file(format!("persons/{id}.json"), opts).unwrap();
            zip.write_all(serde_json::to_string_pretty(&person).unwrap().as_bytes())
                .unwrap();
        }

        // Document metadata: textual, stays in the flat JSON on every path.
        let mut index = serde_json::Map::new();
        for (i, s) in specs.iter().enumerate() {
            let id = format!("{:08x}-0000-4000-8000-000000000000", i);
            index.insert(
                id.clone(),
                serde_json::json!({
                    "id": id,
                    "type": "document",
                    "axgf_version": "1.0",
                    "filename": format!("payload-{i}.bin"),
                    "mime_type": "application/octet-stream",
                    "status": "present",
                    "file": { "path": s.path, "size_bytes": s.size }
                }),
            );
        }
        zip.start_file("documents/index.json", opts).unwrap();
        zip.write_all(
            serde_json::to_string_pretty(&serde_json::Value::Object(index))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();

        // The payloads themselves, written in chunks so generating a large
        // fixture does not itself need the whole payload in memory.
        let mut chunk = vec![0u8; 256 * 1024];
        for s in specs {
            zip.start_file(&s.path, opts).unwrap();
            let mut hasher = Sha256::new();
            let mut left = s.size;
            while left > 0 {
                let n = left.min(chunk.len());
                rng.fill(&mut chunk[..n]);
                hasher.update(&chunk[..n]);
                zip.write_all(&chunk[..n]).unwrap();
                left -= n;
            }
            let digest = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            digests.push((s.path.clone(), digest));
        }

        zip.finish().unwrap();
    }

    (buf, digests)
}

/// Convenience: `n` payloads of `each` bytes plus one of `largest` bytes.
pub fn specs(n: usize, each: usize, largest: usize) -> Vec<Spec> {
    let mut v: Vec<Spec> = (0..n)
        .map(|i| Spec {
            path: format!("documents/files/{:08x}-0000-4000-8000-000000000000.bin", i),
            size: each,
        })
        .collect();
    v.push(Spec {
        path: "documents/files/ffffffff-0000-4000-8000-000000000000.bin".to_string(),
        size: largest,
    });
    v
}

/// Read every payload out of an archive, returning `(path, sha256)` pairs in
/// archive order. Used to compare a re-exported bundle against its source.
pub fn payload_digests(zip_bytes: &[u8]) -> Vec<(String, String)> {
    let mut ar = zip::ZipArchive::new(Cursor::new(zip_bytes)).expect("valid zip");
    let mut out = Vec::new();
    for i in 0..ar.len() {
        let mut f = ar.by_index(i).unwrap();
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        let textual = name == "manifest.json"
            || name == "documents/index.json"
            || name.starts_with("schema/")
            || (name.ends_with(".json") && !name.starts_with("documents/files/"));
        if textual {
            continue;
        }
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut buf).unwrap();
        out.push((name, sha256_hex(&buf)));
    }
    out.sort();
    out
}

/// Every entry name in an archive, sorted.
pub fn entry_names(zip_bytes: &[u8]) -> Vec<String> {
    let mut ar = zip::ZipArchive::new(Cursor::new(zip_bytes)).expect("valid zip");
    let mut v: Vec<String> = (0..ar.len())
        .map(|i| ar.by_index(i).unwrap().name().to_string())
        .collect();
    v.sort();
    v
}

// -------------------------------------------------------------------------
// heap measurement
// -------------------------------------------------------------------------

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// A global allocator that records the high-water mark of live heap bytes.
///
/// A test binary opts in with
/// `#[global_allocator] static A: common::Tracking = common::Tracking;`.
/// Because the peak is process-wide and the test harness runs tests on
/// several threads at once, a binary that installs this must contain exactly
/// one `#[test]`, or the readings interleave.
///
/// `alloc_zeroed` is left to the trait default, which routes through `alloc`
/// and so stays accounted for. `realloc` forwards to `System.realloc`: the
/// default implementation would allocate-copy-free on every `Vec` growth,
/// which is unlike what the same code does in production. A growth satisfied
/// in place therefore does not count as transiently holding both blocks —
/// exactly as in a real run.
pub struct Tracking;

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            note_alloc(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            if new_size >= layout.size() {
                note_alloc(new_size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        p
    }
}

fn note_alloc(n: usize) {
    let live = LIVE.fetch_add(n, Ordering::Relaxed) + n;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

/// Drop the high-water mark back to what is live now, so the next phase
/// measures only itself.
pub fn reset_peak() {
    PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Peak live bytes since the last [`reset_peak`], above `baseline`.
pub fn peak_above(baseline: usize) -> usize {
    PEAK.load(Ordering::Relaxed).saturating_sub(baseline)
}

/// Live heap bytes right now.
pub fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
}

/// Bytes as MiB, for readable test output.
pub fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// A `Write + Seek` that tracks a position but discards the bytes, so a
/// streaming export's own cost can be measured without the finished archive
/// dominating the reading. Archive correctness is established separately in
/// `tests/streaming.rs`.
#[derive(Default)]
pub struct NullSink {
    pos: u64,
    len: u64,
}

impl std::io::Write for NullSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pos += buf.len() as u64;
        self.len = self.len.max(self.pos);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for NullSink {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        use std::io::SeekFrom::*;
        self.pos = match from {
            Start(n) => n,
            End(n) => (self.len as i64 + n) as u64,
            Current(n) => (self.pos as i64 + n) as u64,
        };
        Ok(self.pos)
    }
}
