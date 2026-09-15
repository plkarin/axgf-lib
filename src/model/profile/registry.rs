// SPDX-License-Identifier: Apache-2.0
//! The AXGF 1.1 attribute registry (SPEC_1.1 §5), as data.
//!
//! Every attribute the profile defines, in the order and groups the
//! specification presents them: its path on the Person, whether it holds a
//! single claim or a series, the shape of the claim's `value`, and its
//! sensitive class. Nothing here does anything. It is the table an
//! implementation reads to build a form, to check a vocabulary, or to find
//! every attribute of one class without writing the list out a second time.

use super::vocab::{self, Vocabulary};

/// One of the four sensitive classes (SPEC_1.1 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SensitiveClass {
    /// Health, and the other special categories that are neither biometric
    /// nor genetic: belief, political opinion, membership.
    Health,
    /// Templates and measures that identify a body or a voice.
    Biometrics,
    /// Genetic and other molecular data.
    Genomics,
    /// Criminal proceedings.
    Legal,
}

impl SensitiveClass {
    /// The four classes, in the specification's order.
    pub const ALL: [SensitiveClass; 4] = [
        SensitiveClass::Health,
        SensitiveClass::Biometrics,
        SensitiveClass::Genomics,
        SensitiveClass::Legal,
    ];

    /// The class as the bundle spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            SensitiveClass::Health => "health",
            SensitiveClass::Biometrics => "biometrics",
            SensitiveClass::Genomics => "genomics",
            SensitiveClass::Legal => "legal",
        }
    }

    /// Read a class back from its spelling.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

/// Whether an attribute holds one claim or a series of them (SPEC_1.1 §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    /// One claim object: fixed at birth or by one event.
    Single,
    /// An array of claims: anything that can change in a lifetime.
    Series,
}

/// The shape of a claim's `value`, or of one field of an object value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A non-empty string: free text.
    Text,
    /// A number in a fixed unit, within bounds where the specification sets any.
    Number {
        /// Inclusive lower bound.
        min: Option<f64>,
        /// Inclusive upper bound.
        max: Option<f64>,
        /// The unit, empty for a ratio or a score.
        unit: &'static str,
    },
    /// An integer in a fixed unit.
    Integer {
        /// Inclusive lower bound.
        min: Option<i64>,
        /// Inclusive upper bound.
        max: Option<i64>,
        /// The unit, empty when there is none.
        unit: &'static str,
    },
    /// `true` or `false`.
    Boolean,
    /// A lowercase UUID v4 referring to another entity.
    Uuid,
    /// `HH:MM` or `HH:MM:SS`, local time as recorded.
    TimeOfDay,
    /// `{ lat, lon, precision }`.
    Coordinates,
    /// A BCP 47 language tag, or `und`.
    LanguageTag,
    /// An ISO 4217 alphabetic currency code.
    CurrencyCode,
    /// A term from a closed vocabulary.
    Vocab(&'static Vocabulary),
    /// A string matching this regular expression.
    Pattern(&'static str),
    /// A rank term from the vocabulary its object's `country` selects.
    Rank,
    /// A reference to a Document, restricted to these artefact types.
    Artefact(&'static [&'static str]),
    /// An object with these fields, of which at least one group in `one_of`
    /// must be present when `one_of` is not empty.
    Object {
        /// The fields, in the specification's order.
        fields: &'static [Field],
        /// Each inner list names fields of which at least one is required.
        one_of: &'static [&'static [&'static str]],
    },
}

/// One field of an object-valued claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    /// The field's key inside `value`.
    pub key: &'static str,
    /// Whether the schema requires it.
    pub required: bool,
    /// Its shape.
    pub shape: Shape,
}

/// One attribute of the profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attribute {
    /// What is recorded, named as the specification's table names it.
    pub name: &'static str,
    /// The key of the group it belongs to ([`Group::key`]).
    pub group: &'static str,
    /// The Person block that holds it: `morphology`, `health`, or the 1.0
    /// blocks `identity`, `birth` and `death` that 1.1 extends.
    pub block: &'static str,
    /// Its key inside that block.
    pub key: &'static str,
    /// `block.key`, as SPEC_1.1 writes it.
    pub path: &'static str,
    /// One claim or a series.
    pub cardinality: Cardinality,
    /// The shape of each claim's `value`.
    pub shape: Shape,
    /// Its sensitive class, if it has one.
    pub class: Option<SensitiveClass>,
}

/// Something a group covers that a 1.0 field or another entity already
/// records, so 1.1 adds no attribute for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapped {
    /// What is recorded.
    pub name: &'static str,
    /// Where it is recorded.
    pub location: &'static str,
}

/// One of the fourteen groups (SPEC_1.1 §5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Group {
    /// A stable key: `identity`, `morphology`, … `digital_legacy`.
    pub key: &'static str,
    /// The group's title in the specification, in English.
    pub title: &'static str,
    /// Its attributes, in presentation order.
    pub attributes: &'static [Attribute],
    /// What it covers through existing fields and entities.
    pub mapped: &'static [Mapped],
}

