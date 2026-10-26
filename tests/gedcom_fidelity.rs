// SPDX-License-Identifier: Apache-2.0
//! What a faithful GEDCOM conversion keeps (0.6.1).
//!
//! The 31 defects the CMS's stress campaign of 22 October 2026 found in
//! the 0.5.0 converter, each stated as what a faithful conversion does.
//! They were ignored tests in the CMS (`tests/gedcom_known_defects.rs`);
//! here they hold the converter to them.
//!
//! Every name, place and date here is invented.

use serde_json::Value;

/// A file converted: the bundle, the converter's diagnostics, and the
/// validator's.
struct Converted {
    bundle: Value,
    diagnostics: Vec<String>,
    validation: Vec<String>,
}

fn convert(body: &[&str]) -> Converted {
    let mut text =
        String::from("0 HEAD\n1 GEDC\n2 VERS 5.5.1\n2 FORM LINEAGE-LINKED\n1 CHAR UTF-8\n");
    for l in body {
        text.push_str(l);
        text.push('\n');
    }
    text.push_str("0 TRLR\n");
    let env = axgf_rs::convert_gedcom(text.as_bytes(), 0.8, "fr");
    let diagnostics = env
        .diagnostics
        .iter()
        .map(|d| format!("{}: {}", d.code.as_str(), d.message))
        .collect();
    let bundle = env.data["bundle"].clone();
    let v = axgf_rs::validate(&bundle.to_string());
    let validation = v
        .diagnostics
        .iter()
        .map(|d| format!("{}/{:?}: {}", d.code.as_str(), d.severity, d.message))
        .collect();
    Converted {
        bundle,
        diagnostics,
        validation,
    }
}

impl Converted {
    fn persons(&self) -> Vec<&Value> {
        let mut v: Vec<&Value> = self.bundle["persons"]
            .as_object()
            .map(|m| m.values().collect())
            .unwrap_or_default();
        v.sort_by_key(|p| {
            p["identity"]["name"]["display"]
                .as_str()
                .unwrap_or("")
                .to_string()
        });
        v
    }
    fn person(&self, display: &str) -> &Value {
        self.persons()
            .into_iter()
            .find(|p| p["identity"]["name"]["display"] == display)
            .unwrap_or_else(|| panic!("no {display}"))
    }
    fn birth(&self, display: &str) -> Value {
        self.person(display)["birth"]["date"].clone()
    }
    fn families(&self) -> Vec<&Value> {
        self.bundle["families"]
            .as_object()
            .map(|m| m.values().collect())
            .unwrap_or_default()
    }
    fn n(&self, collection: &str) -> usize {
        self.bundle[collection].as_object().map_or(0, |m| m.len())
    }
    fn schema_failures(&self) -> Vec<&String> {
        self.validation
            .iter()
            .filter(|d| d.starts_with("SCHEMA_VALIDATION_FAILED"))
            .collect()
    }
}

fn born(date: &str) -> Converted {
    convert(&[
        "0 @I1@ INDI",
        "1 NAME Date /Cas/",
        "1 BIRT",
        &format!("2 DATE {date}"),
    ])
}

// --------------------------------------------------------------------- dates

/// "ABT 12 MAR 1850" is about a day, not about a year.
#[test]
fn an_approximate_full_date_keeps_its_precision() {
    let c = born("ABT 12 MAR 1850");
    assert_eq!(c.birth("Date Cas")["value"], "1850-03-12");
    assert_eq!(c.birth("Date Cas")["precision"], "exact");
    assert_eq!(c.birth("Date Cas")["circa"], true);
    let c = born("ABT MAR 1850");
    assert_eq!(c.birth("Date Cas")["precision"], "month");
}

/// GEDCOM's other two approximations: calculated and estimated.
#[test]
fn calculated_and_estimated_dates_are_dates() {
    for d in ["CAL 1850", "EST 1850"] {
        let c = born(d);
        assert_eq!(c.birth("Date Cas")["value"], "1850", "{d}");
        assert_eq!(c.birth("Date Cas")["circa"], true, "{d}");
    }
}

/// A period: FROM … TO …, FROM …, TO ….
#[test]
fn periods_are_ranges() {
    let c = born("FROM 1850 TO 1860");
    assert_eq!(c.birth("Date Cas")["range"]["earliest"]["value"], "1850");
    assert_eq!(c.birth("Date Cas")["range"]["latest"]["value"], "1860");
}

/// An interpreted date: the date, and the words it was read from.
#[test]
fn an_interpreted_date_keeps_its_date_and_its_phrase() {
    let c = born("INT 1850 (selon la famille)");
    assert_eq!(c.birth("Date Cas")["value"], "1850");
    assert!(c.birth("Date Cas")["note"]
        .as_str()
        .unwrap_or("")
        .contains("selon la famille"));
}

