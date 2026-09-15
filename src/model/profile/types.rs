// SPDX-License-Identifier: Apache-2.0
//! Typed structs for the AXGF 1.1 profile blocks (SPEC_1.1 §5).
//!
//! Each block is a struct of attributes; each attribute is a [`Claim`] or a
//! series of them; an object-valued claim has a `…Value` struct for its
//! `value`. Vocabulary terms are `String`s — the model keeps whatever the
//! bundle holds, and [`crate::validate`] says whether it is in vocabulary —
//! and numbers are [`serde_json::Number`] so that `158` and `158.0`
//! round-trip as they were written. Every struct ends in `extra`, so
//! nothing a later version adds is lost passing through this one.

use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::claim::{ArtefactRef, Claim, Coordinates};
#[allow(unused_imports)]
use super::vocab;
use crate::model::common::Extra;

/// The `value` of `identity.titles`: titles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleValue {
    /// `text`. Free text.
    pub text: String,
    /// `kind`. A term of [`vocab::TITLE_KIND`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `civil_status.register_entries`: civil register entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterEntryValue {
    /// `register_type`. A term of [`vocab::REGISTER_TYPE`].
    pub register_type: String,
    /// `office`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub office: Option<String>,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// `volume`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// `page`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// `entry_number`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_number: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `civil_status.marginal_annotations`: marginal annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginalAnnotationValue {
    /// `text`. Free text.
    pub text: String,
    /// `register_type`. A term of [`vocab::REGISTER_TYPE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register_type: Option<String>,
    /// `entry_number`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_number: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `morphology.body_composition`: body composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyCompositionValue {
    /// `bone_percent`. A number in %.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bone_percent: Option<Number>,
    /// `muscle_percent`. A number in %.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muscle_percent: Option<Number>,
    /// `fat_percent`. A number in %.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fat_percent: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `morphology.pigmentation`: pigmentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PigmentationValue {
    /// `kind`. A term of [`vocab::PIGMENTATION_MARK`].
    pub kind: String,
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_region: Option<String>,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `morphology.scars`: scars.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScarValue {
    /// `description`. Free text.
    pub description: String,
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_region: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `morphology.tattoos`: tattoos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TattooValue {
    /// `description`. Free text.
    pub description: String,
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_region: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `morphology.moles`: moles with location and shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleValue {
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    pub body_region: String,
    /// `location`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// `shape`. A term of [`vocab::MOLE_SHAPE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// `diameter_mm`. A number in mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter_mm: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `biometrics.spoken_accent`: spoken accent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccentValue {
    /// `language`. A BCP 47 language tag.
    pub language: String,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `biometrics.hearing`: hearing capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HearingValue {
    /// `ear`. A term of [`vocab::LATERALITY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ear: Option<String>,
    /// `grade`. A term of [`vocab::HEARING_GRADE`].
    pub grade: String,
    /// `threshold_db`. A number in dB HL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_db: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `biometrics.visual_acuity`: visual acuity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualAcuityValue {
    /// `eye`. A term of [`vocab::LATERALITY`].
    pub eye: String,
    /// `decimal`. A number.
    pub decimal: Number,
    /// `corrected`. Yes or no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corrected: Option<bool>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `biometrics.optical_correction`: optical correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpticalCorrectionValue {
    /// `kind`. A term of [`vocab::OPTICAL_CORRECTION`].
    pub kind: String,
    /// `prescription`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prescription: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.blood_pressure`: mean blood pressure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodPressureValue {
    /// `systolic`. A whole number in mmHg.
    pub systolic: Number,
    /// `diastolic`. A whole number in mmHg.
    pub diastolic: Number,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.respiratory_capacity`: respiratory capacity (FEV1/FVC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespiratoryCapacityValue {
    /// `fev1_litres`. A number in L.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fev1_litres: Option<Number>,
    /// `fvc_litres`. A number in L.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fvc_litres: Option<Number>,
    /// `fev1_fvc_ratio`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fev1_fvc_ratio: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.conditions`: chronic pathologies; diagnosed chronic illnesses; cardiovascular, respiratory, metabolic and endocrine pathologies; autoimmune diseases; oncological pathologies; psychiatric and neurodevelopmental conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionValue {
    /// `description`. Free text.
    pub description: String,
    /// `icd10_chapter`. A term of [`vocab::ICD10_CHAPTER`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icd10_chapter: Option<String>,
    /// `chronic`. Yes or no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chronic: Option<bool>,
    /// `autoimmune`. Yes or no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoimmune: Option<bool>,
    /// `diagnosis`. A term of [`vocab::DIAGNOSIS_STATUS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.surgeries`: surgical history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeryValue {
    /// `description`. Free text.
    pub description: String,
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_region: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.injuries`: trauma and fracture sequelae.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjuryValue {
    /// `description`. Free text.
    pub description: String,
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_region: Option<String>,
    /// `fracture`. Yes or no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fracture: Option<bool>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.deformities`: physical deformities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeformityValue {
    /// `description`. Free text.
    pub description: String,
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_region: Option<String>,
    /// `congenital`. Yes or no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub congenital: Option<bool>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.amputations`: amputations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmputationValue {
    /// `body_region`. A term of [`vocab::BODY_REGION`].
    pub body_region: String,
    /// `level`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// `cause`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.prostheses`: prostheses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProsthesisValue {
    /// `kind`. A term of [`vocab::PROSTHESIS_KIND`].
    pub kind: String,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.implants`: biomedical implants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplantValue {
    /// `kind`. A term of [`vocab::IMPLANT_KIND`].
    pub kind: String,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.devices`: pacemakers and intracorporeal devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceValue {
    /// `kind`. A term of [`vocab::DEVICE_KIND`].
    pub kind: String,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.medications`: long-term medication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicationValue {
    /// `substance`. Free text.
    pub substance: String,
    /// `dose`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dose: Option<String>,
    /// `indication`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indication: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.allergies`: drug allergies; food allergies; environmental allergies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllergyValue {
    /// `type`. A term of [`vocab::ALLERGY_TYPE`].
    pub r#type: String,
    /// `allergen`. Free text.
    pub allergen: String,
    /// `severity`. A term of [`vocab::ALLERGY_SEVERITY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// `reaction`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.vaccinations`: vaccination status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaccinationValue {
    /// `pathogen`. A term of [`vocab::PATHOGEN`].
    pub pathogen: String,
    /// `status`. A term of [`vocab::VACCINATION_STATUS`].
    pub status: String,
    /// `dose`. A whole number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dose: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.serology`: serology status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerologyValue {
    /// `pathogen`. A term of [`vocab::PATHOGEN`].
    pub pathogen: String,
    /// `result`. A term of [`vocab::SEROLOGY_RESULT`].
    pub result: String,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.lab_results`: basic metabolic panel; lipid panel; liver panel; renal panel; HbA1c; iron metabolism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabResultValue {
    /// `panel`. A term of [`vocab::LAB_PANEL`].
    pub panel: String,
    /// `analyte`. A term of [`vocab::LAB_ANALYTE`].
    pub analyte: String,
    /// `result`. A number.
    pub result: Number,
    /// `unit`. A term of [`vocab::LAB_UNIT`].
    pub unit: String,
    /// `reference_low`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_low: Option<Number>,
    /// `reference_high`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_high: Option<Number>,
    /// `flag`. A term of [`vocab::LAB_FLAG`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.deficiencies`: vitamin and mineral deficiencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeficiencyValue {
    /// `nutrient`. A term of [`vocab::NUTRIENT`].
    pub nutrient: String,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.sleep_disorders`: sleep disorders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepDisorderValue {
    /// `category`. A term of [`vocab::SLEEP_DISORDER`].
    pub category: String,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `health.mental_health_assessments`: mental health assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentValue {
    /// `instrument`. A term of [`vocab::ASSESSMENT_INSTRUMENT`].
    pub instrument: String,
    /// `score`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<Number>,
    /// `severity`. A term of [`vocab::ASSESSMENT_SEVERITY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// `summary`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.autosomal_mapping`: autosomal DNA mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosomalMappingValue {
    /// `provider`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// `test`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// `reference_build`. A term of [`vocab::REFERENCE_BUILD`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_build: Option<String>,
    /// `snp_count`. A whole number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snp_count: Option<Number>,
    /// `document_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.y_haplogroup` and its siblings: Y haplogroup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaplogroupValue {
    /// `major`. A term of [`vocab::Y_HAPLOGROUP`].
    pub major: String,
    /// `subclade`. A string in the specification's notation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subclade: Option<String>,
    /// `tree_version`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_version: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.whole_genome_sequencing`: whole genome sequencing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencingValue {
    /// `provider`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// `coverage`. A number in x.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Number>,
    /// `reference_build`. A term of [`vocab::REFERENCE_BUILD`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_build: Option<String>,
    /// `file_format`. A term of [`vocab::GENOMIC_FILE_FORMAT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_format: Option<String>,
    /// `document_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.risk_variants`: risk variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskVariantValue {
    /// `variant`. Free text.
    pub variant: String,
    /// `gene`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gene: Option<String>,
    /// `zygosity`. A term of [`vocab::ZYGOSITY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zygosity: Option<String>,
    /// `significance`. A term of [`vocab::CLINICAL_SIGNIFICANCE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub significance: Option<String>,
    /// `condition`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.hereditary_conditions`: hereditary pathologies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HereditaryConditionValue {
    /// `description`. Free text.
    pub description: String,
    /// `inheritance`. A term of [`vocab::INHERITANCE_PATTERN`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inheritance: Option<String>,
    /// `carrier_status`. A term of [`vocab::CARRIER_STATUS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier_status: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.predispositions`: genetic predispositions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredispositionValue {
    /// `description`. Free text.
    pub description: String,
    /// `polygenic_score`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygenic_score: Option<Number>,
    /// `percentile`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentile: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.epigenetic_markers`: epigenetic markers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpigeneticMarkerValue {
    /// `description`. Free text.
    pub description: String,
    /// `marker`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.epigenetic_age`: epigenetic biological age.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpigeneticAgeValue {
    /// `clock`. A term of [`vocab::EPIGENETIC_CLOCK`].
    pub clock: String,
    /// `age_years`. A number in years.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_years: Option<Number>,
    /// `pace`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.gut_microbiome` and its siblings: gut microbiome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrobiomeValue {
    /// `provider`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// `shannon_diversity`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shannon_diversity: Option<Number>,
    /// `summary`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// `document_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `genomics.toxicological_sensitivities`: toxicological sensitivities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxicologicalSensitivityValue {
    /// `substance`. Free text.
    pub substance: String,
    /// `gene`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gene: Option<String>,
    /// `metaboliser_status`. A term of [`vocab::METABOLISER_STATUS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metaboliser_status: Option<String>,
    /// `description`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `death.causes`: direct causes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CauseValue {
    /// `description`. Free text.
    pub description: String,
    /// `icd10_chapter`. A term of [`vocab::ICD10_CHAPTER`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icd10_chapter: Option<String>,
    /// `sequence`. A whole number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<Number>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `death.contributing_factors`: contributing factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributingFactorValue {
    /// `description`. Free text.
    pub description: String,
    /// `icd10_chapter`. A term of [`vocab::ICD10_CHAPTER`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icd10_chapter: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `death.autopsy`: autopsy and forensic report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopsyValue {
    /// `kind`. A term of [`vocab::AUTOPSY`].
    pub kind: String,
    /// `findings`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<String>,
    /// `document_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `death.disposition`: burial or cremation place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispositionValue {
    /// `method`. A term of [`vocab::DISPOSITION`].
    pub method: String,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `death.grave`: grave coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraveValue {
    /// `coordinates`. A position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinates>,
    /// `plot`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plot: Option<String>,
    /// `inscription`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inscription: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `residence.addresses`: principal address; chronological address history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressValue {
    /// `use`. A term of [`vocab::ADDRESS_USE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#use: Option<String>,
    /// `lines`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<String>,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// `postal_code`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// `country`. A term of [`vocab::COUNTRY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// `coordinates`. A position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinates>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `residence.acquired_nationalities`: acquired nationalities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalityValue {
    /// `country`. A term of [`vocab::COUNTRY`].
    pub country: String,
    /// `mode`. A term of [`vocab::NATIONALITY_MODE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `residence.spoken_languages`: spoken languages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpokenLanguageValue {
    /// `language`. A BCP 47 language tag.
    pub language: String,
    /// `proficiency`. A term of [`vocab::LANGUAGE_PROFICIENCY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proficiency: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `education.diplomas`: diplomas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomaValue {
    /// `title`. Free text.
    pub title: String,
    /// `isced_level`. A term of [`vocab::ISCED_LEVEL`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isced_level: Option<String>,
    /// `institution`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub institution: Option<String>,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `education.institutions`: institutions attended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionValue {
    /// `name`. Free text.
    pub name: String,
    /// `isced_level`. A term of [`vocab::ISCED_LEVEL`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isced_level: Option<String>,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `education.income`: average income level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeValue {
    /// `quintile`. A term of [`vocab::INCOME_QUINTILE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quintile: Option<String>,
    /// `amount`. A number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Number>,
    /// `currency`. An ISO 4217 code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// `period`. A term of [`vocab::PAY_PERIOD`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `education.real_estate`: real estate holdings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealEstateValue {
    /// `description`. Free text.
    pub description: String,
    /// `tenure`. A term of [`vocab::TENURE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenure: Option<String>,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// `document_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `military.distinctions`: honorific distinctions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistinctionValue {
    /// `name`. Free text.
    pub name: String,
    /// `kind`. A term of [`vocab::DISTINCTION_KIND`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// `conferred_by`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conferred_by: Option<String>,
    /// `country`. A term of [`vocab::COUNTRY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `military.citations`: military citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationValue {
    /// `text`. Free text.
    pub text: String,
    /// `unit`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `military.ranks`: rank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankValue {
    /// `country`. A term of [`vocab::COUNTRY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// `service`. A term of [`vocab::MILITARY_SERVICE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// `rank`. A term of the rank vocabulary `country` selects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
    /// `rank_text`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_text: Option<String>,
    /// `category`. A term of [`vocab::RANK_CATEGORY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `military.service_numbers`: service number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNumberValue {
    /// `number`. Free text.
    pub number: String,
    /// `service`. A term of [`vocab::MILITARY_SERVICE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// `country`. A term of [`vocab::COUNTRY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `legal.criminal_record`: criminal record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriminalCaseValue {
    /// `offence`. Free text.
    pub offence: String,
    /// `iccs_section`. A term of [`vocab::ICCS_SECTION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iccs_section: Option<String>,
    /// `jurisdiction`. A term of [`vocab::COUNTRY`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
    /// `court`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,
    /// `outcome`. A term of [`vocab::CASE_OUTCOME`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    /// `sentence`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sentence: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `belief.religions`: religion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReligionValue {
    /// `tradition`. A term of [`vocab::RELIGION`].
    pub tradition: String,
    /// `denomination`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denomination: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `belief.sacraments`: sacraments received.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SacramentValue {
    /// `sacrament`. A term of [`vocab::SACRAMENT`].
    pub sacrament: String,
    /// `place_id`. A UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `belief.political_leanings`: political leanings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalLeaningValue {
    /// `position`. A term of [`vocab::POLITICAL_POSITION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// `party`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `belief.memberships`: union and association memberships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipValue {
    /// `organisation`. Free text.
    pub organisation: String,
    /// `kind`. A term of [`vocab::MEMBERSHIP_KIND`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// `role`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `personality.big_five`: personality traits (Big Five).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigFiveValue {
    /// `openness`. A whole number.
    pub openness: Number,
    /// `conscientiousness`. A whole number.
    pub conscientiousness: Number,
    /// `extraversion`. A whole number.
    pub extraversion: Number,
    /// `agreeableness`. A whole number.
    pub agreeableness: Number,
    /// `neuroticism`. A whole number.
    pub neuroticism: Number,
    /// `instrument`. A term of [`vocab::PERSONALITY_INSTRUMENT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `personality.sports`: sport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SportValue {
    /// `sport`. Free text.
    pub sport: String,
    /// `level`. A term of [`vocab::SPORT_LEVEL`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `personality.dietary_habits`: dietary habits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DietValue {
    /// `diet`. A term of [`vocab::DIET`].
    pub diet: String,
    /// `details`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `personality.dependencies`: dependencies and addictions (tobacco, alcohol, substances).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyValue {
    /// `substance`. A term of [`vocab::SUBSTANCE`].
    pub substance: String,
    /// `pattern`. A term of [`vocab::USE_PATTERN`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `value` of `digital_legacy.carbon_footprint`: estimated carbon footprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintValue {
    /// `tonnes_co2e_per_year`. A number in t CO2e/yr.
    pub tonnes_co2e_per_year: Number,
    /// `method`. Free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `civil_status` block: Civil status (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CivilStatus {
    /// Birth certificate number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_certificate_number: Option<Claim<String>>,
    /// Civil register entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub register_entries: Vec<Claim<RegisterEntryValue>>,
    /// Marginal annotations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marginal_annotations: Vec<Claim<MarginalAnnotationValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `morphology` block: Morphology (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Morphology {
    /// Exact height.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub height: Vec<Claim<Number>>,
    /// Weight over time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weight: Vec<Claim<Number>>,
    /// BMI.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bmi: Vec<Claim<Number>>,
    /// Body composition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_composition: Vec<Claim<BodyCompositionValue>>,
    /// Build.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build: Vec<Claim<String>>,
    /// Precise eye colour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eye_colour: Vec<Claim<String>>,
    /// Eye shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eye_shape: Vec<Claim<String>>,
    /// Eye spacing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eye_spacing: Vec<Claim<String>>,
    /// Natural hair colour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hair_colour: Vec<Claim<String>>,
    /// Hair texture.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hair_texture: Vec<Claim<String>>,
    /// Hairline.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hairline: Vec<Claim<String>>,
    /// Facial hair.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facial_hair: Vec<Claim<String>>,
    /// Body hair.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_hair: Vec<Claim<String>>,
    /// Skin tone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_tone: Option<Claim<String>>,
    /// Skin undertone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_undertone: Option<Claim<String>>,
    /// Freckles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub freckles: Vec<Claim<String>>,
    /// Pigmentation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pigmentation: Vec<Claim<PigmentationValue>>,
    /// Scars.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scars: Vec<Claim<ScarValue>>,
    /// Tattoos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tattoos: Vec<Claim<TattooValue>>,
    /// Moles with location and shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moles: Vec<Claim<MoleValue>>,
    /// Facial asymmetries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facial_asymmetries: Vec<Claim<String>>,
    /// Facial bone structure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub face_shape: Vec<Claim<String>>,
    /// Nose shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nose_shape: Vec<Claim<String>>,
    /// Ear shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ear_shape: Vec<Claim<String>>,
    /// Lip shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lip_shape: Vec<Claim<String>>,
    /// Dentition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dentition: Vec<Claim<String>>,
    /// Dental malocclusion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub malocclusion: Vec<Claim<String>>,
    /// Posture.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub posture: Vec<Claim<String>>,
    /// Gait.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gait: Vec<Claim<String>>,
    /// Distinguishing features.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distinguishing_features: Vec<Claim<String>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `biometrics` block: Biometrics (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Biometrics {
    /// Fingerprints. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fingerprints: Vec<Claim<ArtefactRef>>,
    /// Retinal print. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retinal_print: Vec<Claim<ArtefactRef>>,
    /// Voice signature. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_signature: Vec<Claim<ArtefactRef>>,
    /// Fundamental voice frequency. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_frequency: Vec<Claim<Number>>,
    /// Vocal timbre. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vocal_timbre: Vec<Claim<String>>,
    /// Spoken accent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spoken_accent: Vec<Claim<AccentValue>>,
    /// Speech rate. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speech_rate: Vec<Claim<Number>>,
    /// Verbal tics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verbal_tics: Vec<Claim<String>>,
    /// Frequent vocabulary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frequent_vocabulary: Vec<Claim<String>>,
    /// Register of speech.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speech_register: Vec<Claim<String>>,
    /// Motor tics. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub motor_tics: Vec<Claim<String>>,
    /// Handedness.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handedness: Vec<Claim<String>>,
    /// Hearing capacity. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hearing: Vec<Claim<HearingValue>>,
    /// Visual acuity. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_acuity: Vec<Claim<VisualAcuityValue>>,
    /// Optical correction. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optical_correction: Vec<Claim<OpticalCorrectionValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `health` block: Health (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Health {
    /// Blood group. Class `health`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blood_group: Option<Claim<String>>,
    /// Rhesus. Class `health`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rhesus: Option<Claim<String>>,
    /// Mean blood pressure. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blood_pressure: Vec<Claim<BloodPressureValue>>,
    /// Resting heart rate. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resting_heart_rate: Vec<Claim<Number>>,
    /// Respiratory capacity (FEV1/FVC). Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub respiratory_capacity: Vec<Claim<RespiratoryCapacityValue>>,
    /// Chronic pathologies; diagnosed chronic illnesses; cardiovascular, respiratory, metabolic and endocrine pathologies; autoimmune diseases; oncological pathologies; psychiatric and neurodevelopmental conditions. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Claim<ConditionValue>>,
    /// Surgical history. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surgeries: Vec<Claim<SurgeryValue>>,
    /// Trauma and fracture sequelae. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub injuries: Vec<Claim<InjuryValue>>,
    /// Physical deformities. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deformities: Vec<Claim<DeformityValue>>,
    /// Amputations. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub amputations: Vec<Claim<AmputationValue>>,
    /// Prostheses. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prostheses: Vec<Claim<ProsthesisValue>>,
    /// Biomedical implants. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implants: Vec<Claim<ImplantValue>>,
    /// Pacemakers and intracorporeal devices. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<Claim<DeviceValue>>,
    /// Long-term medication. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub medications: Vec<Claim<MedicationValue>>,
    /// Drug allergies; food allergies; environmental allergies. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allergies: Vec<Claim<AllergyValue>>,
    /// Vaccination status. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vaccinations: Vec<Claim<VaccinationValue>>,
    /// Serology status. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serology: Vec<Claim<SerologyValue>>,
    /// Basic metabolic panel; lipid panel; liver panel; renal panel; HbA1c; iron metabolism. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lab_results: Vec<Claim<LabResultValue>>,
    /// Vitamin and mineral deficiencies. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deficiencies: Vec<Claim<DeficiencyValue>>,
    /// Sleep disorders. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sleep_disorders: Vec<Claim<SleepDisorderValue>>,
    /// Mental health assessment. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mental_health_assessments: Vec<Claim<AssessmentValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `genomics` block: Genomics (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Genomics {
    /// Autosomal DNA mapping. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub autosomal_mapping: Vec<Claim<AutosomalMappingValue>>,
    /// Y haplogroup. Class `genomics`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_haplogroup: Option<Claim<HaplogroupValue>>,
    /// Mitochondrial haplogroup. Class `genomics`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mt_haplogroup: Option<Claim<HaplogroupValue>>,
    /// Whole genome sequencing. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub whole_genome_sequencing: Vec<Claim<SequencingValue>>,
    /// Risk variants. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risk_variants: Vec<Claim<RiskVariantValue>>,
    /// Hereditary pathologies. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hereditary_conditions: Vec<Claim<HereditaryConditionValue>>,
    /// Genetic predispositions. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub predispositions: Vec<Claim<PredispositionValue>>,
    /// Epigenetic markers. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub epigenetic_markers: Vec<Claim<EpigeneticMarkerValue>>,
    /// Epigenetic biological age. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub epigenetic_age: Vec<Claim<EpigeneticAgeValue>>,
    /// Gut microbiome. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gut_microbiome: Vec<Claim<MicrobiomeValue>>,
    /// Skin microbiome. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skin_microbiome: Vec<Claim<MicrobiomeValue>>,
    /// Toxicological sensitivities. Class `genomics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub toxicological_sensitivities: Vec<Claim<ToxicologicalSensitivityValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `residence` block: Residence and nationality (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Residence {
    /// Principal address; chronological address history.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<Claim<AddressValue>>,
    /// Nationality of origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nationality_of_origin: Option<Claim<String>>,
    /// Acquired nationalities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acquired_nationalities: Vec<Claim<NationalityValue>>,
    /// Mother tongue.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mother_tongue: Vec<Claim<String>>,
    /// Spoken languages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spoken_languages: Vec<Claim<SpokenLanguageValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `education` block: Education and work (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Education {
    /// Education level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level: Vec<Claim<String>>,
    /// Diplomas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diplomas: Vec<Claim<DiplomaValue>>,
    /// Institutions attended.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub institutions: Vec<Claim<InstitutionValue>>,
    /// Average income level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub income: Vec<Claim<IncomeValue>>,
    /// Real estate holdings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub real_estate: Vec<Claim<RealEstateValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `military` block: Military and honours (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Military {
    /// Honorific distinctions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distinctions: Vec<Claim<DistinctionValue>>,
    /// Military citations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Claim<CitationValue>>,
    /// Rank.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranks: Vec<Claim<RankValue>>,
    /// Regiment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<Claim<String>>,
    /// Service number.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service_numbers: Vec<Claim<ServiceNumberValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `legal` block: Legal (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Legal {
    /// Criminal record. Class `legal`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub criminal_record: Vec<Claim<CriminalCaseValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `belief` block: Belief and affiliation (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Belief {
    /// Religion. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub religions: Vec<Claim<ReligionValue>>,
    /// Sacraments received. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sacraments: Vec<Claim<SacramentValue>>,
    /// Individual beliefs. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beliefs: Vec<Claim<String>>,
    /// Political leanings. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub political_leanings: Vec<Claim<PoliticalLeaningValue>>,
    /// Union and association memberships. Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memberships: Vec<Claim<MembershipValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `personality` block: Personality and behaviour (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Personality {
    /// Personality traits (Big Five).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub big_five: Vec<Claim<BigFiveValue>>,
    /// Personality traits (MBTI).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mbti: Vec<Claim<String>>,
    /// Introversion/extraversion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub introversion_extraversion: Vec<Claim<String>>,
    /// Stress tolerance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stress_tolerance: Vec<Claim<String>>,
    /// Decision-making style.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_style: Vec<Claim<String>>,
    /// Interests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interests: Vec<Claim<String>>,
    /// Hobbies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hobbies: Vec<Claim<String>>,
    /// Sport.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sports: Vec<Claim<SportValue>>,
    /// Dietary habits.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dietary_habits: Vec<Claim<DietValue>>,
    /// Dependencies and addictions (tobacco, alcohol, substances). Class `health`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Claim<DependencyValue>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The `digital_legacy` block: Digital legacy (SPEC_1.1 §5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DigitalLegacy {
    /// Source morphological 3D model (mesh or point cloud). Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_models: Vec<Claim<ArtefactRef>>,
    /// HD skin texture map. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skin_textures: Vec<Claim<ArtefactRef>>,
    /// Skeletal and articular rigging. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rigs: Vec<Claim<ArtefactRef>>,
    /// Source audio for voice synthesis. Class `biometrics`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_corpora: Vec<Claim<ArtefactRef>>,
    /// Written corpus for LLM training.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_corpora: Vec<Claim<ArtefactRef>>,
    /// Digital trace history.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub digital_traces: Vec<Claim<ArtefactRef>>,
    /// Estimated carbon footprint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carbon_footprint: Vec<Claim<CarbonFootprintValue>>,
    /// AI-driven reactive behaviour model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviour_models: Vec<Claim<ArtefactRef>>,
    /// Forward-compatible extras.
    #[serde(flatten)]
    pub extra: Extra,
}
