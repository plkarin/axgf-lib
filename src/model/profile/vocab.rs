// SPDX-License-Identifier: Apache-2.0
//! The closed vocabularies of AXGF 1.1 (SPEC_1.1 §6), as data.
//!
//! One constant per vocabulary, named after it: [`BLOOD_GROUP`] is
//! `$defs/vocab_blood_group` in `schema/axgf-1.1.schema.json`. The terms are
//! identifiers, not labels — an interface translates them for display and
//! stores the term. `tests/profile_registry.rs` holds every constant here
//! against the embedded schema's enumerations, so the two cannot drift.

/// A closed vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vocabulary {
    /// Its name: the schema defines it as `$defs/vocab_<name>`.
    pub name: &'static str,
    /// Its title in the specification, in English.
    pub title: &'static str,
    /// Every term, in the specification's order.
    pub terms: &'static [&'static str],
}

impl Vocabulary {
    /// Whether `term` is one of this vocabulary's terms. Exact and
    /// case-sensitive: `a` is not the blood group `A`.
    pub fn contains(&self, term: &str) -> bool {
        self.terms.contains(&term)
    }
}

/// Sensitive class. AXGF 1.1 §4.
pub const SENSITIVE_CLASS: Vocabulary = Vocabulary {
    name: "sensitive_class",
    title: "Sensitive class",
    terms: &["health", "biometrics", "genomics", "legal"],
};

/// Laterality.
pub const LATERALITY: Vocabulary = Vocabulary {
    name: "laterality",
    title: "Laterality",
    terms: &["left", "right", "both"],
};

/// Body region.
pub const BODY_REGION: Vocabulary = Vocabulary {
    name: "body_region",
    title: "Body region",
    terms: &[
        "head",
        "face",
        "neck",
        "left_shoulder",
        "right_shoulder",
        "left_arm",
        "right_arm",
        "left_hand",
        "right_hand",
        "chest",
        "abdomen",
        "upper_back",
        "lower_back",
        "pelvis",
        "left_leg",
        "right_leg",
        "left_foot",
        "right_foot",
        "internal",
        "whole_body",
        "other",
    ],
};

/// Artefact type.
pub const ARTEFACT_TYPE: Vocabulary = Vocabulary {
    name: "artefact_type",
    title: "Artefact type",
    terms: &[
        "mesh",
        "point_cloud",
        "skin_texture_map",
        "skeletal_rig",
        "voice_corpus",
        "text_corpus",
        "trace_archive",
        "behaviour_model",
        "fingerprint_card",
        "fingerprint_template",
        "retinal_image",
        "voiceprint",
    ],
};

/// Consent.
pub const CONSENT: Vocabulary = Vocabulary {
    name: "consent",
    title: "Consent",
    terms: &[
        "given",
        "given_by_estate",
        "refused",
        "withdrawn",
        "not_asked",
        "unknown",
    ],
};

/// Sex at birth.
pub const SEX_AT_BIRTH: Vocabulary = Vocabulary {
    name: "sex_at_birth",
    title: "Sex at birth",
    terms: &["female", "male", "intersex", "undetermined", "unknown"],
};

/// Gender identity.
pub const GENDER_IDENTITY: Vocabulary = Vocabulary {
    name: "gender_identity",
    title: "Gender identity",
    terms: &[
        "woman",
        "man",
        "non_binary",
        "other",
        "undisclosed",
        "unknown",
    ],
};

/// Title kind.
pub const TITLE_KIND: Vocabulary = Vocabulary {
    name: "title_kind",
    title: "Title kind",
    terms: &[
        "nobility",
        "academic",
        "professional",
        "religious",
        "military",
        "civic",
        "courtesy",
        "other",
    ],
};

/// Register entry type.
pub const REGISTER_TYPE: Vocabulary = Vocabulary {
    name: "register_type",
    title: "Register entry type",
    terms: &[
        "birth",
        "baptism",
        "marriage",
        "death",
        "burial",
        "divorce",
        "recognition",
        "legitimation",
        "adoption",
        "name_change",
        "other",
    ],
};

/// Build.
pub const BUILD: Vocabulary = Vocabulary {
    name: "build",
    title: "Build",
    terms: &["slight", "slim", "average", "sturdy", "stout", "heavy"],
};

/// Eye colour.
pub const EYE_COLOUR: Vocabulary = Vocabulary {
    name: "eye_colour",
    title: "Eye colour",
    terms: &[
        "light_blue",
        "blue",
        "dark_blue",
        "grey",
        "blue_grey",
        "green",
        "grey_green",
        "hazel",
        "amber",
        "light_brown",
        "brown",
        "dark_brown",
        "black",
        "mixed",
        "other",
    ],
};

/// Eye shape.
pub const EYE_SHAPE: Vocabulary = Vocabulary {
    name: "eye_shape",
    title: "Eye shape",
    terms: &[
        "almond",
        "round",
        "hooded",
        "monolid",
        "deep_set",
        "protruding",
        "upturned",
        "downturned",
        "other",
    ],
};

/// Eye spacing.
pub const EYE_SPACING: Vocabulary = Vocabulary {
    name: "eye_spacing",
    title: "Eye spacing",
    terms: &["close_set", "average", "wide_set"],
};

/// Natural hair colour.
pub const HAIR_COLOUR: Vocabulary = Vocabulary {
    name: "hair_colour",
    title: "Natural hair colour",
    terms: &[
        "black",
        "dark_brown",
        "brown",
        "light_brown",
        "auburn",
        "red",
        "strawberry_blond",
        "dark_blond",
        "blond",
        "light_blond",
        "grey",
        "white",
        "none",
        "other",
    ],
};

/// Hair texture.
pub const HAIR_TEXTURE: Vocabulary = Vocabulary {
    name: "hair_texture",
    title: "Hair texture",
    terms: &["straight", "wavy", "curly", "coily", "other"],
};

/// Hairline.
pub const HAIRLINE: Vocabulary = Vocabulary {
    name: "hairline",
    title: "Hairline",
    terms: &[
        "straight",
        "rounded",
        "widows_peak",
        "m_shaped",
        "bell_shaped",
        "uneven",
        "receding",
        "bald",
    ],
};

/// Facial hair.
pub const FACIAL_HAIR: Vocabulary = Vocabulary {
    name: "facial_hair",
    title: "Facial hair",
    terms: &[
        "none",
        "stubble",
        "moustache",
        "goatee",
        "full_beard",
        "sideburns",
        "other",
    ],
};

/// Body hair.
pub const BODY_HAIR: Vocabulary = Vocabulary {
    name: "body_hair",
    title: "Body hair",
    terms: &["none", "sparse", "moderate", "dense"],
};

/// Skin tone (Fitzpatrick phototype). Fitzpatrick, T. B. (1988), The validity and practicality of sun-reactive skin types I through VI.
pub const SKIN_TONE: Vocabulary = Vocabulary {
    name: "skin_tone",
    title: "Skin tone (Fitzpatrick phototype)",
    terms: &[
        "type_i", "type_ii", "type_iii", "type_iv", "type_v", "type_vi",
    ],
};

/// Skin undertone.
pub const SKIN_UNDERTONE: Vocabulary = Vocabulary {
    name: "skin_undertone",
    title: "Skin undertone",
    terms: &["cool", "neutral", "warm", "olive"],
};

/// Freckles.
pub const FRECKLES: Vocabulary = Vocabulary {
    name: "freckles",
    title: "Freckles",
    terms: &["none", "few", "moderate", "many"],
};

/// Pigmentation mark.
pub const PIGMENTATION_MARK: Vocabulary = Vocabulary {
    name: "pigmentation_mark",
    title: "Pigmentation mark",
    terms: &[
        "birthmark",
        "port_wine_stain",
        "cafe_au_lait_spot",
        "depigmented_patch",
        "hyperpigmented_patch",
        "other",
    ],
};