/// A dual year, before the new year moved to January.
#[test]
fn a_dual_year_is_a_date() {
    let c = born("12 FEB 1750/51");
    assert!(
        c.birth("Date Cas")["value"]
            .as_str()
            .is_some_and(|v| v.starts_with("175")),
        "{}",
        c.birth("Date Cas")
    );
}

/// AXGF has calendars for Julian, French Republican and Hebrew dates;
/// GEDCOM marks them with an escape.
#[test]
fn dates_in_other_calendars_are_read_in_their_calendar() {
    for (d, cal) in [
        ("@#DJULIAN@ 1 JAN 1700", "julian"),
        ("@#DFRENCH R@ 18 BRUM 0008", "republican_french"),
        ("@#DHEBREW@ 1 TSH 5600", "hebrew"),
    ] {
        let c = born(d);
        assert_eq!(c.birth("Date Cas")["calendar"], cal, "{d}");
    }
}

/// The thirty-first of February is not an exact date.
#[test]
fn an_impossible_date_is_not_taken_as_exact() {
    for d in ["31 FEB 1900", "0 JAN 1900", "32 JAN 1900"] {
        let c = born(d);
        assert_ne!(
            c.birth("Date Cas")["precision"],
            "exact",
            "{d}: {}",
            c.birth("Date Cas")
        );
    }
}

/// A year outside 0–9999 has no ISO 8601 form the schema accepts.
#[test]
fn a_year_out_of_range_does_not_make_the_bundle_invalid() {
    for d in ["12 MAR 99999", "12 MAR -5"] {
        assert!(
            born(d).schema_failures().is_empty(),
            "{d}: {:?}",
            born(d).schema_failures()
        );
    }
}

// --------------------------------------------------------------------- names

/// "Jean /Dupont/ Jr.": the schema has no `suffix` component.
#[test]
fn a_name_with_a_suffix_makes_a_valid_bundle() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Jean /Dupont/ Jr.",
        "0 @I2@ INDI",
        "1 NAME /Dupont/ Jean",
    ]);
    assert!(c.schema_failures().is_empty(), "{:?}", c.schema_failures());
}

/// A name written as its pieces, with no NAME value.
#[test]
fn a_name_written_as_pieces_is_a_name() {
    let c = convert(&["0 @I1@ INDI", "1 NAME", "2 GIVN Pièces", "2 SURN Séparées"]);
    assert_eq!(
        c.persons()[0]["identity"]["name"]["display"],
        "Pièces Séparées"
    );
}

/// A name with no closing slash, or two surnames: no slash in a component.
#[test]
fn no_component_keeps_a_slash() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Jean /Dupont",
        "0 @I2@ INDI",
        "1 NAME Jean /Dupont/ /Martin/",
    ]);
    for p in c.persons() {
        for comp in p["identity"]["name"]["components"].as_array().unwrap() {
            assert!(!comp["value"].as_str().unwrap().contains('/'), "{comp}");
        }
    }
}

/// A birth name and a married name are typed as such.
#[test]
fn a_birth_name_is_typed_birth() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Marie /Épousée/",
        "2 TYPE married",
        "1 NAME Maria /Néeainsi/",
        "2 TYPE birth",
    ]);
    let names = &c.persons()[0]["identity"]["names"];
    assert!(
        names
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["type"] == "birth"),
        "{names}"
    );
}

/// GEDCOM 7's `SEX X`.
#[test]
fn gedcom_7_sex_x_is_not_unknown() {
    let c = convert(&["0 @I1@ INDI", "1 NAME Sexe /X/", "1 SEX X"]);
    assert_ne!(c.persons()[0]["identity"]["gender"]["value"], "U");
}

// -------------------------------------------------------------------- events

/// Christenings, burials, residences, censuses, emigrations, generic
/// events: carried, or at least said to be left behind. Today they vanish
/// without a diagnostic, and the conversion page tells the reader that the
/// skipped tags it lists prove nothing was silently dropped.
#[test]
fn every_event_is_carried_or_reported() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Toute /Saisons/",
        "1 CHR",
        "2 DATE 3 JAN 1850",
        "1 BAPM",
        "2 DATE 4 JAN 1850",
        "1 RESI",
        "2 PLAC Rennes",
        "1 CENS",
        "2 DATE 1876",
        "1 EMIG",
        "2 DATE 1880",
        "1 IMMI",
        "2 DATE 1881",
        "1 EVEN",
        "2 TYPE Médaille",
        "2 DATE 1900",
        "1 BURI",
        "2 DATE 1930",
    ]);
    let carried = c.n("events");
    let reported = c.diagnostics.len();
    assert!(
        carried + reported >= 8,
        "events: {carried}, diagnostics: {:?}",
        c.diagnostics
    );
}