/// The attributes of *Identity and civil status*.
pub const IDENTITY_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "titles",
        group: "identity",
        block: "identity",
        key: "titles",
        path: "identity.titles",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "text",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "kind",
                    required: false,
                    shape: Shape::Vocab(&vocab::TITLE_KIND),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "sex at birth",
        group: "identity",
        block: "identity",
        key: "sex_at_birth",
        path: "identity.sex_at_birth",
        cardinality: Cardinality::Single,
        shape: Shape::Vocab(&vocab::SEX_AT_BIRTH),
        class: None,
    },
    Attribute {
        name: "gender identity",
        group: "identity",
        block: "identity",
        key: "gender_identity",
        path: "identity.gender_identity",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::GENDER_IDENTITY),
        class: None,
    },
    Attribute {
        name: "birth time",
        group: "identity",
        block: "birth",
        key: "time",
        path: "birth.time",
        cardinality: Cardinality::Single,
        shape: Shape::TimeOfDay,
        class: None,
    },
    Attribute {
        name: "birth coordinates",
        group: "identity",
        block: "birth",
        key: "coordinates",
        path: "birth.coordinates",
        cardinality: Cardinality::Single,
        shape: Shape::Coordinates,
        class: None,
    },
    Attribute {
        name: "birth certificate number",
        group: "identity",
        block: "civil_status",
        key: "birth_certificate_number",
        path: "civil_status.birth_certificate_number",
        cardinality: Cardinality::Single,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "civil register entries",
        group: "identity",
        block: "civil_status",
        key: "register_entries",
        path: "civil_status.register_entries",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "register_type",
                    required: true,
                    shape: Shape::Vocab(&vocab::REGISTER_TYPE),
                },
                Field {
                    key: "office",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
                Field {
                    key: "volume",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "page",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "entry_number",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "marginal annotations",
        group: "identity",
        block: "civil_status",
        key: "marginal_annotations",
        path: "civil_status.marginal_annotations",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "text",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "register_type",
                    required: false,
                    shape: Shape::Vocab(&vocab::REGISTER_TYPE),
                },
                Field {
                    key: "entry_number",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
];

/// The attributes of *Morphology*.
pub const MORPHOLOGY_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "exact height",
        group: "morphology",
        block: "morphology",
        key: "height",
        path: "morphology.height",
        cardinality: Cardinality::Series,
        shape: Shape::Number {
            min: Some(20.0),
            max: Some(300.0),
            unit: "cm",
        },
        class: None,
    },
    Attribute {
        name: "weight over time",
        group: "morphology",
        block: "morphology",
        key: "weight",
        path: "morphology.weight",
        cardinality: Cardinality::Series,
        shape: Shape::Number {
            min: Some(0.3),
            max: Some(700.0),
            unit: "kg",
        },
        class: None,
    },
    Attribute {
        name: "BMI",
        group: "morphology",
        block: "morphology",
        key: "bmi",
        path: "morphology.bmi",
        cardinality: Cardinality::Series,
        shape: Shape::Number {
            min: Some(5.0),
            max: Some(150.0),
            unit: "kg/m²",
        },
        class: None,
    },
    Attribute {
        name: "body composition",
        group: "morphology",
        block: "morphology",
        key: "body_composition",
        path: "morphology.body_composition",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "bone_percent",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(100.0),
                        unit: "%",
                    },
                },
                Field {
                    key: "muscle_percent",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(100.0),
                        unit: "%",
                    },
                },
                Field {
                    key: "fat_percent",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(100.0),
                        unit: "%",
                    },
                },
            ],
            one_of: &[&["bone_percent"], &["muscle_percent"], &["fat_percent"]],
        },
        class: None,
    },
    Attribute {
        name: "build",
        group: "morphology",
        block: "morphology",
        key: "build",
        path: "morphology.build",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::BUILD),
        class: None,
    },
    Attribute {
        name: "precise eye colour",
        group: "morphology",
        block: "morphology",
        key: "eye_colour",
        path: "morphology.eye_colour",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::EYE_COLOUR),
        class: None,
    },
    Attribute {
        name: "eye shape",
        group: "morphology",
        block: "morphology",
        key: "eye_shape",
        path: "morphology.eye_shape",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::EYE_SHAPE),
        class: None,
    },
    Attribute {
        name: "eye spacing",
        group: "morphology",
        block: "morphology",
        key: "eye_spacing",
        path: "morphology.eye_spacing",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::EYE_SPACING),
        class: None,
    },
    Attribute {
        name: "natural hair colour",
        group: "morphology",
        block: "morphology",
        key: "hair_colour",
        path: "morphology.hair_colour",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::HAIR_COLOUR),
        class: None,
    },
    Attribute {
        name: "hair texture",
        group: "morphology",
        block: "morphology",
        key: "hair_texture",
        path: "morphology.hair_texture",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::HAIR_TEXTURE),
        class: None,
    },
    Attribute {
        name: "hairline",
        group: "morphology",
        block: "morphology",
        key: "hairline",
        path: "morphology.hairline",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::HAIRLINE),
        class: None,
    },
    Attribute {
        name: "facial hair",
        group: "morphology",
        block: "morphology",
        key: "facial_hair",
        path: "morphology.facial_hair",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::FACIAL_HAIR),
        class: None,
    },
    Attribute {
        name: "body hair",
        group: "morphology",
        block: "morphology",
        key: "body_hair",
        path: "morphology.body_hair",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::BODY_HAIR),
        class: None,
    },
    Attribute {
        name: "skin tone",
        group: "morphology",
        block: "morphology",
        key: "skin_tone",
        path: "morphology.skin_tone",
        cardinality: Cardinality::Single,
        shape: Shape::Vocab(&vocab::SKIN_TONE),
        class: None,
    },
    Attribute {
        name: "skin undertone",
        group: "morphology",
        block: "morphology",
        key: "skin_undertone",
        path: "morphology.skin_undertone",
        cardinality: Cardinality::Single,
        shape: Shape::Vocab(&vocab::SKIN_UNDERTONE),
        class: None,
    },
    Attribute {
        name: "freckles",
        group: "morphology",
        block: "morphology",
        key: "freckles",
        path: "morphology.freckles",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::FRECKLES),
        class: None,
    },
    Attribute {
        name: "pigmentation",
        group: "morphology",
        block: "morphology",
        key: "pigmentation",
        path: "morphology.pigmentation",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "kind",
                    required: true,
                    shape: Shape::Vocab(&vocab::PIGMENTATION_MARK),
                },
                Field {
                    key: "body_region",
                    required: false,
                    shape: Shape::Vocab(&vocab::BODY_REGION),
                },
                Field {
                    key: "description",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "scars",
        group: "morphology",
        block: "morphology",
        key: "scars",
        path: "morphology.scars",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "body_region",
                    required: false,
                    shape: Shape::Vocab(&vocab::BODY_REGION),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "tattoos",
        group: "morphology",
        block: "morphology",
        key: "tattoos",
        path: "morphology.tattoos",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "body_region",
                    required: false,
                    shape: Shape::Vocab(&vocab::BODY_REGION),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "moles with location and shape",
        group: "morphology",
        block: "morphology",
        key: "moles",
        path: "morphology.moles",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "body_region",
                    required: true,
                    shape: Shape::Vocab(&vocab::BODY_REGION),
                },
                Field {
                    key: "location",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "shape",
                    required: false,
                    shape: Shape::Vocab(&vocab::MOLE_SHAPE),
                },
                Field {
                    key: "diameter_mm",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(500.0),
                        unit: "mm",
                    },
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "facial asymmetries",
        group: "morphology",
        block: "morphology",
        key: "facial_asymmetries",
        path: "morphology.facial_asymmetries",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "facial bone structure",
        group: "morphology",
        block: "morphology",
        key: "face_shape",
        path: "morphology.face_shape",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::FACE_SHAPE),
        class: None,
    },
    Attribute {
        name: "nose shape",
        group: "morphology",
        block: "morphology",
        key: "nose_shape",
        path: "morphology.nose_shape",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::NOSE_SHAPE),
        class: None,
    },
    Attribute {
        name: "ear shape",
        group: "morphology",
        block: "morphology",
        key: "ear_shape",
        path: "morphology.ear_shape",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::EAR_SHAPE),
        class: None,
    },
    Attribute {
        name: "lip shape",
        group: "morphology",
        block: "morphology",
        key: "lip_shape",
        path: "morphology.lip_shape",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::LIP_SHAPE),
        class: None,
    },
    Attribute {
        name: "dentition",
        group: "morphology",
        block: "morphology",
        key: "dentition",
        path: "morphology.dentition",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::DENTITION),
        class: None,
    },
    Attribute {
        name: "dental malocclusion",
        group: "morphology",
        block: "morphology",
        key: "malocclusion",
        path: "morphology.malocclusion",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::MALOCCLUSION),
        class: None,
    },
    Attribute {
        name: "posture",
        group: "morphology",
        block: "morphology",
        key: "posture",
        path: "morphology.posture",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::POSTURE),
        class: None,
    },
    Attribute {
        name: "gait",
        group: "morphology",
        block: "morphology",
        key: "gait",
        path: "morphology.gait",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::GAIT),
        class: None,
    },
    Attribute {
        name: "distinguishing features",
        group: "morphology",
        block: "morphology",
        key: "distinguishing_features",
        path: "morphology.distinguishing_features",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
];

