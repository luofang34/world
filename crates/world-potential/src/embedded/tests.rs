use sha2::{Digest, Sha256};
use std::format;
use std::string::String;

use super::*;

/// SHA-256 of the committed EGM96 excerpt, so an edit to it is caught.
const EGM96_EXCERPT_SHA256: &str =
    "886e1c6c00909e00a0f3b684131d4b5a9215c2facca7d38a40cd98a6e4fa872c";

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn the_committed_files_are_the_recorded_ones() {
    assert_eq!(sha256(IGRF14_TEXT), IGRF14.source_sha256);
    assert_eq!(sha256(EGM96_TEXT), EGM96_EXCERPT_SHA256);
}

#[test]
fn the_egm96_excerpt_is_every_row_to_degree_36_and_no_more() {
    let rows = EGM96_TEXT.lines().filter(|l| l.starts_with("gfc")).count();
    // Degree 0 plus degrees 2..=36; ICGEM's EGM96 has no degree-1 rows.
    assert_eq!(rows, 1 + (2..=36).map(|n| n + 1).sum::<usize>());
    let model = egm96_degree_36().unwrap();
    assert_eq!(model.coefficients().degree(), 36);
    assert_eq!(model.gm_m3ps2(), 0.398_600_441_5e15);
    assert_eq!(model.radius_m(), 0.637_813_630_0e7);
    assert_eq!(model.tide_system(), crate::TideSystem::TideFree);
    assert_eq!(model.coefficients().cosine()[3], -0.484_165_371_736e-3);
    assert_eq!(model.provenance(), Some(EGM96_DEGREE_36));
}

#[test]
fn igrf14_spans_1900_to_2030_in_five_year_segments() {
    let series = igrf14().unwrap();
    assert_eq!(series.segments().len(), 26);
    let w = series.validity().unwrap();
    assert_eq!((w.from.value(), w.until.value()), (1900.0, 2030.0));
    let last = series.segments().last().unwrap();
    assert_eq!(last.main().degree(), 13);
    // g10 of 2025.0 is -29350.0 nT, stored as g/√3.
    let g10 = last.main().cosine()[1] * 3.0_f64.sqrt();
    // Values are stored after one division by √3; 1e-9 nT is rounding.
    assert!((g10 + 29_350.0).abs() < 1e-9);
    // The 2025-30 column: g10 changes by +12.6 nT/year.
    let g10_rate = last.secular().cosine()[1] * 3.0_f64.sqrt();
    assert!((g10_rate - 12.6).abs() < 1e-12);
    // Between epochs the rate is the difference over five years:
    // g10(1905) - g10(1900) = -31464 - (-31543) nT.
    let first = &series.segments()[0];
    let rate_1900 = first.secular().cosine()[1] * 3.0_f64.sqrt();
    assert!((rate_1900 - 79.0 / 5.0).abs() < 1e-9);
    assert_eq!(series.provenance(), Some(IGRF14));
}