/// Mole shape.
pub const MOLE_SHAPE: Vocabulary = Vocabulary {
    name: "mole_shape",
    title: "Mole shape",
    terms: &["round", "oval", "irregular", "other"],
};

/// Face shape.
pub const FACE_SHAPE: Vocabulary = Vocabulary {
    name: "face_shape",
    title: "Face shape",
    terms: &[
        "oval",
        "round",
        "square",
        "oblong",
        "heart",
        "diamond",
        "triangular",
    ],
};

/// Nose shape.
pub const NOSE_SHAPE: Vocabulary = Vocabulary {
    name: "nose_shape",
    title: "Nose shape",
    terms: &[
        "straight", "aquiline", "snub", "upturned", "flat", "broad", "bulbous", "crooked", "other",
    ],
};

/// Ear shape.
pub const EAR_SHAPE: Vocabulary = Vocabulary {
    name: "ear_shape",
    title: "Ear shape",
    terms: &[
        "free_lobe",
        "attached_lobe",
        "protruding",
        "close_set",
        "pointed",
        "other",
    ],
};

/// Lip shape.
pub const LIP_SHAPE: Vocabulary = Vocabulary {
    name: "lip_shape",
    title: "Lip shape",
    terms: &[
        "thin",
        "medium",
        "full",
        "bow_shaped",
        "wide",
        "downturned",
        "other",
    ],
};

/// Dentition.
pub const DENTITION: Vocabulary = Vocabulary {
    name: "dentition",
    title: "Dentition",
    terms: &[
        "primary",
        "mixed",
        "permanent_complete",
        "permanent_partial_loss",
        "edentulous",
        "partial_denture",
        "full_denture",
        "implants",
    ],
};

/// Malocclusion (Angle class). Angle, E. H. (1899), Classification of malocclusion.
pub const MALOCCLUSION: Vocabulary = Vocabulary {
    name: "malocclusion",
    title: "Malocclusion (Angle class)",
    terms: &[
        "normal",
        "class_i",
        "class_ii_division_1",
        "class_ii_division_2",
        "class_iii",
    ],
};

/// Posture. Kendall, McCreary et al., Muscles: Testing and Function (postural types).
pub const POSTURE: Vocabulary = Vocabulary {
    name: "posture",
    title: "Posture",
    terms: &[
        "ideal",
        "kyphotic_lordotic",
        "flat_back",
        "sway_back",
        "stooped",
        "scoliotic",
        "other",
    ],
};

/// Gait.
pub const GAIT: Vocabulary = Vocabulary {
    name: "gait",
    title: "Gait",
    terms: &[
        "brisk",
        "average",
        "slow",
        "shuffling",
        "limping",
        "waddling",
        "unsteady",
        "stiff",
        "other",
    ],
};

/// Vocal timbre.
pub const VOCAL_TIMBRE: Vocabulary = Vocabulary {
    name: "vocal_timbre",
    title: "Vocal timbre",
    terms: &[
        "bright", "dark", "warm", "breathy", "nasal", "hoarse", "resonant", "thin", "other",
    ],
};

/// Register of speech. Joos, M. (1961), The Five Clocks.
pub const SPEECH_REGISTER: Vocabulary = Vocabulary {
    name: "speech_register",
    title: "Register of speech",
    terms: &["frozen", "formal", "consultative", "casual", "intimate"],
};

/// Handedness.
pub const HANDEDNESS: Vocabulary = Vocabulary {
    name: "handedness",
    title: "Handedness",
    terms: &["left", "right", "ambidextrous", "mixed", "unknown"],
};

/// Hearing grade. World Health Organization (2021), World report on hearing.
pub const HEARING_GRADE: Vocabulary = Vocabulary {
    name: "hearing_grade",
    title: "Hearing grade",
    terms: &[
        "normal",
        "mild",
        "moderate",
        "moderately_severe",
        "severe",
        "profound",
        "complete",
    ],
};

/// Optical correction.
pub const OPTICAL_CORRECTION: Vocabulary = Vocabulary {
    name: "optical_correction",
    title: "Optical correction",
    terms: &[
        "none",
        "glasses",
        "contact_lenses",
        "glasses_and_contact_lenses",
        "refractive_surgery",
        "intraocular_lens",
        "other",
    ],
};

/// Blood group (ABO). ISBT ABO blood group system.
pub const BLOOD_GROUP: Vocabulary = Vocabulary {
    name: "blood_group",
    title: "Blood group (ABO)",
    terms: &["A", "B", "AB", "O"],
};

/// Rhesus (RhD). ISBT Rh blood group system.
pub const RHESUS: Vocabulary = Vocabulary {
    name: "rhesus",
    title: "Rhesus (RhD)",
    terms: &["positive", "negative", "weak_d", "unknown"],
};

/// ICD-10 chapter. WHO International Statistical Classification of Diseases and Related Health Problems, 10th revision.
pub const ICD10_CHAPTER: Vocabulary = Vocabulary {
    name: "icd10_chapter",
    title: "ICD-10 chapter",
    terms: &[
        "infectious_parasitic",
        "neoplasms",
        "blood_immune",
        "endocrine_metabolic",
        "mental_behavioural",
        "nervous_system",
        "eye_adnexa",
        "ear_mastoid",
        "circulatory",
        "respiratory",
        "digestive",
        "skin",
        "musculoskeletal",
        "genitourinary",
        "pregnancy_childbirth",
        "perinatal",
        "congenital",
        "ill_defined",
        "injury_poisoning",
        "external_causes",
        "health_factors",
        "special_purposes",
    ],
};

/// Diagnosis status.
pub const DIAGNOSIS_STATUS: Vocabulary = Vocabulary {
    name: "diagnosis_status",
    title: "Diagnosis status",
    terms: &["diagnosed", "suspected", "self_reported", "unknown"],
};

/// Prosthesis.
pub const PROSTHESIS_KIND: Vocabulary = Vocabulary {
    name: "prosthesis_kind",
    title: "Prosthesis",
    terms: &[
        "limb", "joint", "ocular", "dental", "auditory", "breast", "other",
    ],
};

/// Biomedical implant.
pub const IMPLANT_KIND: Vocabulary = Vocabulary {
    name: "implant_kind",
    title: "Biomedical implant",
    terms: &[
        "orthopaedic",
        "dental",
        "cochlear",
        "breast",
        "intraocular_lens",
        "contraceptive",
        "cosmetic",
        "other",
    ],
};

/// Intracorporeal device.
pub const DEVICE_KIND: Vocabulary = Vocabulary {
    name: "device_kind",
    title: "Intracorporeal device",
    terms: &[
        "pacemaker",
        "implantable_defibrillator",
        "cardiac_resynchronisation",
        "ventricular_assist",
        "neurostimulator",
        "insulin_pump",
        "drug_port",
        "shunt",
        "stent",
        "other",
    ],
};

/// Allergy type.
pub const ALLERGY_TYPE: Vocabulary = Vocabulary {
    name: "allergy_type",
    title: "Allergy type",
    terms: &[
        "drug",
        "food",
        "environmental",
        "insect_venom",
        "latex",
        "other",
    ],
};

/// Allergy severity.
pub const ALLERGY_SEVERITY: Vocabulary = Vocabulary {
    name: "allergy_severity",
    title: "Allergy severity",
    terms: &["mild", "moderate", "severe", "anaphylactic", "unknown"],
};