/// The attributes of *Biometrics*.
pub const BIOMETRICS_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "fingerprints",
        group: "biometrics",
        block: "biometrics",
        key: "fingerprints",
        path: "biometrics.fingerprints",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["fingerprint_card", "fingerprint_template"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "retinal print",
        group: "biometrics",
        block: "biometrics",
        key: "retinal_print",
        path: "biometrics.retinal_print",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["retinal_image"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "voice signature",
        group: "biometrics",
        block: "biometrics",
        key: "voice_signature",
        path: "biometrics.voice_signature",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["voiceprint"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "fundamental voice frequency",
        group: "biometrics",
        block: "biometrics",
        key: "voice_frequency",
        path: "biometrics.voice_frequency",
        cardinality: Cardinality::Series,
        shape: Shape::Number {
            min: Some(30.0),
            max: Some(1100.0),
            unit: "Hz",
        },
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "vocal timbre",
        group: "biometrics",
        block: "biometrics",
        key: "vocal_timbre",
        path: "biometrics.vocal_timbre",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::VOCAL_TIMBRE),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "spoken accent",
        group: "biometrics",
        block: "biometrics",
        key: "spoken_accent",
        path: "biometrics.spoken_accent",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "language",
                    required: true,
                    shape: Shape::LanguageTag,
                },
                Field {
                    key: "description",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "speech rate",
        group: "biometrics",
        block: "biometrics",
        key: "speech_rate",
        path: "biometrics.speech_rate",
        cardinality: Cardinality::Series,
        shape: Shape::Number {
            min: Some(10.0),
            max: Some(600.0),
            unit: "words/min",
        },
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "verbal tics",
        group: "biometrics",
        block: "biometrics",
        key: "verbal_tics",
        path: "biometrics.verbal_tics",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "frequent vocabulary",
        group: "biometrics",
        block: "biometrics",
        key: "frequent_vocabulary",
        path: "biometrics.frequent_vocabulary",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "register of speech",
        group: "biometrics",
        block: "biometrics",
        key: "speech_register",
        path: "biometrics.speech_register",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::SPEECH_REGISTER),
        class: None,
    },
    Attribute {
        name: "motor tics",
        group: "biometrics",
        block: "biometrics",
        key: "motor_tics",
        path: "biometrics.motor_tics",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "handedness",
        group: "biometrics",
        block: "biometrics",
        key: "handedness",
        path: "biometrics.handedness",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::HANDEDNESS),
        class: None,
    },
    Attribute {
        name: "hearing capacity",
        group: "biometrics",
        block: "biometrics",
        key: "hearing",
        path: "biometrics.hearing",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "ear",
                    required: false,
                    shape: Shape::Vocab(&vocab::LATERALITY),
                },
                Field {
                    key: "grade",
                    required: true,
                    shape: Shape::Vocab(&vocab::HEARING_GRADE),
                },
                Field {
                    key: "threshold_db",
                    required: false,
                    shape: Shape::Number {
                        min: Some(-10.0),
                        max: Some(130.0),
                        unit: "dB HL",
                    },
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "visual acuity",
        group: "biometrics",
        block: "biometrics",
        key: "visual_acuity",
        path: "biometrics.visual_acuity",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "eye",
                    required: true,
                    shape: Shape::Vocab(&vocab::LATERALITY),
                },
                Field {
                    key: "decimal",
                    required: true,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(2.5),
                        unit: "",
                    },
                },
                Field {
                    key: "corrected",
                    required: false,
                    shape: Shape::Boolean,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "optical correction",
        group: "biometrics",
        block: "biometrics",
        key: "optical_correction",
        path: "biometrics.optical_correction",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "kind",
                    required: true,
                    shape: Shape::Vocab(&vocab::OPTICAL_CORRECTION),
                },
                Field {
                    key: "prescription",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
];

