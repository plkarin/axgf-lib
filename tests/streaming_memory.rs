// SPDX-License-Identifier: Apache-2.0
//! Measured proof that the streaming path is bounded by the largest single
//! payload rather than by their sum.
//!
//! The claim is quantitative, so it is measured rather than argued. A
//! tracking global allocator records the high-water mark of live heap bytes;
//! each phase resets the mark, runs one operation, and reads it back.
//!
//! # Why this is the only test in its binary
//!
//! The peak is a process-wide quantity and the test harness runs tests
//! concurrently, so a second test here would interleave its allocations with
//! this one's and corrupt the reading. There is exactly one `#[test]`, and
//! its phases run in sequence.
//!
//! # Why the fixture is small, and stored rather than deflated
//!
//! This one runs on every `cargo test`, so it uses a fixture big enough to
//! make the ratio unambiguous and small enough to be quick. Its payloads are
//! stored rather than deflated purely for speed: compression is CPU, not
//! memory, and both read paths behave identically either way — the streaming
//! one decompresses through a fixed window, the non-streaming one buffers
//! the whole payload regardless. The same measurement at a realistic 70 MiB,
//! properly deflated, lives in `tests/streaming_memory_large.rs`, which is
//! `#[ignore]`d.
//!
//! # What is measured
//!
//! Live heap bytes from Rust allocations, above the baseline of the
//! already-built fixture. That is the quantity the caller controls and the
//! one that decides whether a host is killed. It is deliberately not RSS:
//! RSS additionally reflects arenas the allocator has not returned to the
//! OS, which is a property of the allocator rather than of this API.

mod common;

use std::io::{Cursor, Write};

use axgf_rs::boundary::envelope::Status;
use axgf_rs::{
    export_bundle, export_bundle_streaming, import_bundle, import_bundle_streaming,
    import_bundle_textual,
};
use common::{live, mib, peak_above, reset_peak, specs, synthetic_axgf, NullSink, Rng};

#[global_allocator]
static ALLOC: common::Tracking = common::Tracking;

/// Thirty-one payloads of 256 KiB plus one of 4 MiB: 11.75 MiB in total,
/// nearly three times the largest single payload, and every payload after
/// the first well under it.
const EACH: usize = 256 * 1024;
const LARGEST: usize = 4 * 1024 * 1024;
const COUNT: usize = 31;

/// A stand-in for "available memory" in the sense the requirement means it:
/// a ceiling the process must stay under. It is set below the size of the
/// largest single payload on purpose, so that a payload which cannot be held
/// whole still has to import successfully.
const BUDGET: usize = 2 * 1024 * 1024;

