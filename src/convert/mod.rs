// SPDX-License-Identifier: Apache-2.0
//! # convert — foreign-format ingestion
//!
//! One converter per supported source format. Each returns a flat-bundle
//! JSON wrapped in an [`crate::boundary::envelope::Envelope`]; downstream
//! validation and CRUD then work identically to any other bundle.
//!
//! Currently: [`gedcom`] for GEDCOM 5.5.1 (behind the `gedcom` feature).

#[cfg(feature = "gedcom")]
pub mod gedcom;