/// Pathogen or vaccine target.
pub const PATHOGEN: Vocabulary = Vocabulary {
    name: "pathogen",
    title: "Pathogen or vaccine target",
    terms: &[
        "diphtheria",
        "tetanus",
        "pertussis",
        "poliomyelitis",
        "measles",
        "mumps",
        "rubella",
        "varicella",
        "smallpox",
        "tuberculosis",
        "hepatitis_a",
        "hepatitis_b",
        "hepatitis_c",
        "haemophilus_influenzae_b",
        "pneumococcal",
        "meningococcal",
        "human_papillomavirus",
        "influenza",
        "covid_19",
        "rotavirus",
        "yellow_fever",
        "typhoid",
        "cholera",
        "rabies",
        "japanese_encephalitis",
        "tick_borne_encephalitis",
        "hiv",
        "syphilis",
        "toxoplasmosis",
        "cytomegalovirus",
        "epstein_barr",
        "other",
    ],
};

/// Vaccination status.
pub const VACCINATION_STATUS: Vocabulary = Vocabulary {
    name: "vaccination_status",
    title: "Vaccination status",
    terms: &[
        "vaccinated",
        "partially_vaccinated",
        "unvaccinated",
        "contraindicated",
        "unknown",
    ],
};

/// Serology result.
pub const SEROLOGY_RESULT: Vocabulary = Vocabulary {
    name: "serology_result",
    title: "Serology result",
    terms: &["positive", "negative", "equivocal", "unknown"],
};

/// Laboratory panel.
pub const LAB_PANEL: Vocabulary = Vocabulary {
    name: "lab_panel",
    title: "Laboratory panel",
    terms: &[
        "basic_metabolic",
        "lipid",
        "liver",
        "renal",
        "glycated_haemoglobin",
        "iron",
    ],
};

/// Laboratory analyte.
pub const LAB_ANALYTE: Vocabulary = Vocabulary {
    name: "lab_analyte",
    title: "Laboratory analyte",
    terms: &[
        "sodium",
        "potassium",
        "chloride",
        "bicarbonate",
        "urea",
        "creatinine",
        "glucose",
        "calcium",
        "total_cholesterol",
        "ldl_cholesterol",
        "hdl_cholesterol",
        "triglycerides",
        "non_hdl_cholesterol",
        "alt",
        "ast",
        "alp",
        "ggt",
        "total_bilirubin",
        "direct_bilirubin",
        "albumin",
        "total_protein",
        "egfr",
        "uric_acid",
        "phosphate",
        "urine_albumin_creatinine_ratio",
        "hba1c",
        "serum_iron",
        "ferritin",
        "transferrin",
        "transferrin_saturation",
        "tibc",
    ],
};

/// Laboratory unit (UCUM). Unified Code for Units of Measure.
pub const LAB_UNIT: Vocabulary = Vocabulary {
    name: "lab_unit",
    title: "Laboratory unit (UCUM)",
    terms: &[
        "mmol/L",
        "umol/L",
        "mg/dL",
        "g/L",
        "g/dL",
        "U/L",
        "%",
        "mmol/mol",
        "ug/L",
        "ug/dL",
        "ng/mL",
        "mL/min/{1.73_m2}",
        "mg/mmol",
        "mg/g",
    ],
};

/// Laboratory flag.
pub const LAB_FLAG: Vocabulary = Vocabulary {
    name: "lab_flag",
    title: "Laboratory flag",
    terms: &["low", "normal", "high", "critical_low", "critical_high"],
};

/// Nutrient.
pub const NUTRIENT: Vocabulary = Vocabulary {
    name: "nutrient",
    title: "Nutrient",
    terms: &[
        "vitamin_a",
        "thiamine",
        "riboflavin",
        "niacin",
        "vitamin_b6",
        "folate",
        "vitamin_b12",
        "vitamin_c",
        "vitamin_d",
        "vitamin_e",
        "vitamin_k",
        "iron",
        "zinc",
        "magnesium",
        "calcium",
        "iodine",
        "selenium",
        "copper",
        "potassium",
        "phosphorus",
        "other",
    ],
};

/// Sleep disorder category. American Academy of Sleep Medicine, International Classification of Sleep Disorders, 3rd edition.
pub const SLEEP_DISORDER: Vocabulary = Vocabulary {
    name: "sleep_disorder",
    title: "Sleep disorder category",
    terms: &[
        "insomnia",
        "sleep_related_breathing",
        "central_hypersomnolence",
        "circadian_rhythm",
        "parasomnia",
        "sleep_related_movement",
        "other",
    ],
};

/// Assessment instrument.
pub const ASSESSMENT_INSTRUMENT: Vocabulary = Vocabulary {
    name: "assessment_instrument",
    title: "Assessment instrument",
    terms: &[
        "phq_9",
        "gad_7",
        "bdi_ii",
        "hads",
        "k10",
        "gds_15",
        "mmse",
        "moca",
        "audit",
        "clinical_interview",
        "other",
    ],
};

/// Assessment severity.
pub const ASSESSMENT_SEVERITY: Vocabulary = Vocabulary {
    name: "assessment_severity",
    title: "Assessment severity",
    terms: &[
        "none_minimal",
        "mild",
        "moderate",
        "moderately_severe",
        "severe",
    ],
};

/// Reference genome build. Genome Reference Consortium; Telomere-to-Telomere Consortium.
pub const REFERENCE_BUILD: Vocabulary = Vocabulary {
    name: "reference_build",
    title: "Reference genome build",
    terms: &["grch36", "grch37", "grch38", "t2t_chm13"],
};

/// Genomic file format.
pub const GENOMIC_FILE_FORMAT: Vocabulary = Vocabulary {
    name: "genomic_file_format",
    title: "Genomic file format",
    terms: &[
        "raw_microarray",
        "fastq",
        "bam",
        "cram",
        "vcf",
        "gvcf",
        "other",
    ],
};

/// Y-DNA major haplogroup. ISOGG Y-DNA Haplogroup Tree.
pub const Y_HAPLOGROUP: Vocabulary = Vocabulary {
    name: "y_haplogroup",
    title: "Y-DNA major haplogroup",
    terms: &[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T",
    ],
};

/// mtDNA major haplogroup. PhyloTree (the mtDNA nomenclature ISOGG adopts).
pub const MT_HAPLOGROUP: Vocabulary = Vocabulary {
    name: "mt_haplogroup",
    title: "mtDNA major haplogroup",
    terms: &[
        "L0", "L1", "L2", "L3", "L4", "L5", "L6", "M", "N", "R", "A", "B", "C", "D", "E", "F", "G",
        "H", "HV", "I", "J", "K", "O", "P", "Q", "S", "T", "U", "V", "W", "X", "Y", "Z",
    ],
};

/// Zygosity.
pub const ZYGOSITY: Vocabulary = Vocabulary {
    name: "zygosity",
    title: "Zygosity",
    terms: &[
        "heterozygous",
        "homozygous",
        "hemizygous",
        "compound_heterozygous",
    ],
};

/// Clinical significance. Richards et al. (2015), ACMG/AMP standards for the interpretation of sequence variants.
pub const CLINICAL_SIGNIFICANCE: Vocabulary = Vocabulary {
    name: "clinical_significance",
    title: "Clinical significance",
    terms: &[
        "pathogenic",
        "likely_pathogenic",
        "uncertain_significance",
        "likely_benign",
        "benign",
    ],
};

/// Inheritance pattern.
pub const INHERITANCE_PATTERN: Vocabulary = Vocabulary {
    name: "inheritance_pattern",
    title: "Inheritance pattern",
    terms: &[
        "autosomal_dominant",
        "autosomal_recessive",
        "x_linked_dominant",
        "x_linked_recessive",
        "y_linked",
        "mitochondrial",
        "multifactorial",
        "unknown",
    ],
};

/// Carrier status.
pub const CARRIER_STATUS: Vocabulary = Vocabulary {
    name: "carrier_status",
    title: "Carrier status",
    terms: &["affected", "carrier", "not_carrier", "unknown"],
};

