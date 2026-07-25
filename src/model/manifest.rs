// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the top-level bundle [`Manifest`], mirroring
//! `#/$defs/manifest` in the schema and SPEC §3.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::Extra;

/// Optional details about the software that generated the bundle.
/// Mirrors `manifest.generator`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Generator {
    /// Generator name (e.g. `ax-genealogy`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Generator version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Optional public URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Time coverage of the family data. Mirrors `manifest.family.time_span`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FamilyTimeSpan {
    /// Earliest known date in the bundle (ISO 8601 partial).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub earliest: Option<String>,
    /// Latest known date in the bundle (ISO 8601 partial).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Family-level metadata block. Mirrors `manifest.family`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FamilyInfo {
    /// Human-readable family name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Short description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// BCP 47 culture tag most represented in the bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_culture: Option<String>,
    /// Human-readable primary place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_place: Option<String>,
    /// Time coverage of the data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_span: Option<FamilyTimeSpan>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Counts of each entity kind in the bundle. Recomputed by
/// `export_bundle` before writing. Mirrors `manifest.stats`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    /// Number of persons in the bundle.
    #[serde(default)]
    pub persons: u64,
    /// Number of families in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub families: u64,
    /// Number of events in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub events: u64,
    /// Number of links in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub links: u64,
    /// Number of occupations in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub occupations: u64,
    /// Number of source records in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub sources: u64,
    /// Number of places in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub places: u64,
    /// Number of documents in the bundle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub documents: u64,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

fn is_zero(n: &u64) -> bool { *n == 0 }

/// Privacy flags declared by the exporter. Mirrors `manifest.privacy`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Privacy {
    /// `true` when any Person has `is_living = true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains_living_persons: Option<bool>,
    /// `true` when living-person fields were stripped for export.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub living_persons_redacted: Option<bool>,
    /// Exporter's GDPR-compliance assertion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gdpr_compliant: Option<bool>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// License block. Mirrors `manifest.license`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct License {
    /// License type identifier (e.g. `private`, `cc-by`, `cc0`).
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub kind: Option<String>,
    /// URL to the license text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional note (e.g. "Family use only").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The top-level manifest for an AXGF bundle. Mirrors
/// `#/$defs/manifest` and SPEC §3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// AXGF spec version (`"1.0"` for this library).
    pub axgf: String,
    /// Bundle creation timestamp (ISO 8601).
    pub created_at: String,
    /// Last-modification timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Optional generator information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<Generator>,
    /// Optional family metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<FamilyInfo>,
    /// Entity counts (recomputed on export).
    pub stats: Stats,
    /// Optional checksum map (arbitrary key/value strings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksums: Option<Value>,
    /// Optional privacy flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy: Option<Privacy>,
    /// Optional license block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    /// Optional GEDCOM compatibility markers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<Value>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
