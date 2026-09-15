// SPDX-License-Identifier: Apache-2.0
//! Typed representation of the AXGF [`Person`] entity, mirroring
//! `#/$defs/person` in the schema and SPEC §4.1.

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use super::common::{AiHypothesis, AxgfDate, AxgfName, BaseEntity, DocumentLink, Extra};
use super::place::Coordinates;
use super::profile::types::{
    AutopsyValue, Belief, Biometrics, CauseValue, CivilStatus, ContributingFactorValue,
    DigitalLegacy, DispositionValue, Education, Genomics, GraveValue, Health, Legal, Military,
    Morphology, Personality, Residence, TitleValue,
};
use super::profile::Claim;

/// A person's gender declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Gender {
    /// One of `M`, `F`, `NB`, `U` (unknown).
    pub value: String,
    /// Optional explanatory note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Identity block (name, alternate names, gender, is-living flag, visibility).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Identity {
    /// Primary display name.
    pub name: AxgfName,
    /// Alternate names (birth name, aliases, transliterations, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<AxgfName>,
    /// Gender information.
    pub gender: Gender,
    /// Whether the person is currently living (drives privacy handling).
    pub is_living: bool,
    /// Visibility level: `public | members | contributors | private`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    /// 1.1: titles, dated (SPEC_1.1 §5.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub titles: Vec<Claim<TitleValue>>,
    /// 1.1: what the birth was registered as, a term of
    /// [`super::profile::vocab::SEX_AT_BIRTH`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex_at_birth: Option<Claim<String>>,
    /// 1.1: gender identity over time, terms of
    /// [`super::profile::vocab::GENDER_IDENTITY`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gender_identity: Vec<Claim<String>>,
    /// 1.1: per-class visibility for this person, which can only tighten the
    /// default (SPEC_1.1 §4.4). Keys are sensitive classes, values visibilities.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub class_visibility: BTreeMap<String, String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Vital event block (birth or death), attached directly to a Person.
/// A first-class [`crate::model::event::Event`] MAY additionally exist and
/// be linked via `event_id`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Vital {
    /// Date of the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<AxgfDate>,
    /// Referenced [`crate::model::place::Place`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Death-specific: cause of death (unused for birth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// Overall confidence in \[0.0, 1.0\].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Referenced [`crate::model::source::Source`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Referenced first-class [`crate::model::event::Event`] UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// 1.1: local time as recorded (SPEC_1.1 §5.1, §5.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<Claim<String>>,
    /// 1.1: where it happened, when known more precisely than the place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Claim<Coordinates>>,
    /// 1.1, death only: the causal chain. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<Claim<CauseValue>>,
    /// 1.1, death only: conditions that contributed. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributing_factors: Vec<Claim<ContributingFactorValue>>,
    /// 1.1, death only: autopsy and forensic report. Class `health`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autopsy: Option<Claim<AutopsyValue>>,
    /// 1.1, death only: burial, cremation, and any later reburial.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disposition: Vec<Claim<DispositionValue>>,
    /// 1.1, death only: where the grave is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grave: Vec<Claim<GraveValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// AI metadata attached to a Person. Mirrors `person.ai`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonAi {
    /// Relative path to the Markdown vault page for this person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_page: Option<String>,
    /// Embedding model identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Last time the embedding was recomputed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_updated_at: Option<String>,
    /// Machine-generated hypotheses about this person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hypotheses: Vec<AiHypothesis>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A Person entity. See SPEC §4.1 for the full field-by-field contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    /// Base entity fields (`id`, `type`, `axgf_version`, timestamps, …).
    #[serde(flatten)]
    pub base: BaseEntity,
    /// Identity block (name, gender, visibility).
    pub identity: Identity,
    /// Birth vitals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth: Option<Vital>,
    /// Death vitals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<Vital>,
    /// Free-form biography.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    /// Free-form notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Documents attached to this person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<DocumentLink>,
    /// AI metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<PersonAi>,
    /// 1.1: civil status (SPEC_1.1 §5.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub civil_status: Option<CivilStatus>,
    /// 1.1: morphology (SPEC_1.1 §5.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub morphology: Option<Morphology>,
    /// 1.1: biometrics (SPEC_1.1 §5.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biometrics: Option<Biometrics>,
    /// 1.1: health (SPEC_1.1 §5.4). Class `health` throughout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<Health>,
    /// 1.1: genomics (SPEC_1.1 §5.5). Class `genomics` throughout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genomics: Option<Genomics>,
    /// 1.1: residence and nationality (SPEC_1.1 §5.7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residence: Option<Residence>,
    /// 1.1: education and work (SPEC_1.1 §5.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub education: Option<Education>,
    /// 1.1: military and honours (SPEC_1.1 §5.9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub military: Option<Military>,
    /// 1.1: legal (SPEC_1.1 §5.10). Class `legal`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal: Option<Legal>,
    /// 1.1: belief and affiliation (SPEC_1.1 §5.11). Class `health`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belief: Option<Belief>,
    /// 1.1: personality and behaviour (SPEC_1.1 §5.12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personality: Option<Personality>,
    /// 1.1: digital legacy (SPEC_1.1 §5.14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digital_legacy: Option<DigitalLegacy>,
    /// Forward-compatible extras — anything else the spec adds later.
    #[serde(flatten)]
    pub extra: Extra,
}
