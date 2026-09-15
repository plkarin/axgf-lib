// SPDX-License-Identifier: Apache-2.0
//! # profile — the AXGF 1.1 extended person profile
//!
//! [SPEC_1.1] adds fourteen groups of attributes about a person. This module
//! is that addition, in three parts, and none of them is behaviour:
//!
//! - [`claim`] — the shape every attribute holds: a [`Claim`], dated,
//!   sourced and rated with 1.0's own date, source and confidence.
//! - [`vocab`] and [`registry`] — the 102 closed vocabularies and the table
//!   of every attribute: its path, whether it is a single claim or a series,
//!   the shape of its value and its sensitive class. An implementation reads
//!   these to build a form, to check a term, or to find every `health`
//!   attribute without keeping a second list that will one day disagree with
//!   the first.
//! - [`types`] — typed structs for the new Person blocks, for code that wants
//!   them. The boundary stays JSON; these never cross it.
//!
//! What 1.1 means for validation — out-of-vocabulary values, the version a
//! bundle must declare — lives in [`crate::logic::validate`], and what it
//! means for writing — raising a manifest to 1.1 — in [`crate::logic::crud`].
//!
//! [SPEC_1.1]: https://github.com/plkarin/axgf-spec/blob/main/SPEC_1.1.md

pub mod claim;
pub mod registry;
pub mod types;
pub mod vocab;

pub use claim::{ArtefactRef, Claim, ClaimBound, Coordinates};
pub use registry::{
    attribute, attributes, attributes_of_class, group, Attribute, Cardinality, Field, Group,
    Mapped, SensitiveClass, Shape, GROUPS, PROFILE_BLOCKS,
};
pub use types::*;