#[test]
fn streaming_is_bounded_by_the_largest_payload_and_inline_is_bounded_by_their_sum() {
    let specs = specs(COUNT, EACH, LARGEST);
    let total: usize = specs.iter().map(|s| s.size).sum();
    // The fixture must contain a payload larger than the budget, or the
    // oversized-payload case is not being exercised at all.
    const { assert!(LARGEST > BUDGET) };

    // Payload bytes are pseudo-random regardless, so that the large variant
    // of this test — which does deflate — measures a real archive.
    let (zip, _digests) = synthetic_axgf(&specs, false);

    // ---- phase 1: streaming import, copying each payload straight out ----
    let base = live();
    reset_peak();
    let mut count = 0usize;
    let mut bytes_seen = 0u64;
    let env = import_bundle_streaming(Cursor::new(&zip), |p| {
        count += 1;
        bytes_seen += p.copy_to(std::io::sink())?;
        Ok(())
    });
    let peak_stream_copy = peak_above(base);
    assert_eq!(env.status, Status::Ok);
    assert_eq!(count, specs.len());
    assert_eq!(bytes_seen as usize, total);
    drop(env);

    // ---- phase 2: streaming import, buffering each payload whole ----
    let base = live();
    reset_peak();
    let env = import_bundle_streaming(Cursor::new(&zip), |p| {
        let v = p.read_to_end()?;
        std::hint::black_box(&v);
        Ok(())
    });
    let peak_stream_buffered = peak_above(base);
    assert_eq!(env.status, Status::Ok);
    drop(env);

    // ---- phase 3: textual import, touching no payload at all ----
    let base = live();
    reset_peak();
    let textual = import_bundle_textual(Cursor::new(&zip));
    let peak_textual = peak_above(base);
    assert_eq!(textual.status, Status::Ok);
    let textual_json = textual.data.to_string();
    drop(textual);

    // ---- phase 4: the non-streaming import, for comparison ----
    let base = live();
    reset_peak();
    let inline = import_bundle(&zip);
    let peak_inline = peak_above(base);
    assert_eq!(inline.status, Status::Ok);

    // ---- phase 5: non-streaming export, from the bundle it requires ----
    let inline_json = inline.data.to_string();
    drop(inline);
    let base = live();
    reset_peak();
    let out = export_bundle(&inline_json);
    let peak_export_inline = peak_above(base);
    assert_eq!(out.status, Status::Ok);
    drop(out);
    drop(inline_json);

    // ---- phase 6: streaming export, supplying payloads on demand ----
    let base = live();
    reset_peak();
    let mut rng = Rng::new(99);
    let mut chunk = vec![0u8; 64 * 1024];
    let mut sink = NullSink::default();
    let out = export_bundle_streaming(&textual_json, &mut sink, |slot| {
        let mut left = slot.expected_size() as usize;
        while left > 0 {
            let n = left.min(chunk.len());
            rng.fill(&mut chunk[..n]);
            slot.write_all(&chunk[..n])?;
            left -= n;
        }
        Ok(())
    });
    let peak_export_stream = peak_above(base);
    assert_eq!(out.status, Status::Ok, "{:?}", out.diagnostics);
    assert_eq!(out.data["payloads_written"], specs.len());

    eprintln!("\n  peak heap above baseline, per operation");
    eprintln!("  ---------------------------------------------------------");
    for (label, v) in [
        ("import  streaming  (copy_to)", peak_stream_copy),
        ("import  streaming  (read_to_end)", peak_stream_buffered),
        ("import  textual    (no payloads)", peak_textual),
        ("import  non-streaming", peak_inline),
        ("export  streaming", peak_export_stream),
        ("export  non-streaming", peak_export_inline),
    ] {
        eprintln!("  {label:<34} {:>8.2} MiB", mib(v));
    }
    eprintln!(
        "  ---------------------------------------------------------\n  \
         payload total {:.2} MiB, largest {:.2} MiB, budget {:.2} MiB\n",
        mib(total),
        mib(LARGEST),
        mib(BUDGET)
    );

    // ---- the assertions ----

    // Copying straight through never holds a whole payload, so the peak is
    // the copy buffer plus the textual bundle. Note this also settles the
    // oversized-payload case: the fixture's largest payload is twice the
    // budget, and it still imports, because it is never held whole.
    assert!(
        peak_stream_copy < BUDGET,
        "streaming import with copy_to peaked at {:.2} MiB, over the {:.2} MiB budget",
        mib(peak_stream_copy),
        mib(BUDGET)
    );

    // Buffering each payload whole costs one payload at a time. Allow a
    // factor of two for the growth behaviour of the Vec holding it.
    assert!(
        peak_stream_buffered < 2 * LARGEST,
        "streaming import with read_to_end peaked at {:.2} MiB; bounded by the largest \
         payload ({:.2} MiB) means under {:.2} MiB",
        mib(peak_stream_buffered),
        mib(LARGEST),
        mib(2 * LARGEST)
    );

    // The textual half is bounded by the tree, not the media.
    assert!(
        peak_textual < BUDGET,
        "textual import peaked at {:.2} MiB and should be a small fraction of the \
         {:.2} MiB budget",
        mib(peak_textual),
        mib(BUDGET)
    );

    // And the thing being fixed. Base64 inflates by a third and the encoded
    // form is then copied into a JSON value, so the non-streaming import
    // needs more than every payload put together — and vastly more than the
    // budget that streaming respected.
    assert!(
        peak_inline > total,
        "non-streaming import peaked at {:.2} MiB, expected to exceed the {:.2} MiB \
         payload total",
        mib(peak_inline),
        mib(total)
    );
    assert!(
        peak_inline > 4 * BUDGET,
        "non-streaming import peaked at {:.2} MiB; it cannot honour the {:.2} MiB budget \
         that streaming met",
        mib(peak_inline),
        mib(BUDGET)
    );

    // The headline ratio, asserted so it cannot quietly regress.
    assert!(
        peak_inline > 4 * peak_stream_buffered,
        "expected the non-streaming import ({:.2} MiB) to cost several times the \
         streaming one ({:.2} MiB)",
        mib(peak_inline),
        mib(peak_stream_buffered)
    );

    // Export tells the same story: streaming never holds the archive.
    assert!(
        peak_export_stream < BUDGET,
        "streaming export peaked at {:.2} MiB, expected under the {:.2} MiB budget",
        mib(peak_export_stream),
        mib(BUDGET)
    );
    assert!(
        peak_export_inline > total,
        "non-streaming export peaked at {:.2} MiB, expected to exceed the {:.2} MiB \
         payload total",
        mib(peak_export_inline),
        mib(total)
    );
}
