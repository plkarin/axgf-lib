// SPDX-License-Identifier: Apache-2.0
//! # cffi adapter — C ABI wrappers (stub)
//!
//! Enabled with `--features cffi`. This adapter is currently a
//! compile-check stub: it declares the intended shape of the C
//! surface but does not yet manage C string lifetimes. The full
//! implementation lands in V1.1 once the FFI conventions for the
//! consumer targets (Python via cffi, Go via cgo) are locked in.
//!
//! ## Intended surface (V1.1)
//!
//! Each boundary function will take UTF-8-encoded, NUL-terminated C
//! strings for JSON arguments and return a heap-allocated
//! NUL-terminated response that the caller MUST free with an
//! `axgf_free` symbol. Byte-array arguments will carry an explicit
//! `len`. Rolling this out requires enabling `unsafe_code` in the
//! crate root — deferred so V1 keeps the `#![forbid(unsafe_code)]`
//! guarantee across the whole library.

/// Return the current AXGF spec version this build writes when
/// creating or exporting bundles. Safe wrapper over the
/// [`crate::CURRENT_SPEC_VERSION`] constant so bindings can smoke-
/// test that the adapter is linked at all.
pub fn current_spec_version() -> &'static str {
    crate::CURRENT_SPEC_VERSION
}