/// The attributes of *Health*.
pub const HEALTH_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "blood group",
        group: "health",
        block: "health",
        key: "blood_group",
        path: "health.blood_group",
        cardinality: Cardinality::Single,
        shape: Shape::Vocab(&vocab::BLOOD_GROUP),
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "rhesus",
        group: "health",
        block: "health",
        key: "rhesus",
        path: "health.rhesus",
        cardinality: Cardinality::Single,
        shape: Shape::Vocab(&vocab::RHESUS),
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "mean blood pressure",
        group: "health",
        block: "health",
        key: "blood_pressure",
        path: "health.blood_pressure",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "systolic", required: true, shape: Shape::Integer { min: Some(40), max: Some(300), unit: "mmHg" } },
                Field { key: "diastolic", required: true, shape: Shape::Integer { min: Some(20), max: Some(200), unit: "mmHg" } },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "resting heart rate",
        group: "health",
        block: "health",
        key: "resting_heart_rate",
        path: "health.resting_heart_rate",
        cardinality: Cardinality::Series,
        shape: Shape::Integer { min: Some(20), max: Some(250), unit: "bpm" },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "respiratory capacity (FEV1/FVC)",
        group: "health",
        block: "health",
        key: "respiratory_capacity",
        path: "health.respiratory_capacity",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "fev1_litres", required: false, shape: Shape::Number { min: Some(0.0), max: Some(10.0), unit: "L" } },
                Field { key: "fvc_litres", required: false, shape: Shape::Number { min: Some(0.0), max: Some(12.0), unit: "L" } },
                Field { key: "fev1_fvc_ratio", required: false, shape: Shape::Number { min: Some(0.0), max: Some(1.0), unit: "" } },
            ],
            one_of: &[&["fev1_litres"], &["fvc_litres"], &["fev1_fvc_ratio"]],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "chronic pathologies; diagnosed chronic illnesses; cardiovascular, respiratory, metabolic and endocrine pathologies; autoimmune diseases; oncological pathologies; psychiatric and neurodevelopmental conditions",
        group: "health",
        block: "health",
        key: "conditions",
        path: "health.conditions",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "description", required: true, shape: Shape::Text },
                Field { key: "icd10_chapter", required: false, shape: Shape::Vocab(&vocab::ICD10_CHAPTER) },
                Field { key: "chronic", required: false, shape: Shape::Boolean },
                Field { key: "autoimmune", required: false, shape: Shape::Boolean },
                Field { key: "diagnosis", required: false, shape: Shape::Vocab(&vocab::DIAGNOSIS_STATUS) },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "surgical history",
        group: "health",
        block: "health",
        key: "surgeries",
        path: "health.surgeries",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "description", required: true, shape: Shape::Text },
                Field { key: "body_region", required: false, shape: Shape::Vocab(&vocab::BODY_REGION) },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "trauma and fracture sequelae",
        group: "health",
        block: "health",
        key: "injuries",
        path: "health.injuries",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "description", required: true, shape: Shape::Text },
                Field { key: "body_region", required: false, shape: Shape::Vocab(&vocab::BODY_REGION) },
                Field { key: "fracture", required: false, shape: Shape::Boolean },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "physical deformities",
        group: "health",
        block: "health",
        key: "deformities",
        path: "health.deformities",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "description", required: true, shape: Shape::Text },
                Field { key: "body_region", required: false, shape: Shape::Vocab(&vocab::BODY_REGION) },
                Field { key: "congenital", required: false, shape: Shape::Boolean },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "amputations",
        group: "health",
        block: "health",
        key: "amputations",
        path: "health.amputations",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "body_region", required: true, shape: Shape::Vocab(&vocab::BODY_REGION) },
                Field { key: "level", required: false, shape: Shape::Text },
                Field { key: "cause", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "prostheses",
        group: "health",
        block: "health",
        key: "prostheses",
        path: "health.prostheses",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "kind", required: true, shape: Shape::Vocab(&vocab::PROSTHESIS_KIND) },
                Field { key: "description", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "biomedical implants",
        group: "health",
        block: "health",
        key: "implants",
        path: "health.implants",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "kind", required: true, shape: Shape::Vocab(&vocab::IMPLANT_KIND) },
                Field { key: "description", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "pacemakers and intracorporeal devices",
        group: "health",
        block: "health",
        key: "devices",
        path: "health.devices",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "kind", required: true, shape: Shape::Vocab(&vocab::DEVICE_KIND) },
                Field { key: "description", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "long-term medication",
        group: "health",
        block: "health",
        key: "medications",
        path: "health.medications",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "substance", required: true, shape: Shape::Text },
                Field { key: "dose", required: false, shape: Shape::Text },
                Field { key: "indication", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "drug allergies; food allergies; environmental allergies",
        group: "health",
        block: "health",
        key: "allergies",
        path: "health.allergies",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "type", required: true, shape: Shape::Vocab(&vocab::ALLERGY_TYPE) },
                Field { key: "allergen", required: true, shape: Shape::Text },
                Field { key: "severity", required: false, shape: Shape::Vocab(&vocab::ALLERGY_SEVERITY) },
                Field { key: "reaction", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "vaccination status",
        group: "health",
        block: "health",
        key: "vaccinations",
        path: "health.vaccinations",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "pathogen", required: true, shape: Shape::Vocab(&vocab::PATHOGEN) },
                Field { key: "status", required: true, shape: Shape::Vocab(&vocab::VACCINATION_STATUS) },
                Field { key: "dose", required: false, shape: Shape::Integer { min: Some(1), max: Some(20), unit: "" } },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "serology status",
        group: "health",
        block: "health",
        key: "serology",
        path: "health.serology",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "pathogen", required: true, shape: Shape::Vocab(&vocab::PATHOGEN) },
                Field { key: "result", required: true, shape: Shape::Vocab(&vocab::SEROLOGY_RESULT) },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "basic metabolic panel; lipid panel; liver panel; renal panel; HbA1c; iron metabolism",
        group: "health",
        block: "health",
        key: "lab_results",
        path: "health.lab_results",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "panel", required: true, shape: Shape::Vocab(&vocab::LAB_PANEL) },
                Field { key: "analyte", required: true, shape: Shape::Vocab(&vocab::LAB_ANALYTE) },
                Field { key: "result", required: true, shape: Shape::Number { min: None, max: None, unit: "" } },
                Field { key: "unit", required: true, shape: Shape::Vocab(&vocab::LAB_UNIT) },
                Field { key: "reference_low", required: false, shape: Shape::Number { min: None, max: None, unit: "" } },
                Field { key: "reference_high", required: false, shape: Shape::Number { min: None, max: None, unit: "" } },
                Field { key: "flag", required: false, shape: Shape::Vocab(&vocab::LAB_FLAG) },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "vitamin and mineral deficiencies",
        group: "health",
        block: "health",
        key: "deficiencies",
        path: "health.deficiencies",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "nutrient", required: true, shape: Shape::Vocab(&vocab::NUTRIENT) },
                Field { key: "description", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "sleep disorders",
        group: "health",
        block: "health",
        key: "sleep_disorders",
        path: "health.sleep_disorders",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "category", required: true, shape: Shape::Vocab(&vocab::SLEEP_DISORDER) },
                Field { key: "description", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "mental health assessment",
        group: "health",
        block: "health",
        key: "mental_health_assessments",
        path: "health.mental_health_assessments",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field { key: "instrument", required: true, shape: Shape::Vocab(&vocab::ASSESSMENT_INSTRUMENT) },
                Field { key: "score", required: false, shape: Shape::Number { min: Some(0.0), max: Some(1000.0), unit: "" } },
                Field { key: "severity", required: false, shape: Shape::Vocab(&vocab::ASSESSMENT_SEVERITY) },
                Field { key: "summary", required: false, shape: Shape::Text },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
];