/// Someone buried or cremated has died, even with no DEAT line.
#[test]
fn a_burial_means_a_death() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Enterré /Sansdécès/",
        "1 BURI",
        "2 DATE 1801",
    ]);
    assert_eq!(c.persons()[0]["identity"]["is_living"], false);
}

/// The cause of a death.
#[test]
fn the_cause_of_death_is_kept() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Mort /Cause/",
        "1 DEAT",
        "2 DATE 1930",
        "2 CAUS Grippe",
    ]);
    assert!(c.persons()[0].to_string().contains("Grippe"));
}

/// A divorce ends a union.
#[test]
fn a_divorce_ends_the_union() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Un /Couple/",
        "0 @I2@ INDI",
        "1 NAME Deux /Couple/",
        "0 @F1@ FAM",
        "1 HUSB @I1@",
        "1 WIFE @I2@",
        "1 MARR",
        "2 DATE 1875",
        "1 DIV",
        "2 DATE 1890",
    ]);
    assert!(
        c.families()[0]["union"].get("end").is_some(),
        "{}",
        c.families()[0]
    );
}

/// An occupation held from one year to another.
#[test]
fn an_occupation_period_has_both_ends() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Métier /Daté/",
        "1 OCCU Couturière",
        "2 DATE FROM 1868 TO 1880",
    ]);
    let occ = c.bundle["occupations"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(occ["valid_from"]["date"]["value"], "1868");
    assert_eq!(occ["valid_until"]["date"]["value"], "1880");
}

// ------------------------------------------------------------------- sources

/// A citation on a birth (which source, which page, how sure) is evidence;
/// the source itself is carried, the citation is not.
#[test]
fn a_citation_on_an_event_is_kept() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Cité /Source/",
        "1 BIRT",
        "2 DATE 1850",
        "2 SOUR @S1@",
        "3 PAGE acte 12",
        "3 QUAY 3",
        "0 @S1@ SOUR",
        "1 TITL Registre fictif",
    ]);
    let all = c.bundle.to_string();
    assert!(
        all.contains("acte 12"),
        "the page of the citation is nowhere in the bundle"
    );
}

// ------------------------------------------------------------------ families

/// An adoption recorded where GEDCOM 5.5.1 puts it: on the child's FAMC.
#[test]
fn an_adoption_on_the_childs_famc_is_its_lineage() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Père /Lignée/",
        "1 FAMS @F1@",
        "0 @I2@ INDI",
        "1 NAME Adopté /Lignée/",
        "1 FAMC @F1@",
        "2 PEDI adopted",
        "0 @F1@ FAM",
        "1 HUSB @I1@",
        "1 CHIL @I2@",
    ]);
    let child = &c.families()[0]["children"][0];
    assert_eq!(child["lineage"], "adoptive", "{child}");
}

/// A child listed twice in one family is one child.
#[test]
fn a_child_listed_twice_is_one_child() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Enfant /Double/",
        "0 @F1@ FAM",
        "1 CHIL @I1@",
        "1 CHIL @I1@",
    ]);
    assert_eq!(c.families()[0]["children"].as_array().unwrap().len(), 1);
}

/// A child whose own record names the family, though the family's record
/// forgot it.
#[test]
fn a_child_linked_only_from_its_own_record_belongs_to_the_family() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Père /Seul/",
        "1 FAMS @F1@",
        "0 @I2@ INDI",
        "1 NAME Enfant /Oublié/",
        "1 FAMC @F1@",
        "0 @F1@ FAM",
        "1 HUSB @I1@",
    ]);
    assert_eq!(
        c.families()[0]["children"].as_array().map(Vec::len),
        Some(1)
    );
}

/// A FAM record whose HUSB is a family and whose CHIL is a source.
#[test]
fn a_pointer_to_the_wrong_kind_of_record_is_not_a_person() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Type /Erroné/",
        "0 @F1@ FAM",
        "1 HUSB @F1@",
        "1 CHIL @S1@",
        "1 WIFE @I1@",
        "0 @S1@ SOUR",
        "1 TITL Une source",
    ]);
    let persons: Vec<String> = c.bundle["persons"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let fam = c.families()[0];
    for p in fam["union"]["persons"]
        .as_array()
        .unwrap()
        .iter()
        .chain(fam["children"].as_array().into_iter().flatten())
    {
        assert!(
            persons.contains(&p["person_id"].as_str().unwrap().to_string()),
            "{p} is not a person"
        );
    }
}

