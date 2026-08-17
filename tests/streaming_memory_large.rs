// SPDX-License-Identifier: Apache-2.0
//! The same measurement as `tests/streaming_memory.rs`, at a size that
//! resembles a real media-carrying bundle: 70 MiB of payloads, the largest
//! 8 MiB.
//!
//! Generating and deflating that much incompressible data takes the best
//! part of a minute in a debug build, which is far too slow to sit in the
//! default suite — a slow test is a test people learn to skip. It is
//! `#[ignore]`d, and the fast version guards against regression on every
//! run.
//!
//! Run it with:
//!
//! ```text
//! cargo test --test streaming_memory_large -- --ignored --nocapture
//! ```
//!
//! `--nocapture` matters: the numbers are printed, not just asserted.

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

const EACH: usize = 2 * 1024 * 1024;
const LARGEST: usize = 8 * 1024 * 1024;
const COUNT: usize = 31;

#[test]
#[ignore = "generates a 70 MiB fixture; run with --ignored"]
fn streaming_holds_at_seventy_megabytes_of_payloads() {
    let specs = specs(COUNT, EACH, LARGEST);
    let total: usize = specs.iter().map(|s| s.size).sum();
    let (zip, _) = synthetic_axgf(&specs, true);
    eprintln!(
        "\nfixture: {} payloads, {:.1} MiB total, largest {:.1} MiB, archive {:.1} MiB",
        specs.len(),
        mib(total),
        mib(LARGEST),
        mib(zip.len())
    );

    let base = live();
    reset_peak();
    let env = import_bundle_streaming(Cursor::new(&zip), |p| {
        p.copy_to(std::io::sink())?;
        Ok(())
    });
    let peak_stream_copy = peak_above(base);
    assert_eq!(env.status, Status::Ok);
    drop(env);

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

    let base = live();
    reset_peak();
    let textual = import_bundle_textual(Cursor::new(&zip));
    let peak_textual = peak_above(base);
    assert_eq!(textual.status, Status::Ok);
    let textual_json = textual.data.to_string();
    drop(textual);

    let base = live();
    reset_peak();
    let inline = import_bundle(&zip);
    let peak_inline = peak_above(base);
    assert_eq!(inline.status, Status::Ok);

    let inline_json = inline.data.to_string();
    drop(inline);
    let base = live();
    reset_peak();
    let out = export_bundle(&inline_json);
    let peak_export_inline = peak_above(base);
    assert_eq!(out.status, Status::Ok);
    drop(out);
    drop(inline_json);

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
        eprintln!("  {label:<34} {:>9.2} MiB", mib(v));
    }
    eprintln!(
        "  ---------------------------------------------------------\n  \
         payload total {:.1} MiB, largest payload {:.1} MiB\n",
        mib(total),
        mib(LARGEST)
    );

    assert!(peak_stream_copy < EACH);
    assert!(peak_stream_buffered < 2 * LARGEST);
    assert!(peak_textual < EACH);
    assert!(peak_inline > total);
    assert!(peak_inline > 8 * peak_stream_buffered);
    assert!(peak_export_stream < 2 * LARGEST);
    assert!(peak_export_inline > total);
}