/// The attributes of *Genomics*.
pub const GENOMICS_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "autosomal DNA mapping",
        group: "genomics",
        block: "genomics",
        key: "autosomal_mapping",
        path: "genomics.autosomal_mapping",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "provider",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "test",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "reference_build",
                    required: false,
                    shape: Shape::Vocab(&vocab::REFERENCE_BUILD),
                },
                Field {
                    key: "snp_count",
                    required: false,
                    shape: Shape::Integer {
                        min: Some(1),
                        max: Some(100000000),
                        unit: "",
                    },
                },
                Field {
                    key: "document_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "Y haplogroup",
        group: "genomics",
        block: "genomics",
        key: "y_haplogroup",
        path: "genomics.y_haplogroup",
        cardinality: Cardinality::Single,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "major",
                    required: true,
                    shape: Shape::Vocab(&vocab::Y_HAPLOGROUP),
                },
                Field {
                    key: "subclade",
                    required: false,
                    shape: Shape::Pattern(
                        "^[A-T](0{1,2})?([0-9a-z]*|-[A-Z]{1,5}[0-9]+(\\.[0-9]+)*)$",
                    ),
                },
                Field {
                    key: "tree_version",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "mitochondrial haplogroup",
        group: "genomics",
        block: "genomics",
        key: "mt_haplogroup",
        path: "genomics.mt_haplogroup",
        cardinality: Cardinality::Single,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "major",
                    required: true,
                    shape: Shape::Vocab(&vocab::MT_HAPLOGROUP),
                },
                Field {
                    key: "subclade",
                    required: false,
                    shape: Shape::Pattern("^(L[0-6]|HV|[A-Z])[0-9A-Za-z'+.*]*$"),
                },
                Field {
                    key: "tree_version",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "whole genome sequencing",
        group: "genomics",
        block: "genomics",
        key: "whole_genome_sequencing",
        path: "genomics.whole_genome_sequencing",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "provider",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "coverage",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(10000.0),
                        unit: "x",
                    },
                },
                Field {
                    key: "reference_build",
                    required: false,
                    shape: Shape::Vocab(&vocab::REFERENCE_BUILD),
                },
                Field {
                    key: "file_format",
                    required: false,
                    shape: Shape::Vocab(&vocab::GENOMIC_FILE_FORMAT),
                },
                Field {
                    key: "document_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "risk variants",
        group: "genomics",
        block: "genomics",
        key: "risk_variants",
        path: "genomics.risk_variants",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "variant",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "gene",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "zygosity",
                    required: false,
                    shape: Shape::Vocab(&vocab::ZYGOSITY),
                },
                Field {
                    key: "significance",
                    required: false,
                    shape: Shape::Vocab(&vocab::CLINICAL_SIGNIFICANCE),
                },
                Field {
                    key: "condition",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "hereditary pathologies",
        group: "genomics",
        block: "genomics",
        key: "hereditary_conditions",
        path: "genomics.hereditary_conditions",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "inheritance",
                    required: false,
                    shape: Shape::Vocab(&vocab::INHERITANCE_PATTERN),
                },
                Field {
                    key: "carrier_status",
                    required: false,
                    shape: Shape::Vocab(&vocab::CARRIER_STATUS),
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "genetic predispositions",
        group: "genomics",
        block: "genomics",
        key: "predispositions",
        path: "genomics.predispositions",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "polygenic_score",
                    required: false,
                    shape: Shape::Number {
                        min: None,
                        max: None,
                        unit: "",
                    },
                },
                Field {
                    key: "percentile",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(100.0),
                        unit: "",
                    },
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "epigenetic markers",
        group: "genomics",
        block: "genomics",
        key: "epigenetic_markers",
        path: "genomics.epigenetic_markers",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "marker",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "epigenetic biological age",
        group: "genomics",
        block: "genomics",
        key: "epigenetic_age",
        path: "genomics.epigenetic_age",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "clock",
                    required: true,
                    shape: Shape::Vocab(&vocab::EPIGENETIC_CLOCK),
                },
                Field {
                    key: "age_years",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(200.0),
                        unit: "years",
                    },
                },
                Field {
                    key: "pace",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(5.0),
                        unit: "",
                    },
                },
            ],
            one_of: &[&["age_years"], &["pace"]],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "gut microbiome",
        group: "genomics",
        block: "genomics",
        key: "gut_microbiome",
        path: "genomics.gut_microbiome",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "provider",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "shannon_diversity",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(20.0),
                        unit: "",
                    },
                },
                Field {
                    key: "summary",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "document_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "skin microbiome",
        group: "genomics",
        block: "genomics",
        key: "skin_microbiome",
        path: "genomics.skin_microbiome",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "provider",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "shannon_diversity",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(20.0),
                        unit: "",
                    },
                },
                Field {
                    key: "summary",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "document_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
    Attribute {
        name: "toxicological sensitivities",
        group: "genomics",
        block: "genomics",
        key: "toxicological_sensitivities",
        path: "genomics.toxicological_sensitivities",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "substance",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "gene",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "metaboliser_status",
                    required: false,
                    shape: Shape::Vocab(&vocab::METABOLISER_STATUS),
                },
                Field {
                    key: "description",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Genomics),
    },
];

/// The attributes of *Death*.
pub const DEATH_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "death time",
        group: "death",
        block: "death",
        key: "time",
        path: "death.time",
        cardinality: Cardinality::Single,
        shape: Shape::TimeOfDay,
        class: None,
    },
    Attribute {
        name: "death coordinates",
        group: "death",
        block: "death",
        key: "coordinates",
        path: "death.coordinates",
        cardinality: Cardinality::Single,
        shape: Shape::Coordinates,
        class: None,
    },
    Attribute {
        name: "direct causes",
        group: "death",
        block: "death",
        key: "causes",
        path: "death.causes",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "icd10_chapter",
                    required: false,
                    shape: Shape::Vocab(&vocab::ICD10_CHAPTER),
                },
                Field {
                    key: "sequence",
                    required: false,
                    shape: Shape::Integer {
                        min: Some(1),
                        max: Some(10),
                        unit: "",
                    },
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "contributing factors",
        group: "death",
        block: "death",
        key: "contributing_factors",
        path: "death.contributing_factors",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "icd10_chapter",
                    required: false,
                    shape: Shape::Vocab(&vocab::ICD10_CHAPTER),
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "autopsy and forensic report",
        group: "death",
        block: "death",
        key: "autopsy",
        path: "death.autopsy",
        cardinality: Cardinality::Single,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "kind",
                    required: true,
                    shape: Shape::Vocab(&vocab::AUTOPSY),
                },
                Field {
                    key: "findings",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "document_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "burial or cremation place",
        group: "death",
        block: "death",
        key: "disposition",
        path: "death.disposition",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "method",
                    required: true,
                    shape: Shape::Vocab(&vocab::DISPOSITION),
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "grave coordinates",
        group: "death",
        block: "death",
        key: "grave",
        path: "death.grave",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "coordinates",
                    required: false,
                    shape: Shape::Coordinates,
                },
                Field {
                    key: "plot",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "inscription",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[&["coordinates"], &["plot"]],
        },
        class: None,
    },
];