/// Epigenetic clock.
pub const EPIGENETIC_CLOCK: Vocabulary = Vocabulary {
    name: "epigenetic_clock",
    title: "Epigenetic clock",
    terms: &[
        "horvath",
        "hannum",
        "phenoage",
        "grimage",
        "dunedinpace",
        "other",
    ],
};

/// Metaboliser status. Clinical Pharmacogenetics Implementation Consortium (CPIC) standardised terms.
pub const METABOLISER_STATUS: Vocabulary = Vocabulary {
    name: "metaboliser_status",
    title: "Metaboliser status",
    terms: &["poor", "intermediate", "normal", "rapid", "ultrarapid"],
};

/// Autopsy.
pub const AUTOPSY: Vocabulary = Vocabulary {
    name: "autopsy",
    title: "Autopsy",
    terms: &[
        "not_performed",
        "clinical",
        "forensic",
        "external_examination",
        "unknown",
    ],
};

/// Disposition of the body.
pub const DISPOSITION: Vocabulary = Vocabulary {
    name: "disposition",
    title: "Disposition of the body",
    terms: &[
        "burial",
        "cremation",
        "entombment",
        "burial_at_sea",
        "natural_burial",
        "body_donation",
        "other",
        "unknown",
    ],
};

/// Address use.
pub const ADDRESS_USE: Vocabulary = Vocabulary {
    name: "address_use",
    title: "Address use",
    terms: &["principal", "secondary", "temporary", "postal", "other"],
};

/// Country. ISO 3166-1 alpha-2, plus the historical codes of AXGF 1.0 §6.5.
pub const COUNTRY: Vocabulary = Vocabulary {
    name: "country",
    title: "Country",
    terms: &[
        "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX",
        "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ",
        "BR", "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK",
        "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM",
        "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR",
        "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS",
        "GT", "GU", "GW", "GY", "HK", "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN",
        "IO", "IQ", "IR", "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN",
        "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV",
        "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK", "ML", "MM", "MN", "MO", "MP", "MQ",
        "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI",
        "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM",
        "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW", "SA", "SB", "SC",
        "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV",
        "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR",
        "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
        "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW", "SU", "DD", "YU", "CS", "OT",
    ],
};

/// How a nationality was acquired.
pub const NATIONALITY_MODE: Vocabulary = Vocabulary {
    name: "nationality_mode",
    title: "How a nationality was acquired",
    terms: &[
        "descent",
        "birth_in_territory",
        "naturalisation",
        "marriage",
        "registration",
        "restoration",
        "state_succession",
        "other",
    ],
};

/// Language proficiency. Council of Europe, Common European Framework of Reference for Languages.
pub const LANGUAGE_PROFICIENCY: Vocabulary = Vocabulary {
    name: "language_proficiency",
    title: "Language proficiency",
    terms: &["a1", "a2", "b1", "b2", "c1", "c2", "native"],
};

/// Education level (ISCED 2011). UNESCO Institute for Statistics, International Standard Classification of Education 2011.
pub const ISCED_LEVEL: Vocabulary = Vocabulary {
    name: "isced_level",
    title: "Education level (ISCED 2011)",
    terms: &[
        "isced_0", "isced_1", "isced_2", "isced_3", "isced_4", "isced_5", "isced_6", "isced_7",
        "isced_8",
    ],
};

/// Income quintile.
pub const INCOME_QUINTILE: Vocabulary = Vocabulary {
    name: "income_quintile",
    title: "Income quintile",
    terms: &["q1", "q2", "q3", "q4", "q5"],
};

/// Pay period.
pub const PAY_PERIOD: Vocabulary = Vocabulary {
    name: "pay_period",
    title: "Pay period",
    terms: &["hourly", "daily", "weekly", "monthly", "annual"],
};

/// Tenure.
pub const TENURE: Vocabulary = Vocabulary {
    name: "tenure",
    title: "Tenure",
    terms: &[
        "owned",
        "co_owned",
        "leasehold",
        "rented",
        "usufruct",
        "other",
    ],
};

/// Distinction.
pub const DISTINCTION_KIND: Vocabulary = Vocabulary {
    name: "distinction_kind",
    title: "Distinction",
    terms: &["order", "decoration", "medal", "title", "other"],
};

/// Service.
pub const MILITARY_SERVICE: Vocabulary = Vocabulary {
    name: "military_service",
    title: "Service",
    terms: &[
        "army",
        "navy",
        "air_force",
        "marines",
        "gendarmerie",
        "border_guard",
        "national_guard",
        "other",
    ],
};

/// Rank category.
pub const RANK_CATEGORY: Vocabulary = Vocabulary {
    name: "rank_category",
    title: "Rank category",
    terms: &[
        "enlisted",
        "non_commissioned",
        "warrant",
        "officer_cadet",
        "junior_officer",
        "senior_officer",
        "general_officer",
    ],
};

/// Military rank — Armée de terre (France).
pub const MILITARY_RANK_FR: Vocabulary = Vocabulary {
    name: "military_rank_fr",
    title: "Military rank — Armée de terre (France)",
    terms: &[
        "soldat_de_2e_classe",
        "soldat_de_1re_classe",
        "caporal",
        "caporal_chef",
        "caporal_chef_de_1re_classe",
        "sergent",
        "sergent_chef",
        "adjudant",
        "adjudant_chef",
        "major",
        "aspirant",
        "sous_lieutenant",
        "lieutenant",
        "capitaine",
        "commandant",
        "lieutenant_colonel",
        "colonel",
        "general_de_brigade",
        "general_de_division",
        "general_de_corps_d_armee",
        "general_d_armee",
        "marechal_de_france",
    ],
};

/// Military rank — Wojska Lądowe (Polska).
pub const MILITARY_RANK_PL: Vocabulary = Vocabulary {
    name: "military_rank_pl",
    title: "Military rank — Wojska Lądowe (Polska)",
    terms: &[
        "szeregowy",
        "starszy_szeregowy",
        "kapral",
        "starszy_kapral",
        "plutonowy",
        "sierzant",
        "starszy_sierzant",
        "mlodszy_chorazy",
        "chorazy",
        "starszy_chorazy",
        "starszy_chorazy_sztabowy",
        "podporucznik",
        "porucznik",
        "kapitan",
        "major",
        "podpulkownik",
        "pulkownik",
        "general_brygady",
        "general_dywizji",
        "general_broni",
        "general",
        "marszalek_polski",
    ],
};

/// Military rank — Heer (Deutschland).
pub const MILITARY_RANK_DE: Vocabulary = Vocabulary {
    name: "military_rank_de",
    title: "Military rank — Heer (Deutschland)",
    terms: &[
        "soldat",
        "gefreiter",
        "obergefreiter",
        "hauptgefreiter",
        "stabsgefreiter",
        "oberstabsgefreiter",
        "unteroffizier",
        "stabsunteroffizier",
        "feldwebel",
        "oberfeldwebel",
        "hauptfeldwebel",
        "stabsfeldwebel",
        "oberstabsfeldwebel",
        "fahnenjunker",
        "faehnrich",
        "oberfaehnrich",
        "leutnant",
        "oberleutnant",
        "hauptmann",
        "stabshauptmann",
        "major",
        "oberstleutnant",
        "oberst",
        "brigadegeneral",
        "generalmajor",
        "generalleutnant",
        "general",
    ],
};

