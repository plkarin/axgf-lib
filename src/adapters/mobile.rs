// SPDX-License-Identifier: Apache-2.0
//! # mobile adapter — UniFFI wrappers (stub)
//!
//! Enabled with `--features mobile`. This adapter is currently a
//! compile-check stub: it declares the intended UniFFI-callable
//! interface but does not yet emit the `.udl` scaffolding. The full
//! implementation lands in V1.1 once the mobile-side calling
//! conventions (structured value passing vs JSON strings) are
//! locked in with the Android and iOS integrators.
//!
//! For V1, mobile callers can consume the library via the
//! [`crate::adapters::cffi`] C ABI or by depending directly on the
//! Rust crate.

/// See [`crate::CURRENT_SPEC_VERSION`]. Placeholder so the crate
/// compiles under `--features mobile`.
pub fn current_spec_version() -> String {
    crate::CURRENT_SPEC_VERSION.to_string()
}