/// The attributes of *Residence and nationality*.
pub const RESIDENCE_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "principal address; chronological address history",
        group: "residence",
        block: "residence",
        key: "addresses",
        path: "residence.addresses",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "use",
                    required: false,
                    shape: Shape::Vocab(&vocab::ADDRESS_USE),
                },
                Field {
                    key: "lines",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
                Field {
                    key: "postal_code",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "country",
                    required: false,
                    shape: Shape::Vocab(&vocab::COUNTRY),
                },
                Field {
                    key: "coordinates",
                    required: false,
                    shape: Shape::Coordinates,
                },
            ],
            one_of: &[&["lines"], &["place_id"]],
        },
        class: None,
    },
    Attribute {
        name: "nationality of origin",
        group: "residence",
        block: "residence",
        key: "nationality_of_origin",
        path: "residence.nationality_of_origin",
        cardinality: Cardinality::Single,
        shape: Shape::Vocab(&vocab::COUNTRY),
        class: None,
    },
    Attribute {
        name: "acquired nationalities",
        group: "residence",
        block: "residence",
        key: "acquired_nationalities",
        path: "residence.acquired_nationalities",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "country",
                    required: true,
                    shape: Shape::Vocab(&vocab::COUNTRY),
                },
                Field {
                    key: "mode",
                    required: false,
                    shape: Shape::Vocab(&vocab::NATIONALITY_MODE),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "mother tongue",
        group: "residence",
        block: "residence",
        key: "mother_tongue",
        path: "residence.mother_tongue",
        cardinality: Cardinality::Series,
        shape: Shape::LanguageTag,
        class: None,
    },
    Attribute {
        name: "spoken languages",
        group: "residence",
        block: "residence",
        key: "spoken_languages",
        path: "residence.spoken_languages",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "language",
                    required: true,
                    shape: Shape::LanguageTag,
                },
                Field {
                    key: "proficiency",
                    required: false,
                    shape: Shape::Vocab(&vocab::LANGUAGE_PROFICIENCY),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
];

/// The attributes of *Education and work*.
pub const EDUCATION_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "education level",
        group: "education",
        block: "education",
        key: "level",
        path: "education.level",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::ISCED_LEVEL),
        class: None,
    },
    Attribute {
        name: "diplomas",
        group: "education",
        block: "education",
        key: "diplomas",
        path: "education.diplomas",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "title",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "isced_level",
                    required: false,
                    shape: Shape::Vocab(&vocab::ISCED_LEVEL),
                },
                Field {
                    key: "institution",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "institutions attended",
        group: "education",
        block: "education",
        key: "institutions",
        path: "education.institutions",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "name",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "isced_level",
                    required: false,
                    shape: Shape::Vocab(&vocab::ISCED_LEVEL),
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "average income level",
        group: "education",
        block: "education",
        key: "income",
        path: "education.income",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "quintile",
                    required: false,
                    shape: Shape::Vocab(&vocab::INCOME_QUINTILE),
                },
                Field {
                    key: "amount",
                    required: false,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: None,
                        unit: "",
                    },
                },
                Field {
                    key: "currency",
                    required: false,
                    shape: Shape::CurrencyCode,
                },
                Field {
                    key: "period",
                    required: false,
                    shape: Shape::Vocab(&vocab::PAY_PERIOD),
                },
            ],
            one_of: &[&["quintile"], &["amount"]],
        },
        class: None,
    },
    Attribute {
        name: "real estate holdings",
        group: "education",
        block: "education",
        key: "real_estate",
        path: "education.real_estate",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "description",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "tenure",
                    required: false,
                    shape: Shape::Vocab(&vocab::TENURE),
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
                Field {
                    key: "document_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
];

/// The attributes of *Military and honours*.
pub const MILITARY_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "honorific distinctions",
        group: "military",
        block: "military",
        key: "distinctions",
        path: "military.distinctions",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "name",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "kind",
                    required: false,
                    shape: Shape::Vocab(&vocab::DISTINCTION_KIND),
                },
                Field {
                    key: "conferred_by",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "country",
                    required: false,
                    shape: Shape::Vocab(&vocab::COUNTRY),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "military citations",
        group: "military",
        block: "military",
        key: "citations",
        path: "military.citations",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "text",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "unit",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "rank",
        group: "military",
        block: "military",
        key: "ranks",
        path: "military.ranks",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "country",
                    required: false,
                    shape: Shape::Vocab(&vocab::COUNTRY),
                },
                Field {
                    key: "service",
                    required: false,
                    shape: Shape::Vocab(&vocab::MILITARY_SERVICE),
                },
                Field {
                    key: "rank",
                    required: false,
                    shape: Shape::Rank,
                },
                Field {
                    key: "rank_text",
                    required: false,
                    shape: Shape::Text,
                },
                Field {
                    key: "category",
                    required: false,
                    shape: Shape::Vocab(&vocab::RANK_CATEGORY),
                },
            ],
            one_of: &[&["rank"], &["rank_text"]],
        },
        class: None,
    },
    Attribute {
        name: "regiment",
        group: "military",
        block: "military",
        key: "units",
        path: "military.units",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "service number",
        group: "military",
        block: "military",
        key: "service_numbers",
        path: "military.service_numbers",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "number",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "service",
                    required: false,
                    shape: Shape::Vocab(&vocab::MILITARY_SERVICE),
                },
                Field {
                    key: "country",
                    required: false,
                    shape: Shape::Vocab(&vocab::COUNTRY),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
];

