use std::vec::Vec;

use world_core::{DatasetId, Provenance, WGS84};

use super::*;
use crate::{
    AnomalyGrid, GravityModel, GridGeometry, MagneticModel, OutsideCoverage, ScalarAnomalyGrid,
    TideSystem, triangular_len,
};

fn bytes(model: &dyn CanonicalContent) -> Vec<u8> {
    let mut out = Vec::new();
    model.visit_content(&mut |chunk| out.extend_from_slice(chunk));
    out
}

fn provenance(file: &'static str) -> Provenance {
    Provenance {
        dataset: DatasetId {
            name: "test",
            edition: "1",
        },
        publisher: "nobody",
        source_url: "https://example.invalid",
        source_file: file,
        source_sha256: "00",
        terms: "none",
    }
}

fn triangle(degree: usize, bump: f64) -> Vec<f64> {
    let mut v: Vec<f64> = (0..triangular_len(degree))
        .map(|i| i as f64 * 0.5)
        .collect();
    if let Some(last) = v.last_mut() {
        *last += bump;
    }
    v
}

fn gravity_vec(bump: f64) -> GravityModel<Vec<f64>> {
    let c = HarmonicCoefficients::new(3, triangle(3, bump), triangle(3, 0.0)).unwrap();
    GravityModel::new(3.986e14, 6.378e6, c, TideSystem::TideFree).unwrap()
}

fn gravity_array(bump: f64) -> GravityModel<[f64; 10]> {
    let take = |v: Vec<f64>| <[f64; 10]>::try_from(v).unwrap();
    let c = HarmonicCoefficients::new(3, take(triangle(3, bump)), take(triangle(3, 0.0))).unwrap();
    GravityModel::new(3.986e14, 6.378e6, c, TideSystem::TideFree).unwrap()
}

fn magnetic(bump: f64) -> MagneticModel<Vec<f64>> {
    let window = ValidityWindow {
        from: DecimalYear(2020.0),
        until: DecimalYear(2025.0),
    };
    let main = HarmonicCoefficients::new(2, triangle(2, bump), triangle(2, 0.0)).unwrap();
    let secular = HarmonicCoefficients::new(2, triangle(2, 0.0), triangle(2, 0.0)).unwrap();
    MagneticModel::new(6.3712e6, DecimalYear(2020.0), window, main, secular).unwrap()
}

fn grid_geometry() -> GridGeometry {
    GridGeometry {
        south_deg: 0.0,
        west_deg: 0.0,
        latitude_step_deg: 1.0,
        longitude_step_deg: 1.0,
        rows: 2,
        columns: 2,
    }
}

#[test]
fn identical_models_feed_identical_bytes_and_one_coefficient_changes_them() {
    assert_eq!(bytes(&gravity_vec(0.0)), bytes(&gravity_vec(0.0)));
    assert_ne!(bytes(&gravity_vec(0.0)), bytes(&gravity_vec(1e-12)));
    assert_eq!(bytes(&magnetic(0.0)), bytes(&magnetic(0.0)));
    assert_ne!(bytes(&magnetic(0.0)), bytes(&magnetic(1e-12)));
    #[cfg(feature = "alloc")]
    {
        let series = |bump| crate::MagneticSeries::new(std::vec![magnetic(bump), magnetic(0.0)]);
        assert_eq!(bytes(&series(0.0)), bytes(&series(0.0)));
        assert_ne!(bytes(&series(0.0)), bytes(&series(1e-12)));
    }
    let grid = |v: f64| {
        AnomalyGrid::new(
            grid_geometry(),
            std::vec![0.0, 0.0, 0.0, 0.0, 0.0, v, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            WGS84,
            OutsideCoverage::Zero,
        )
        .unwrap()
    };
    assert_eq!(bytes(&grid(1.0)), bytes(&grid(1.0)));
    assert_ne!(bytes(&grid(1.0)), bytes(&grid(1.0 + f64::EPSILON)));
}

#[test]
fn everything_that_changes_a_sample_changes_the_bytes() {
    let base = bytes(&gravity_vec(0.0));
    let tide = GravityModel::new(
        3.986e14,
        6.378e6,
        HarmonicCoefficients::new(3, triangle(3, 0.0), triangle(3, 0.0)).unwrap(),
        TideSystem::ZeroTide,
    )
    .unwrap();
    assert_ne!(base, bytes(&tide));
    let truncated = gravity_vec(0.0)
        .with_truncation(Truncation {
            min_degree: 2,
            degree: 3,
            order: 3,
        })
        .unwrap();
    assert_ne!(base, bytes(&truncated));
    assert_ne!(
        bytes(&gravity_vec(0.0).with_provenance(provenance("a"))),
        bytes(&gravity_vec(0.0).with_provenance(provenance("b")))
    );
    assert_ne!(
        base,
        bytes(&gravity_vec(0.0).with_provenance(provenance("a")))
    );
    // Epoch and validity are content of a magnetic model.
    let moved = MagneticModel::new(
        6.3712e6,
        DecimalYear(2021.0),
        ValidityWindow {
            from: DecimalYear(2020.0),
            until: DecimalYear(2025.0),
        },
        HarmonicCoefficients::new(2, triangle(2, 0.0), triangle(2, 0.0)).unwrap(),
        HarmonicCoefficients::new(2, triangle(2, 0.0), triangle(2, 0.0)).unwrap(),
    )
    .unwrap();
    assert_ne!(bytes(&magnetic(0.0)), bytes(&moved));
    let outside =
        |o| bytes(&AnomalyGrid::new(grid_geometry(), std::vec![0.0; 12], WGS84, o).unwrap());
    assert_ne!(
        outside(OutsideCoverage::Zero),
        outside(OutsideCoverage::Refuse)
    );
}

#[test]
fn the_feed_does_not_depend_on_the_storage_type() {
    assert_eq!(bytes(&gravity_vec(0.5)), bytes(&gravity_array(0.5)));
    let values = std::vec![1.0; 12];
    let from_vec = AnomalyGrid::new(
        grid_geometry(),
        values.clone(),
        WGS84,
        OutsideCoverage::Zero,
    )
    .unwrap();
    let from_slice = AnomalyGrid::new(
        grid_geometry(),
        values.as_slice(),
        WGS84,
        OutsideCoverage::Zero,
    )
    .unwrap();
    let from_array =
        AnomalyGrid::new(grid_geometry(), [1.0; 12], WGS84, OutsideCoverage::Zero).unwrap();
    assert_eq!(bytes(&from_vec), bytes(&from_slice));
    assert_eq!(bytes(&from_vec), bytes(&from_array));
}

#[test]
fn model_kinds_never_share_a_feed_and_chunking_is_not_content() {
    let vector = AnomalyGrid::new(
        grid_geometry(),
        std::vec![0.0; 12],
        WGS84,
        OutsideCoverage::Zero,
    )
    .unwrap();
    let scalar = ScalarAnomalyGrid::new(
        grid_geometry(),
        std::vec![0.0; 4],
        WGS84,
        OutsideCoverage::Zero,
    )
    .unwrap();
    assert_ne!(bytes(&vector)[0], bytes(&scalar)[0]);
    // A run longer than the batching buffer still concatenates to the
    // length prefix and the little-endian bits, in order.
    let long: Vec<f64> = (0..100).map(f64::from).collect();
    let mut out = Vec::new();
    feed_f64s(&mut |c| out.extend_from_slice(c), &long);
    let expected: Vec<u8> = 100_u64
        .to_le_bytes()
        .into_iter()
        .chain(long.iter().flat_map(|v| v.to_bits().to_le_bytes()))
        .collect();
    assert_eq!(out, expected);
}