/// Military rank — British Army (United Kingdom).
pub const MILITARY_RANK_GB: Vocabulary = Vocabulary {
    name: "military_rank_gb",
    title: "Military rank — British Army (United Kingdom)",
    terms: &[
        "private",
        "lance_corporal",
        "corporal",
        "sergeant",
        "staff_sergeant",
        "warrant_officer_class_2",
        "warrant_officer_class_1",
        "officer_cadet",
        "second_lieutenant",
        "lieutenant",
        "captain",
        "major",
        "lieutenant_colonel",
        "colonel",
        "brigadier",
        "major_general",
        "lieutenant_general",
        "general",
        "field_marshal",
    ],
};

/// Military rank — United States Army.
pub const MILITARY_RANK_US: Vocabulary = Vocabulary {
    name: "military_rank_us",
    title: "Military rank — United States Army",
    terms: &[
        "private",
        "private_first_class",
        "specialist",
        "corporal",
        "sergeant",
        "staff_sergeant",
        "sergeant_first_class",
        "master_sergeant",
        "first_sergeant",
        "sergeant_major",
        "command_sergeant_major",
        "sergeant_major_of_the_army",
        "warrant_officer_1",
        "chief_warrant_officer_2",
        "chief_warrant_officer_3",
        "chief_warrant_officer_4",
        "chief_warrant_officer_5",
        "second_lieutenant",
        "first_lieutenant",
        "captain",
        "major",
        "lieutenant_colonel",
        "colonel",
        "brigadier_general",
        "major_general",
        "lieutenant_general",
        "general",
        "general_of_the_army",
    ],
};

/// Military rank — Сухопутные войска (Россия).
pub const MILITARY_RANK_RU: Vocabulary = Vocabulary {
    name: "military_rank_ru",
    title: "Military rank — Сухопутные войска (Россия)",
    terms: &[
        "ryadovoy",
        "efreytor",
        "mladshiy_serzhant",
        "serzhant",
        "starshiy_serzhant",
        "starshina",
        "praporshchik",
        "starshiy_praporshchik",
        "mladshiy_leytenant",
        "leytenant",
        "starshiy_leytenant",
        "kapitan",
        "mayor",
        "podpolkovnik",
        "polkovnik",
        "general_mayor",
        "general_leytenant",
        "general_polkovnik",
        "general_armii",
        "marshal_rossiyskoy_federatsii",
    ],
};

/// Offence section (ICCS). UNODC, International Classification of Crime for Statistical Purposes, version 1.0 (2015), level 1.
pub const ICCS_SECTION: Vocabulary = Vocabulary {
    name: "iccs_section",
    title: "Offence section (ICCS)",
    terms: &[
        "acts_leading_to_death",
        "acts_causing_harm",
        "sexual_acts",
        "property_with_violence",
        "property_only",
        "controlled_substances",
        "fraud_deception_corruption",
        "public_order_and_state",
        "public_safety_and_security",
        "natural_environment",
        "other_criminal_acts",
    ],
};

/// Case outcome.
pub const CASE_OUTCOME: Vocabulary = Vocabulary {
    name: "case_outcome",
    title: "Case outcome",
    terms: &[
        "convicted",
        "acquitted",
        "dismissed",
        "conviction_quashed",
        "pardoned",
        "amnestied",
        "expunged",
        "pending",
        "unknown",
    ],
};

/// Religious tradition.
pub const RELIGION: Vocabulary = Vocabulary {
    name: "religion",
    title: "Religious tradition",
    terms: &[
        "buddhism",
        "christianity_catholic",
        "christianity_orthodox",
        "christianity_protestant",
        "christianity_other",
        "hinduism",
        "islam_sunni",
        "islam_shia",
        "islam_other",
        "jainism",
        "judaism",
        "sikhism",
        "bahai",
        "shinto",
        "taoism",
        "zoroastrianism",
        "traditional",
        "other",
        "none",
        "unknown",
    ],
};

/// Sacrament or rite.
pub const SACRAMENT: Vocabulary = Vocabulary {
    name: "sacrament",
    title: "Sacrament or rite",
    terms: &[
        "baptism",
        "confirmation",
        "first_communion",
        "reconciliation",
        "anointing_of_the_sick",
        "holy_orders",
        "matrimony",
        "other_rite",
    ],
};

/// Political position.
pub const POLITICAL_POSITION: Vocabulary = Vocabulary {
    name: "political_position",
    title: "Political position",
    terms: &[
        "far_left",
        "left",
        "centre_left",
        "centre",
        "centre_right",
        "right",
        "far_right",
        "apolitical",
        "other",
        "unknown",
    ],
};

/// Membership.
pub const MEMBERSHIP_KIND: Vocabulary = Vocabulary {
    name: "membership_kind",
    title: "Membership",
    terms: &[
        "trade_union",
        "political_party",
        "professional_body",
        "religious_order",
        "religious_association",
        "fraternal_order",
        "veterans_association",
        "sports_club",
        "cultural_association",
        "charitable_association",
        "other",
    ],
};

/// MBTI type. Myers–Briggs Type Indicator.
pub const MBTI: Vocabulary = Vocabulary {
    name: "mbti",
    title: "MBTI type",
    terms: &[
        "ISTJ", "ISFJ", "INFJ", "INTJ", "ISTP", "ISFP", "INFP", "INTP", "ESTP", "ESFP", "ENFP",
        "ENTP", "ESTJ", "ESFJ", "ENFJ", "ENTJ",
    ],
};

/// Personality instrument.
pub const PERSONALITY_INSTRUMENT: Vocabulary = Vocabulary {
    name: "personality_instrument",
    title: "Personality instrument",
    terms: &[
        "neo_pi_3",
        "neo_ffi_3",
        "bfi_2",
        "ipip_neo_120",
        "tipi",
        "hexaco_pi_r",
        "observer_rating",
        "inferred",
        "other",
    ],
};

/// Introversion and extraversion.
pub const INTROVERSION_EXTRAVERSION: Vocabulary = Vocabulary {
    name: "introversion_extraversion",
    title: "Introversion and extraversion",
    terms: &[
        "strongly_introverted",
        "introverted",
        "ambiverted",
        "extraverted",
        "strongly_extraverted",
    ],
};

/// Stress tolerance.
pub const STRESS_TOLERANCE: Vocabulary = Vocabulary {
    name: "stress_tolerance",
    title: "Stress tolerance",
    terms: &["very_low", "low", "moderate", "high", "very_high"],
};

/// Decision-making style. Scott & Bruce (1995), General Decision-Making Style inventory.
pub const DECISION_STYLE: Vocabulary = Vocabulary {
    name: "decision_style",
    title: "Decision-making style",
    terms: &[
        "rational",
        "intuitive",
        "dependent",
        "avoidant",
        "spontaneous",
    ],
};

/// Sport level.
pub const SPORT_LEVEL: Vocabulary = Vocabulary {
    name: "sport_level",
    title: "Sport level",
    terms: &[
        "recreational",
        "amateur_competitive",
        "semi_professional",
        "professional",
    ],
};

/// Dietary pattern.
pub const DIET: Vocabulary = Vocabulary {
    name: "diet",
    title: "Dietary pattern",
    terms: &[
        "omnivore",
        "flexitarian",
        "pescatarian",
        "vegetarian",
        "vegan",
        "other",
    ],
};

/// Substance or behaviour.
pub const SUBSTANCE: Vocabulary = Vocabulary {
    name: "substance",
    title: "Substance or behaviour",
    terms: &[
        "tobacco",
        "alcohol",
        "cannabis",
        "opioids",
        "stimulants",
        "sedatives_hypnotics",
        "hallucinogens",
        "inhalants",
        "gambling",
        "gaming",
        "other",
    ],
};

/// Pattern of use.
pub const USE_PATTERN: Vocabulary = Vocabulary {
    name: "use_pattern",
    title: "Pattern of use",
    terms: &[
        "occasional_use",
        "regular_use",
        "harmful_use",
        "dependence",
        "in_remission",
    ],
};

