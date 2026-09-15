// SPDX-License-Identifier: Apache-2.0
//! # model — typed representations of AXGF entities
//!
//! Strongly-typed Rust structs mirroring the eight entity kinds defined in
//! the specification (`Person`, `Family`, `Event`, `Link`, `Occupation`,
//! `Source`, `Place`, `Document`) plus the top-level `Manifest`. Each kind
//! lives in its own sub-module.
//!
//! **Internal to the library.** These types never cross the public
//! boundary; the boundary is JSON only. Rich typing lives here so
//! [`crate::logic`] can operate safely, but callers see only flat JSON.
//!
//! Every struct uses `#[serde(flatten)] pub extra: BTreeMap<String, Value>`
//! (or an equivalent) to preserve unknown fields untouched across a
//! round-trip, per AXGF principle **P9** (extensible without breaking).
//!
//! [`profile`] holds what AXGF 1.1 adds to a Person — its claim shape, its
//! closed vocabularies, its attribute registry and typed blocks — kept
//! apart so that the eight 1.0 modules still read as the 1.0 model.

pub mod common;
pub mod document;
pub mod event;
pub mod family;
pub mod link;
pub mod manifest;
pub mod occupation;
pub mod person;
pub mod place;
pub mod profile;
pub mod source;
