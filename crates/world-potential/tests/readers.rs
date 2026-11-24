//! The coefficient readers on small fixture files: every value written
//! is the value read, header facts are carried, and refusals are typed.

#![cfg(feature = "alloc")]

use world_core::{DecimalYear, ValidityWindow};
use world_potential::{
    Derivatives, GaussRow, MagneticModel, ParseErrorKind, TideSystem, VectorField, WorkspaceBuffer,
    read_cof, read_gfc, read_shadr, schmidt_to_geodesy, triangular_index,
};

const GFC: &str = include_str!("data/small.gfc");
const COF: &str = include_str!("data/small.COF");
/// The first 976 bytes (header and six coefficient records, CR LF
/// endings, space padded) of GGGRX_1200A_SHA.TAB, the GRAIL GRGM1200A
/// field in the PDS archive (pds-geosciences.wustl.edu,
/// grail-l-lgrs-5-rdr-v1/grail_1001/shadr), sha256 of this excerpt
/// 477d54ab2b05796dbe00cb29eaf95615b232e9647ae77fa470b6432274595abd.
const SHADR: &str = include_str!("data/GGGRX_1200A_SHA_head.TAB");

#[test]
fn a_gfc_file_reads_back_exactly() {
    let gfc = read_gfc(GFC, None).unwrap();
    assert_eq!(gfc.name, "fixture-degree-3");
    assert_eq!(gfc.errors, "formal");
    assert_eq!(gfc.header_degree, 3);
    assert!(!gfc.central_term_implied);
    let m = &gfc.model;
    assert_eq!(m.gm_m3ps2(), 0.398_600_441_5e15);
    assert_eq!(m.radius_m(), 6_378_136.3);
    assert_eq!(m.tide_system(), TideSystem::ZeroTide);
    let c = m.coefficients();
    assert_eq!(c.degree(), 3);
    assert_eq!(c.cosine()[triangular_index(0, 0)], 1.0);
    assert_eq!(c.cosine()[triangular_index(2, 0)], -0.484_165_371_736e-3);
    assert_eq!(c.sine()[triangular_index(2, 2)], -0.140_016_683_654e-5);
    assert_eq!(c.sine()[triangular_index(3, 3)], 0.141_435_626_958e-5);
    assert_eq!(c.cosine()[triangular_index(1, 1)], 0.0);

    let capped = read_gfc(GFC, Some(2)).unwrap();
    assert_eq!(capped.model.coefficients().degree(), 2);
    assert_eq!(&c.cosine()[..6], capped.model.coefficients().cosine());
}

#[test]
fn gfc_refusals_name_their_cause() {
    let kind = |text: &str| read_gfc(text, None).unwrap_err().kind;
    assert_eq!(
        kind(&GFC.replace("radius ", "radios ")),
        ParseErrorKind::MissingHeader("radius")
    );
    assert_eq!(
        kind(&GFC.replace("fully_normalized", "unnormalized")),
        ParseErrorKind::UnsupportedNormalization
    );
    let time_variable = GFC.replace("gfc     3   3", "gfct    3   3");
    let err = read_gfc(&time_variable, None).unwrap_err();
    assert_eq!(err.kind, ParseErrorKind::UnsupportedKey);
    assert_eq!(err.line, GFC.lines().count());
    assert_eq!(
        kind(&GFC.replace("0.957254173792e-06", "0.95x")),
        ParseErrorKind::BadNumber
    );
    assert_eq!(
        kind(&GFC.replace("end_of_head", "end_of_hat")),
        ParseErrorKind::MissingHeader("end_of_head")
    );
}

#[test]
fn a_gfc_without_a_degree_zero_row_gets_the_central_term() {
    let headless: String = GFC
        .lines()
        .filter(|l| !l.starts_with("gfc     0   0"))
        .map(|l| format!("{l}\n"))
        .collect();
    let gfc = read_gfc(&headless, None).unwrap();
    assert!(gfc.central_term_implied);
    assert_eq!(gfc.model.coefficients().cosine()[0], 1.0);
}