// --------------------------------------------------------------- records

/// Two records under one cross-reference: the first is lost without a word.
#[test]
fn a_duplicated_cross_reference_is_reported() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Premier /Doublon/",
        "0 @I1@ INDI",
        "1 NAME Second /Doublon/",
    ]);
    assert!(
        c.n("persons") == 2 || !c.diagnostics.is_empty(),
        "{:?}",
        c.diagnostics
    );
}

/// An INDI with no cross-reference is still somebody.
#[test]
fn a_person_without_a_cross_reference_is_kept_or_reported() {
    let c = convert(&["0 INDI", "1 NAME Sans /Référence/"]);
    assert!(c.n("persons") == 1 || !c.diagnostics.is_empty());
}

/// A note whose text happens to start and end with `@`.
#[test]
fn a_note_that_looks_like_a_pointer_is_kept() {
    let c = convert(&["0 @I1@ INDI", "1 NAME Note /Trompeuse/", "1 NOTE @home@"]);
    assert_eq!(c.persons()[0]["notes"], "@home@");
}

// -------------------------------------------------------------------- places

/// A place's coordinates.
#[test]
fn a_places_coordinates_are_kept() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Lieu /Coordonnées/",
        "1 BIRT",
        "2 PLAC Quimper, Finistère, France",
        "3 MAP",
        "4 LATI N47.9960",
        "4 LONG W4.0970",
    ]);
    let place = c.bundle["places"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert!(
        (place["coordinates"]["lat"].as_f64().unwrap() - 47.996).abs() < 1e-6,
        "{place}"
    );
    assert!(
        (place["coordinates"]["lon"].as_f64().unwrap() + 4.097).abs() < 1e-6,
        "{place}"
    );
}

/// One town written three ways is one place.
#[test]
fn one_town_written_three_ways_is_one_place() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Lieu /Un/",
        "1 BIRT",
        "2 PLAC Paris, France",
        "0 @I2@ INDI",
        "1 NAME Lieu /Deux/",
        "1 BIRT",
        "2 PLAC Paris,France",
        "0 @I3@ INDI",
        "1 NAME Lieu /Trois/",
        "1 BIRT",
        "2 PLAC paris, france",
    ]);
    assert_eq!(c.n("places"), 1);
}

/// An empty PLAC is no place.
#[test]
fn an_empty_place_is_no_place() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Lieu /Vide/",
        "1 BIRT",
        "2 DATE 1900",
        "2 PLAC",
    ]);
    assert_eq!(c.n("places"), 0);
}

// --------------------------------------------------------------------- media

/// One photograph attached twice to one person is attached once.
#[test]
fn a_photograph_referenced_twice_is_attached_once() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Média /Partagé/",
        "1 OBJE @M1@",
        "1 OBJE @M1@",
        "0 @M1@ OBJE",
        "1 FILE photos/portrait.jpg",
    ]);
    assert_eq!(c.persons()[0]["documents"].as_array().unwrap().len(), 1);
}

/// A file named by a Windows path is named by its last segment.
#[test]
fn a_windows_path_names_the_file_by_its_last_segment() {
    let c = convert(&[
        "0 @I1@ INDI",
        "1 NAME Chemin /Windows/",
        "1 OBJE",
        "2 FILE C:\\Users\\Fictif\\portrait.JPG",
        "2 FORM JPEG",
    ]);
    let doc = c.bundle["documents"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(doc["filename"], "portrait.JPG");
}

/// Every file above converts to a bundle the schema accepts.
#[test]
fn every_conversion_here_is_valid() {
    for body in [
        &[
            "0 @I1@ INDI",
            "1 NAME Jean /Dupont/ Jr.",
            "1 SEX X",
            "1 BURI",
            "2 DATE 1801",
        ][..],
        &[
            "0 @I1@ INDI",
            "1 NAME",
            "2 GIVN Pièces",
            "2 SURN Séparées",
            "1 BIRT",
            "2 DATE @#DFRENCH R@ 18 BRUM 0008",
        ][..],
        &[
            "0 @I1@ INDI",
            "1 NAME Jean /Dupont/ /Martin/",
            "1 BIRT",
            "2 DATE 31 FEB 1900",
            "2 PLAC",
            "1 CHR",
            "2 DATE 12 FEB 1750/51",
        ][..],
    ] {
        let c = convert(body);
        assert!(
            c.schema_failures().is_empty(),
            "{body:?}: {:?}",
            c.schema_failures()
        );
    }
}