/// The attributes of *Legal*.
pub const LEGAL_ATTRIBUTES: &[Attribute] = &[Attribute {
    name: "criminal record",
    group: "legal",
    block: "legal",
    key: "criminal_record",
    path: "legal.criminal_record",
    cardinality: Cardinality::Series,
    shape: Shape::Object {
        fields: &[
            Field {
                key: "offence",
                required: true,
                shape: Shape::Text,
            },
            Field {
                key: "iccs_section",
                required: false,
                shape: Shape::Vocab(&vocab::ICCS_SECTION),
            },
            Field {
                key: "jurisdiction",
                required: false,
                shape: Shape::Vocab(&vocab::COUNTRY),
            },
            Field {
                key: "court",
                required: false,
                shape: Shape::Text,
            },
            Field {
                key: "outcome",
                required: false,
                shape: Shape::Vocab(&vocab::CASE_OUTCOME),
            },
            Field {
                key: "sentence",
                required: false,
                shape: Shape::Text,
            },
        ],
        one_of: &[],
    },
    class: Some(SensitiveClass::Legal),
}];

/// The attributes of *Belief and affiliation*.
pub const BELIEF_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "religion",
        group: "belief",
        block: "belief",
        key: "religions",
        path: "belief.religions",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "tradition",
                    required: true,
                    shape: Shape::Vocab(&vocab::RELIGION),
                },
                Field {
                    key: "denomination",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "sacraments received",
        group: "belief",
        block: "belief",
        key: "sacraments",
        path: "belief.sacraments",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "sacrament",
                    required: true,
                    shape: Shape::Vocab(&vocab::SACRAMENT),
                },
                Field {
                    key: "place_id",
                    required: false,
                    shape: Shape::Uuid,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "individual beliefs",
        group: "belief",
        block: "belief",
        key: "beliefs",
        path: "belief.beliefs",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "political leanings",
        group: "belief",
        block: "belief",
        key: "political_leanings",
        path: "belief.political_leanings",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "position",
                    required: false,
                    shape: Shape::Vocab(&vocab::POLITICAL_POSITION),
                },
                Field {
                    key: "party",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[&["position"], &["party"]],
        },
        class: Some(SensitiveClass::Health),
    },
    Attribute {
        name: "union and association memberships",
        group: "belief",
        block: "belief",
        key: "memberships",
        path: "belief.memberships",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "organisation",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "kind",
                    required: false,
                    shape: Shape::Vocab(&vocab::MEMBERSHIP_KIND),
                },
                Field {
                    key: "role",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
];

/// The attributes of *Personality and behaviour*.
pub const PERSONALITY_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "personality traits (Big Five)",
        group: "personality",
        block: "personality",
        key: "big_five",
        path: "personality.big_five",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "openness",
                    required: true,
                    shape: Shape::Integer {
                        min: Some(0),
                        max: Some(100),
                        unit: "",
                    },
                },
                Field {
                    key: "conscientiousness",
                    required: true,
                    shape: Shape::Integer {
                        min: Some(0),
                        max: Some(100),
                        unit: "",
                    },
                },
                Field {
                    key: "extraversion",
                    required: true,
                    shape: Shape::Integer {
                        min: Some(0),
                        max: Some(100),
                        unit: "",
                    },
                },
                Field {
                    key: "agreeableness",
                    required: true,
                    shape: Shape::Integer {
                        min: Some(0),
                        max: Some(100),
                        unit: "",
                    },
                },
                Field {
                    key: "neuroticism",
                    required: true,
                    shape: Shape::Integer {
                        min: Some(0),
                        max: Some(100),
                        unit: "",
                    },
                },
                Field {
                    key: "instrument",
                    required: false,
                    shape: Shape::Vocab(&vocab::PERSONALITY_INSTRUMENT),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "personality traits (MBTI)",
        group: "personality",
        block: "personality",
        key: "mbti",
        path: "personality.mbti",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::MBTI),
        class: None,
    },
    Attribute {
        name: "introversion/extraversion",
        group: "personality",
        block: "personality",
        key: "introversion_extraversion",
        path: "personality.introversion_extraversion",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::INTROVERSION_EXTRAVERSION),
        class: None,
    },
    Attribute {
        name: "stress tolerance",
        group: "personality",
        block: "personality",
        key: "stress_tolerance",
        path: "personality.stress_tolerance",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::STRESS_TOLERANCE),
        class: None,
    },
    Attribute {
        name: "decision-making style",
        group: "personality",
        block: "personality",
        key: "decision_style",
        path: "personality.decision_style",
        cardinality: Cardinality::Series,
        shape: Shape::Vocab(&vocab::DECISION_STYLE),
        class: None,
    },
    Attribute {
        name: "interests",
        group: "personality",
        block: "personality",
        key: "interests",
        path: "personality.interests",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "hobbies",
        group: "personality",
        block: "personality",
        key: "hobbies",
        path: "personality.hobbies",
        cardinality: Cardinality::Series,
        shape: Shape::Text,
        class: None,
    },
    Attribute {
        name: "sport",
        group: "personality",
        block: "personality",
        key: "sports",
        path: "personality.sports",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "sport",
                    required: true,
                    shape: Shape::Text,
                },
                Field {
                    key: "level",
                    required: false,
                    shape: Shape::Vocab(&vocab::SPORT_LEVEL),
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "dietary habits",
        group: "personality",
        block: "personality",
        key: "dietary_habits",
        path: "personality.dietary_habits",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "diet",
                    required: true,
                    shape: Shape::Vocab(&vocab::DIET),
                },
                Field {
                    key: "details",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "dependencies and addictions (tobacco, alcohol, substances)",
        group: "personality",
        block: "personality",
        key: "dependencies",
        path: "personality.dependencies",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "substance",
                    required: true,
                    shape: Shape::Vocab(&vocab::SUBSTANCE),
                },
                Field {
                    key: "pattern",
                    required: false,
                    shape: Shape::Vocab(&vocab::USE_PATTERN),
                },
            ],
            one_of: &[],
        },
        class: Some(SensitiveClass::Health),
    },
];

/// The attributes of *Relationships*.
pub const RELATIONSHIPS_ATTRIBUTES: &[Attribute] = &[];

