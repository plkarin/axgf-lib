// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Place`] entity, mirroring
//! `#/$defs/place` in the schema and SPEC §5.3.

use serde::{Deserialize, Serialize};

use super::common::{BaseEntity, Extra};

/// One name of a place in a specific language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceName {
    /// BCP 47 language tag.
    pub lang: String,
    /// The name in that language.
    pub value: String,
    /// `true` if this is the canonical display name.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_primary: bool,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

fn is_false(b: &bool) -> bool { !*b }

/// Geographic coordinates of a place.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Coordinates {
    /// Latitude in decimal degrees, \[-90, 90\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude in decimal degrees, \[-180, 180\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    /// Precision hint (`city_center`, `building`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// One period during which a place belonged to a given country.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryHistoryEntry {
    /// Country code (ISO 3166-1 alpha-2 or spec-extended, e.g. `SU`).
    pub country: String,
    /// Start of the period (`null` = since forever).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// End of the period (`null` = still current).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// External identifiers for a place. Mirrors `place.identifiers`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaceIdentifiers {
    /// Wikidata Q-number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikidata: Option<String>,
    /// GeoNames id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geonames: Option<String>,
    /// INSEE code (French communes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insee: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A Place entity. See SPEC §5.3.
///
/// # Field name note
///
/// The schema declares two properties both meaning "place kind":
/// `type` (the base-entity discriminator, always `"place"`) and
/// `place_type` (the geographic kind: `city`, `village`, …). The former
/// is inherited from [`BaseEntity`] via `#[serde(flatten)]`; the latter
/// is exposed here as [`Place::place_type`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    /// Base entity fields.
    #[serde(flatten)]
    pub base: BaseEntity,
    /// One-or-more names of the place, in different languages.
    pub names: Vec<PlaceName>,
    /// Geographic kind (`city`, `village`, `country`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_type: Option<String>,
    /// Region / department the place belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Current country (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_current: Option<String>,
    /// Coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinates>,
    /// Country-of-record history (for places whose sovereignty changed).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub country_history: Vec<CountryHistoryEntry>,
    /// External identifiers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<PlaceIdentifiers>,
    /// Free-form note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