/// Lineage. GEDCOM PEDI, extended.
pub const LINEAGE: Vocabulary = Vocabulary {
    name: "lineage",
    title: "Lineage",
    terms: &[
        "biological",
        "adoptive",
        "foster",
        "step",
        "guardianship",
        "unknown",
    ],
};

/// Relation.
pub const LINK_RELATION: Vocabulary = Vocabulary {
    name: "link_relation",
    title: "Relation",
    terms: &[
        "godparent",
        "godchild",
        "witness",
        "officiant",
        "business_partner",
        "employer",
        "employee",
        "mentor",
        "apprentice",
        "close_friend",
        "neighbour",
        "guardian",
        "ward",
        "other",
    ],
};

/// Every vocabulary, in the order SPEC_1.1 §6 indexes them.
pub const ALL: &[&Vocabulary] = &[
    &SENSITIVE_CLASS,
    &LATERALITY,
    &BODY_REGION,
    &ARTEFACT_TYPE,
    &CONSENT,
    &SEX_AT_BIRTH,
    &GENDER_IDENTITY,
    &TITLE_KIND,
    &REGISTER_TYPE,
    &BUILD,
    &EYE_COLOUR,
    &EYE_SHAPE,
    &EYE_SPACING,
    &HAIR_COLOUR,
    &HAIR_TEXTURE,
    &HAIRLINE,
    &FACIAL_HAIR,
    &BODY_HAIR,
    &SKIN_TONE,
    &SKIN_UNDERTONE,
    &FRECKLES,
    &PIGMENTATION_MARK,
    &MOLE_SHAPE,
    &FACE_SHAPE,
    &NOSE_SHAPE,
    &EAR_SHAPE,
    &LIP_SHAPE,
    &DENTITION,
    &MALOCCLUSION,
    &POSTURE,
    &GAIT,
    &VOCAL_TIMBRE,
    &SPEECH_REGISTER,
    &HANDEDNESS,
    &HEARING_GRADE,
    &OPTICAL_CORRECTION,
    &BLOOD_GROUP,
    &RHESUS,
    &ICD10_CHAPTER,
    &DIAGNOSIS_STATUS,
    &PROSTHESIS_KIND,
    &IMPLANT_KIND,
    &DEVICE_KIND,
    &ALLERGY_TYPE,
    &ALLERGY_SEVERITY,
    &PATHOGEN,
    &VACCINATION_STATUS,
    &SEROLOGY_RESULT,
    &LAB_PANEL,
    &LAB_ANALYTE,
    &LAB_UNIT,
    &LAB_FLAG,
    &NUTRIENT,
    &SLEEP_DISORDER,
    &ASSESSMENT_INSTRUMENT,
    &ASSESSMENT_SEVERITY,
    &REFERENCE_BUILD,
    &GENOMIC_FILE_FORMAT,
    &Y_HAPLOGROUP,
    &MT_HAPLOGROUP,
    &ZYGOSITY,
    &CLINICAL_SIGNIFICANCE,
    &INHERITANCE_PATTERN,
    &CARRIER_STATUS,
    &EPIGENETIC_CLOCK,
    &METABOLISER_STATUS,
    &AUTOPSY,
    &DISPOSITION,
    &ADDRESS_USE,
    &COUNTRY,
    &NATIONALITY_MODE,
    &LANGUAGE_PROFICIENCY,
    &ISCED_LEVEL,
    &INCOME_QUINTILE,
    &PAY_PERIOD,
    &TENURE,
    &DISTINCTION_KIND,
    &MILITARY_SERVICE,
    &RANK_CATEGORY,
    &MILITARY_RANK_FR,
    &MILITARY_RANK_PL,
    &MILITARY_RANK_DE,
    &MILITARY_RANK_GB,
    &MILITARY_RANK_US,
    &MILITARY_RANK_RU,
    &ICCS_SECTION,
    &CASE_OUTCOME,
    &RELIGION,
    &SACRAMENT,
    &POLITICAL_POSITION,
    &MEMBERSHIP_KIND,
    &MBTI,
    &PERSONALITY_INSTRUMENT,
    &INTROVERSION_EXTRAVERSION,
    &STRESS_TOLERANCE,
    &DECISION_STYLE,
    &SPORT_LEVEL,
    &DIET,
    &SUBSTANCE,
    &USE_PATTERN,
    &LINEAGE,
    &LINK_RELATION,
];

/// Look a vocabulary up by name.
pub fn by_name(name: &str) -> Option<&'static Vocabulary> {
    ALL.iter().copied().find(|v| v.name == name)
}

/// The analytes each laboratory panel may contain (SPEC_1.1 §5.4). An
/// analyte outside its panel's list is out of vocabulary even though it is
/// a term of [`LAB_ANALYTE`].
pub const LAB_PANEL_ANALYTES: &[(&str, &[&str])] = &[
    (
        "basic_metabolic",
        &[
            "sodium",
            "potassium",
            "chloride",
            "bicarbonate",
            "urea",
            "creatinine",
            "glucose",
            "calcium",
        ],
    ),
    (
        "lipid",
        &[
            "total_cholesterol",
            "ldl_cholesterol",
            "hdl_cholesterol",
            "triglycerides",
            "non_hdl_cholesterol",
        ],
    ),
    (
        "liver",
        &[
            "alt",
            "ast",
            "alp",
            "ggt",
            "total_bilirubin",
            "direct_bilirubin",
            "albumin",
            "total_protein",
        ],
    ),
    (
        "renal",
        &[
            "creatinine",
            "urea",
            "egfr",
            "uric_acid",
            "phosphate",
            "urine_albumin_creatinine_ratio",
        ],
    ),
    ("glycated_haemoglobin", &["hba1c"]),
    (
        "iron",
        &[
            "serum_iron",
            "ferritin",
            "transferrin",
            "transferrin_saturation",
            "tibc",
        ],
    ),
];

/// One rank in a national vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank {
    /// The term stored in `military.ranks[].value.rank`.
    pub term: &'static str,
    /// The rank's title in its own language. Data, not a label: a Polish
    /// *kapral* is shown as *kapral* whatever language the reader uses.
    pub title: &'static str,
    /// Its [`RANK_CATEGORY`] term, which compares across armies.
    pub category: &'static str,
}

/// The ranks of one country's army, selected by `country`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankVocabulary {
    /// The ISO 3166-1 code that selects this vocabulary.
    pub country: &'static str,
    /// The army the ranks belong to, named in its own language.
    pub army: &'static str,
    /// The same terms as a [`Vocabulary`].
    pub vocabulary: &'static Vocabulary,
    /// Every rank, junior first.
    pub ranks: &'static [Rank],
}