/// The attributes of *Digital legacy*.
pub const DIGITAL_LEGACY_ATTRIBUTES: &[Attribute] = &[
    Attribute {
        name: "source morphological 3D model (mesh or point cloud)",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "body_models",
        path: "digital_legacy.body_models",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["mesh", "point_cloud"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "HD skin texture map",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "skin_textures",
        path: "digital_legacy.skin_textures",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["skin_texture_map"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "skeletal and articular rigging",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "rigs",
        path: "digital_legacy.rigs",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["skeletal_rig"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "source audio for voice synthesis",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "voice_corpora",
        path: "digital_legacy.voice_corpora",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["voice_corpus"]),
        class: Some(SensitiveClass::Biometrics),
    },
    Attribute {
        name: "written corpus for LLM training",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "text_corpora",
        path: "digital_legacy.text_corpora",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["text_corpus"]),
        class: None,
    },
    Attribute {
        name: "digital trace history",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "digital_traces",
        path: "digital_legacy.digital_traces",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["trace_archive"]),
        class: None,
    },
    Attribute {
        name: "estimated carbon footprint",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "carbon_footprint",
        path: "digital_legacy.carbon_footprint",
        cardinality: Cardinality::Series,
        shape: Shape::Object {
            fields: &[
                Field {
                    key: "tonnes_co2e_per_year",
                    required: true,
                    shape: Shape::Number {
                        min: Some(0.0),
                        max: Some(100000.0),
                        unit: "t CO2e/yr",
                    },
                },
                Field {
                    key: "method",
                    required: false,
                    shape: Shape::Text,
                },
            ],
            one_of: &[],
        },
        class: None,
    },
    Attribute {
        name: "AI-driven reactive behaviour model",
        group: "digital_legacy",
        block: "digital_legacy",
        key: "behaviour_models",
        path: "digital_legacy.behaviour_models",
        cardinality: Cardinality::Series,
        shape: Shape::Artefact(&["behaviour_model"]),
        class: None,
    },
];

/// The fourteen groups, in the specification's order.
pub const GROUPS: &[Group] = &[
    Group {
        key: "identity",
        title: "Identity and civil status",
        attributes: IDENTITY_ATTRIBUTES,
        mapped: &[
            Mapped { name: "full name", location: "identity.name.display" },
            Mapped { name: "given names", location: "identity.name.components[type=given_name]" },
            Mapped { name: "nicknames", location: "identity.names[type=nickname], identity.name.components[type=nickname]" },
            Mapped { name: "birth date", location: "birth.date" },
            Mapped { name: "birth place", location: "birth.place_id" },
        ],
    },
    Group {
        key: "morphology",
        title: "Morphology",
        attributes: MORPHOLOGY_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "biometrics",
        title: "Biometrics",
        attributes: BIOMETRICS_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "health",
        title: "Health",
        attributes: HEALTH_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "genomics",
        title: "Genomics",
        attributes: GENOMICS_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "death",
        title: "Death",
        attributes: DEATH_ATTRIBUTES,
        mapped: &[
            Mapped { name: "death date", location: "death.date" },
            Mapped { name: "death place", location: "death.place_id" },
        ],
    },
    Group {
        key: "residence",
        title: "Residence and nationality",
        attributes: RESIDENCE_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "education",
        title: "Education and work",
        attributes: EDUCATION_ATTRIBUTES,
        mapped: &[
            Mapped { name: "occupations held", location: "Occupation entities with `person_id` (1.0 §4.5)" },
            Mapped { name: "positions", location: "Occupation.position (new in 1.1)" },
            Mapped { name: "successive employers", location: "Occupation.employer, ordered by `valid_from` (1.0 §4.5)" },
        ],
    },
    Group {
        key: "military",
        title: "Military and honours",
        attributes: MILITARY_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "legal",
        title: "Legal",
        attributes: LEGAL_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "belief",
        title: "Belief and affiliation",
        attributes: BELIEF_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "personality",
        title: "Personality and behaviour",
        attributes: PERSONALITY_ATTRIBUTES,
        mapped: &[
        ],
    },
    Group {
        key: "relationships",
        title: "Relationships",
        attributes: RELATIONSHIPS_ATTRIBUTES,
        mapped: &[
            Mapped { name: "biological father; biological mother", location: "Family where the person is a child with `lineage: biological`; the partners of its union" },
            Mapped { name: "adoptive parents", location: "Family where the person is a child with `lineage: adoptive`" },
            Mapped { name: "siblings", location: "the other children of the families the person is a child of" },
            Mapped { name: "sibling rank", location: "Family.children[].birth_order (1.0)" },
            Mapped { name: "spouses", location: "Family.union.persons of the families the person is a partner in" },
            Mapped { name: "marriage and civil-union dates", location: "Family.union.start.date, with union.type marriage or civil_union (1.0)" },
            Mapped { name: "divorce and separation dates", location: "Family.union.end.date, with union.status ended_by_divorce or ended_by_separation (1.0)" },
            Mapped { name: "children", location: "Family.children of the families the person is a partner in (1.0)" },
            Mapped { name: "godparents", location: "Link with `relation: godparent`, category spiritual" },
            Mapped { name: "official witnesses", location: "Link with `relation: witness`; Event participants with role witness (1.0)" },
            Mapped { name: "business relationships", location: "Link with category professional and `relation` business_partner, employer or employee" },
            Mapped { name: "close friends", location: "Link with `relation: close_friend`, category social" },
        ],
    },
    Group {
        key: "digital_legacy",
        title: "Digital legacy",
        attributes: DIGITAL_LEGACY_ATTRIBUTES,
        mapped: &[
        ],
    },
];

/// The Person blocks 1.1 introduces. Anything under one of these keys is
/// 1.1 content; 1.1 also adds attributes inside the 1.0 blocks `identity`,
/// `birth` and `death`, which [`attributes`] lists with the rest.
pub const PROFILE_BLOCKS: &[&str] = &[
    "civil_status",
    "morphology",
    "biometrics",
    "health",
    "genomics",
    "residence",
    "education",
    "military",
    "legal",
    "belief",
    "personality",
    "digital_legacy",
];

/// Every attribute, in group order.
pub fn attributes() -> impl Iterator<Item = &'static Attribute> {
    GROUPS.iter().flat_map(|g| g.attributes.iter())
}

/// The attribute at `path` (`health.blood_group`), if the profile defines one.
pub fn attribute(path: &str) -> Option<&'static Attribute> {
    attributes().find(|a| a.path == path)
}

/// The group with this key.
pub fn group(key: &str) -> Option<&'static Group> {
    GROUPS.iter().find(|g| g.key == key)
}

/// Every attribute of one sensitive class, wherever its group puts it.
pub fn attributes_of_class(class: SensitiveClass) -> impl Iterator<Item = &'static Attribute> {
    attributes().filter(move |a| a.class == Some(class))
}