#[test]
fn a_cof_file_reads_back_as_converted_gauss_coefficients() {
    let cof = read_cof(COF, 6_371_200.0).unwrap();
    assert_eq!(
        (cof.name.as_str(), cof.release.as_str()),
        ("FIXTURE-2020", "01/01/2020")
    );
    let m = &cof.model;
    assert_eq!(m.epoch(), DecimalYear(2020.0));
    assert_eq!(m.validity().until, DecimalYear(2025.0));
    assert_eq!(m.main().degree(), 2);
    let i = triangular_index(2, 1);
    assert_eq!(m.main().cosine()[i], schmidt_to_geodesy(2, 3_000.0));
    assert_eq!(m.main().sine()[i], schmidt_to_geodesy(2, -3_000.0));
    assert_eq!(m.secular().sine()[i], schmidt_to_geodesy(2, -25.0));
    assert_eq!(m.main().cosine()[0], 0.0);
}

#[test]
fn a_degree_one_cof_is_the_dipole_built_directly() {
    let dipole_cof =
        "2020.0 DIPOLE 01/01/2020\n1 0 -29000.0 0.0 0.0 0.0\n1 1 -1500.0 4600.0 0.0 0.0\n9999\n";
    let read = read_cof(dipole_cof, 6_371_200.0).unwrap().model;
    let window = read.validity();
    let direct = MagneticModel::dipole(
        6_371_200.0,
        [-29_000.0, -1_500.0, 4_600.0],
        DecimalYear(2020.0),
        window,
    )
    .unwrap();
    let mut buffer = WorkspaceBuffer::new(1);
    let mut ws = buffer.workspace().unwrap();
    let p = [4.0e6, 3.0e6, -4.5e6];
    let at = DecimalYear(2021.5);
    assert_eq!(
        read.sample(&mut ws, p, at, Derivatives::Hessian).unwrap(),
        direct.sample(&mut ws, p, at, Derivatives::Hessian).unwrap()
    );
}

#[test]
fn schmidt_rows_and_a_cof_text_give_the_same_model() {
    let rows: Vec<GaussRow> = COF
        .lines()
        .skip(1)
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .take_while(|f| f.len() == 6)
        .map(|f| GaussRow {
            n: f[0].parse().unwrap(),
            m: f[1].parse().unwrap(),
            g: f[2].parse().unwrap(),
            h: f[3].parse().unwrap(),
            g_dot: f[4].parse().unwrap(),
            h_dot: f[5].parse().unwrap(),
        })
        .collect();
    let window = ValidityWindow {
        from: DecimalYear(2020.0),
        until: DecimalYear(2025.0),
    };
    let built =
        MagneticModel::from_schmidt(6_371_200.0, DecimalYear(2020.0), window, &rows).unwrap();
    assert_eq!(built, read_cof(COF, 6_371_200.0).unwrap().model);
}

/// A value as the file writes it, parsed the way the reader must.
#[cfg(test)]
fn value(text: &str) -> f64 {
    text.parse().unwrap()
}

#[test]
fn a_shadr_table_reads_in_si_units_with_the_central_term() {
    let shadr = read_shadr(SHADR, Some(3)).unwrap();
    assert_eq!(shadr.header_degree, 1200);
    let m = &shadr.model;
    assert_eq!(m.radius_m(), 1_738_000.0);
    assert_eq!(m.gm_m3ps2(), value("4.9028001224453001e+03") * 1e9);
    assert_eq!(m.tide_system(), TideSystem::Unstated);
    let c = m.coefficients();
    assert_eq!(c.degree(), 3);
    assert_eq!(c.cosine()[0], 1.0);
    assert_eq!(
        c.cosine()[triangular_index(2, 0)],
        value("-9.0884339347424299e-05")
    );
    assert_eq!(
        c.sine()[triangular_index(2, 1)],
        value("1.1732764234889199e-09")
    );
    assert_eq!(
        c.cosine()[triangular_index(3, 0)],
        value("-3.1973308084610398e-06")
    );
    // Rows beyond the excerpt are absent, so zero.
    assert_eq!(c.cosine()[triangular_index(3, 1)], 0.0);
}

#[test]
fn shadr_refusals_name_their_cause() {
    let unnormalised = SHADR.replacen(" 1200,    1,", " 1200,    0,", 1);
    assert_eq!(
        read_shadr(&unnormalised, Some(3)).unwrap_err().kind,
        ParseErrorKind::UnsupportedNormalization
    );
    let first = SHADR.lines().next().unwrap();
    let rotated_first = format!(
        "{}, 1.0e+01, 0.0e+00",
        first.rsplitn(3, ',').last().unwrap()
    );
    let rotated = SHADR.replacen(first, &rotated_first, 1);
    assert_eq!(
        read_shadr(&rotated, Some(3)).unwrap_err().kind,
        ParseErrorKind::UnsupportedReference
    );
}