/// Every registered national rank vocabulary (SPEC_1.1 §5.9).
pub const MILITARY_RANKS: &[RankVocabulary] = &[
    RankVocabulary {
        country: "FR",
        army: "Armée de terre (France)",
        vocabulary: &MILITARY_RANK_FR,
        ranks: &[
            Rank {
                term: "soldat_de_2e_classe",
                title: "Soldat de 2e classe",
                category: "enlisted",
            },
            Rank {
                term: "soldat_de_1re_classe",
                title: "Soldat de 1re classe",
                category: "enlisted",
            },
            Rank {
                term: "caporal",
                title: "Caporal",
                category: "enlisted",
            },
            Rank {
                term: "caporal_chef",
                title: "Caporal-chef",
                category: "enlisted",
            },
            Rank {
                term: "caporal_chef_de_1re_classe",
                title: "Caporal-chef de 1re classe",
                category: "enlisted",
            },
            Rank {
                term: "sergent",
                title: "Sergent",
                category: "non_commissioned",
            },
            Rank {
                term: "sergent_chef",
                title: "Sergent-chef",
                category: "non_commissioned",
            },
            Rank {
                term: "adjudant",
                title: "Adjudant",
                category: "non_commissioned",
            },
            Rank {
                term: "adjudant_chef",
                title: "Adjudant-chef",
                category: "non_commissioned",
            },
            Rank {
                term: "major",
                title: "Major",
                category: "non_commissioned",
            },
            Rank {
                term: "aspirant",
                title: "Aspirant",
                category: "officer_cadet",
            },
            Rank {
                term: "sous_lieutenant",
                title: "Sous-lieutenant",
                category: "junior_officer",
            },
            Rank {
                term: "lieutenant",
                title: "Lieutenant",
                category: "junior_officer",
            },
            Rank {
                term: "capitaine",
                title: "Capitaine",
                category: "junior_officer",
            },
            Rank {
                term: "commandant",
                title: "Commandant",
                category: "senior_officer",
            },
            Rank {
                term: "lieutenant_colonel",
                title: "Lieutenant-colonel",
                category: "senior_officer",
            },
            Rank {
                term: "colonel",
                title: "Colonel",
                category: "senior_officer",
            },
            Rank {
                term: "general_de_brigade",
                title: "Général de brigade",
                category: "general_officer",
            },
            Rank {
                term: "general_de_division",
                title: "Général de division",
                category: "general_officer",
            },
            Rank {
                term: "general_de_corps_d_armee",
                title: "Général de corps d'armée",
                category: "general_officer",
            },
            Rank {
                term: "general_d_armee",
                title: "Général d'armée",
                category: "general_officer",
            },
            Rank {
                term: "marechal_de_france",
                title: "Maréchal de France",
                category: "general_officer",
            },
        ],
    },
    RankVocabulary {
        country: "PL",
        army: "Wojska Lądowe (Polska)",
        vocabulary: &MILITARY_RANK_PL,
        ranks: &[
            Rank {
                term: "szeregowy",
                title: "szeregowy",
                category: "enlisted",
            },
            Rank {
                term: "starszy_szeregowy",
                title: "starszy szeregowy",
                category: "enlisted",
            },
            Rank {
                term: "kapral",
                title: "kapral",
                category: "non_commissioned",
            },
            Rank {
                term: "starszy_kapral",
                title: "starszy kapral",
                category: "non_commissioned",
            },
            Rank {
                term: "plutonowy",
                title: "plutonowy",
                category: "non_commissioned",
            },
            Rank {
                term: "sierzant",
                title: "sierżant",
                category: "non_commissioned",
            },
            Rank {
                term: "starszy_sierzant",
                title: "starszy sierżant",
                category: "non_commissioned",
            },
            Rank {
                term: "mlodszy_chorazy",
                title: "młodszy chorąży",
                category: "warrant",
            },
            Rank {
                term: "chorazy",
                title: "chorąży",
                category: "warrant",
            },
            Rank {
                term: "starszy_chorazy",
                title: "starszy chorąży",
                category: "warrant",
            },
            Rank {
                term: "starszy_chorazy_sztabowy",
                title: "starszy chorąży sztabowy",
                category: "warrant",
            },
            Rank {
                term: "podporucznik",
                title: "podporucznik",
                category: "junior_officer",
            },
            Rank {
                term: "porucznik",
                title: "porucznik",
                category: "junior_officer",
            },
            Rank {
                term: "kapitan",
                title: "kapitan",
                category: "junior_officer",
            },
            Rank {
                term: "major",
                title: "major",
                category: "senior_officer",
            },
            Rank {
                term: "podpulkownik",
                title: "podpułkownik",
                category: "senior_officer",
            },
            Rank {
                term: "pulkownik",
                title: "pułkownik",
                category: "senior_officer",
            },
            Rank {
                term: "general_brygady",
                title: "generał brygady",
                category: "general_officer",
            },
            Rank {
                term: "general_dywizji",
                title: "generał dywizji",
                category: "general_officer",
            },
            Rank {
                term: "general_broni",
                title: "generał broni",
                category: "general_officer",
            },
            Rank {
                term: "general",
                title: "generał",
                category: "general_officer",
            },
            Rank {
                term: "marszalek_polski",
                title: "marszałek Polski",
                category: "general_officer",
            },
        ],
    },
    RankVocabulary {
        country: "DE",
        army: "Heer (Deutschland)",
        vocabulary: &MILITARY_RANK_DE,
        ranks: &[
            Rank {
                term: "soldat",
                title: "Soldat",
                category: "enlisted",
            },
            Rank {
                term: "gefreiter",
                title: "Gefreiter",
                category: "enlisted",
            },
            Rank {
                term: "obergefreiter",
                title: "Obergefreiter",
                category: "enlisted",
            },
            Rank {
                term: "hauptgefreiter",
                title: "Hauptgefreiter",
                category: "enlisted",
            },
            Rank {
                term: "stabsgefreiter",
                title: "Stabsgefreiter",
                category: "enlisted",
            },
            Rank {
                term: "oberstabsgefreiter",
                title: "Oberstabsgefreiter",
                category: "enlisted",
            },
            Rank {
                term: "unteroffizier",
                title: "Unteroffizier",
                category: "non_commissioned",
            },
            Rank {
                term: "stabsunteroffizier",
                title: "Stabsunteroffizier",
                category: "non_commissioned",
            },
            Rank {
                term: "feldwebel",
                title: "Feldwebel",
                category: "non_commissioned",
            },
            Rank {
                term: "oberfeldwebel",
                title: "Oberfeldwebel",
                category: "non_commissioned",
            },
            Rank {
                term: "hauptfeldwebel",
                title: "Hauptfeldwebel",
                category: "non_commissioned",
            },
            Rank {
                term: "stabsfeldwebel",
                title: "Stabsfeldwebel",
                category: "non_commissioned",
            },
            Rank {
                term: "oberstabsfeldwebel",
                title: "Oberstabsfeldwebel",
                category: "non_commissioned",
            },
            Rank {
                term: "fahnenjunker",
                title: "Fahnenjunker",
                category: "officer_cadet",
            },
            Rank {
                term: "faehnrich",
                title: "Fähnrich",
                category: "officer_cadet",
            },
            Rank {
                term: "oberfaehnrich",
                title: "Oberfähnrich",
                category: "officer_cadet",
            },
            Rank {
                term: "leutnant",
                title: "Leutnant",
                category: "junior_officer",
            },
            Rank {
                term: "oberleutnant",
                title: "Oberleutnant",
                category: "junior_officer",
            },
            Rank {
                term: "hauptmann",
                title: "Hauptmann",
                category: "junior_officer",
            },
            Rank {
                term: "stabshauptmann",
                title: "Stabshauptmann",
                category: "junior_officer",
            },
            Rank {
                term: "major",
                title: "Major",
                category: "senior_officer",
            },
            Rank {
                term: "oberstleutnant",
                title: "Oberstleutnant",
                category: "senior_officer",
            },
            Rank {
                term: "oberst",
                title: "Oberst",
                category: "senior_officer",
            },
            Rank {
                term: "brigadegeneral",
                title: "Brigadegeneral",
                category: "general_officer",
            },
            Rank {
                term: "generalmajor",
                title: "Generalmajor",
                category: "general_officer",
            },
            Rank {
                term: "generalleutnant",
                title: "Generalleutnant",
                category: "general_officer",
            },
            Rank {
                term: "general",
                title: "General",
                category: "general_officer",
            },
        ],
    },
    RankVocabulary {
        country: "GB",
        army: "British Army (United Kingdom)",
        vocabulary: &MILITARY_RANK_GB,
        ranks: &[
            Rank {
                term: "private",
                title: "Private",
                category: "enlisted",
            },
            Rank {
                term: "lance_corporal",
                title: "Lance Corporal",
                category: "non_commissioned",
            },
            Rank {
                term: "corporal",
                title: "Corporal",
                category: "non_commissioned",
            },
            Rank {
                term: "sergeant",
                title: "Sergeant",
                category: "non_commissioned",
            },
            Rank {
                term: "staff_sergeant",
                title: "Staff Sergeant",
                category: "non_commissioned",
            },
            Rank {
                term: "warrant_officer_class_2",
                title: "Warrant Officer Class 2",
                category: "warrant",
            },
            Rank {
                term: "warrant_officer_class_1",
                title: "Warrant Officer Class 1",
                category: "warrant",
            },
            Rank {
                term: "officer_cadet",
                title: "Officer Cadet",
                category: "officer_cadet",
            },
            Rank {
                term: "second_lieutenant",
                title: "Second Lieutenant",
                category: "junior_officer",
            },
            Rank {
                term: "lieutenant",
                title: "Lieutenant",
                category: "junior_officer",
            },
            Rank {
                term: "captain",
                title: "Captain",
                category: "junior_officer",
            },
            Rank {
                term: "major",
                title: "Major",
                category: "senior_officer",
            },
            Rank {
                term: "lieutenant_colonel",
                title: "Lieutenant Colonel",
                category: "senior_officer",
            },
            Rank {
                term: "colonel",
                title: "Colonel",
                category: "senior_officer",
            },
            Rank {
                term: "brigadier",
                title: "Brigadier",
                category: "general_officer",
            },
            Rank {
                term: "major_general",
                title: "Major General",
                category: "general_officer",
            },
            Rank {
                term: "lieutenant_general",
                title: "Lieutenant General",
                category: "general_officer",
            },
            Rank {
                term: "general",
                title: "General",
                category: "general_officer",
            },
            Rank {
                term: "field_marshal",
                title: "Field Marshal",
                category: "general_officer",
            },
        ],
    },
    RankVocabulary {
        country: "US",
        army: "United States Army",
        vocabulary: &MILITARY_RANK_US,
        ranks: &[
            Rank {
                term: "private",
                title: "Private",
                category: "enlisted",
            },
            Rank {
                term: "private_first_class",
                title: "Private First Class",
                category: "enlisted",
            },
            Rank {
                term: "specialist",
                title: "Specialist",
                category: "enlisted",
            },
            Rank {
                term: "corporal",
                title: "Corporal",
                category: "non_commissioned",
            },
            Rank {
                term: "sergeant",
                title: "Sergeant",
                category: "non_commissioned",
            },
            Rank {
                term: "staff_sergeant",
                title: "Staff Sergeant",
                category: "non_commissioned",
            },
            Rank {
                term: "sergeant_first_class",
                title: "Sergeant First Class",
                category: "non_commissioned",
            },
            Rank {
                term: "master_sergeant",
                title: "Master Sergeant",
                category: "non_commissioned",
            },
            Rank {
                term: "first_sergeant",
                title: "First Sergeant",
                category: "non_commissioned",
            },
            Rank {
                term: "sergeant_major",
                title: "Sergeant Major",
                category: "non_commissioned",
            },
            Rank {
                term: "command_sergeant_major",
                title: "Command Sergeant Major",
                category: "non_commissioned",
            },
            Rank {
                term: "sergeant_major_of_the_army",
                title: "Sergeant Major of the Army",
                category: "non_commissioned",
            },
            Rank {
                term: "warrant_officer_1",
                title: "Warrant Officer 1",
                category: "warrant",
            },
            Rank {
                term: "chief_warrant_officer_2",
                title: "Chief Warrant Officer 2",
                category: "warrant",
            },
            Rank {
                term: "chief_warrant_officer_3",
                title: "Chief Warrant Officer 3",
                category: "warrant",
            },
            Rank {
                term: "chief_warrant_officer_4",
                title: "Chief Warrant Officer 4",
                category: "warrant",
            },
            Rank {
                term: "chief_warrant_officer_5",
                title: "Chief Warrant Officer 5",
                category: "warrant",
            },
            Rank {
                term: "second_lieutenant",
                title: "Second Lieutenant",
                category: "junior_officer",
            },
            Rank {
                term: "first_lieutenant",
                title: "First Lieutenant",
                category: "junior_officer",
            },
            Rank {
                term: "captain",
                title: "Captain",
                category: "junior_officer",
            },
            Rank {
                term: "major",
                title: "Major",
                category: "senior_officer",
            },
            Rank {
                term: "lieutenant_colonel",
                title: "Lieutenant Colonel",
                category: "senior_officer",
            },
            Rank {
                term: "colonel",
                title: "Colonel",
                category: "senior_officer",
            },
            Rank {
                term: "brigadier_general",
                title: "Brigadier General",
                category: "general_officer",
            },
            Rank {
                term: "major_general",
                title: "Major General",
                category: "general_officer",
            },
            Rank {
                term: "lieutenant_general",
                title: "Lieutenant General",
                category: "general_officer",
            },
            Rank {
                term: "general",
                title: "General",
                category: "general_officer",
            },
            Rank {
                term: "general_of_the_army",
                title: "General of the Army",
                category: "general_officer",
            },
        ],
    },
    RankVocabulary {
        country: "RU",
        army: "Сухопутные войска (Россия)",
        vocabulary: &MILITARY_RANK_RU,
        ranks: &[
            Rank {
                term: "ryadovoy",
                title: "рядовой",
                category: "enlisted",
            },
            Rank {
                term: "efreytor",
                title: "ефрейтор",
                category: "enlisted",
            },
            Rank {
                term: "mladshiy_serzhant",
                title: "младший сержант",
                category: "non_commissioned",
            },
            Rank {
                term: "serzhant",
                title: "сержант",
                category: "non_commissioned",
            },
            Rank {
                term: "starshiy_serzhant",
                title: "старший сержант",
                category: "non_commissioned",
            },
            Rank {
                term: "starshina",
                title: "старшина",
                category: "non_commissioned",
            },
            Rank {
                term: "praporshchik",
                title: "прапорщик",
                category: "warrant",
            },
            Rank {
                term: "starshiy_praporshchik",
                title: "старший прапорщик",
                category: "warrant",
            },
            Rank {
                term: "mladshiy_leytenant",
                title: "младший лейтенант",
                category: "junior_officer",
            },
            Rank {
                term: "leytenant",
                title: "лейтенант",
                category: "junior_officer",
            },
            Rank {
                term: "starshiy_leytenant",
                title: "старший лейтенант",
                category: "junior_officer",
            },
            Rank {
                term: "kapitan",
                title: "капитан",
                category: "junior_officer",
            },
            Rank {
                term: "mayor",
                title: "майор",
                category: "senior_officer",
            },
            Rank {
                term: "podpolkovnik",
                title: "подполковник",
                category: "senior_officer",
            },
            Rank {
                term: "polkovnik",
                title: "полковник",
                category: "senior_officer",
            },
            Rank {
                term: "general_mayor",
                title: "генерал-майор",
                category: "general_officer",
            },
            Rank {
                term: "general_leytenant",
                title: "генерал-лейтенант",
                category: "general_officer",
            },
            Rank {
                term: "general_polkovnik",
                title: "генерал-полковник",
                category: "general_officer",
            },
            Rank {
                term: "general_armii",
                title: "генерал армии",
                category: "general_officer",
            },
            Rank {
                term: "marshal_rossiyskoy_federatsii",
                title: "маршал Российской Федерации",
                category: "general_officer",
            },
        ],
    },
];

/// The rank vocabulary `country` selects, if 1.1 registers one.
pub fn ranks_for(country: &str) -> Option<&'static RankVocabulary> {
    MILITARY_RANKS.iter().find(|r| r.country == country)
}
