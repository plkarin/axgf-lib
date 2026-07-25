// SPDX-License-Identifier: Apache-2.0
//! # adapters — thin per-target wrappers around the public API
//!
//! Each adapter is **logic-free**: it exists solely to expose the crate's
//! public functions in the calling conventions of a specific target. All
//! semantics live in [`crate::logic`] and [`crate::boundary`].
//!
//! Adapters gated by feature flag:
//!
//! - [`rust`] — default; re-exports the crate root for direct Rust use.
//! - `wasm` — `wasm-bindgen` shims. Enabled by feature `wasm`.
//! - `cffi` — C ABI stubs. Enabled by feature `cffi`.
//! - `mobile` — uniffi-based Kotlin/Swift stubs. Enabled by feature
//!   `mobile`.

pub mod rust;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "cffi")]
pub mod cffi;

#[cfg(feature = "mobile")]
pub mod mobile;
