//! IGRF-14 against values published by NOAA NCEI's geomagnetic
//! calculator.
//!
//! The values are rows of the precomputed grids in the IAGA-VMOD `ppigrf`
//! repository (github.com/IAGA-VMOD/ppigrf, commit 5c45795b,
//! `test/data/dgrf-2010-01-01/b_{n,e,z}.csv`, sha256 98d72e3a…, 255fa3ab…,
//! 50d893b4…), which that repository's README records as NCEI calculator
//! output for 2010-01-01 at 5 km above the WGS-84 ellipsoid. 2010.0 is a
//! definitive (DGRF) epoch, unchanged between IGRF-13, which produced the
//! grid, and IGRF-14.

#![cfg(feature = "alloc")]

use world_core::{DecimalYear, WGS84};
use world_potential::{Derivatives, VectorField, WorkspaceBuffer, igrf14};

/// The calculator prints one decimal; 0.1 nT covers that rounding.
const TOLERANCE_NT: f64 = 0.1;

/// (latitude, longitude, north X, east Y, down Z), nT.
const NCEI_2010: [(f64, f64, f64, f64, f64); 6] = [
    (80.0, 100.0, 2_049.2, 1_300.9, 58_271.4),
    (45.0, 10.0, 22_667.0, 663.6, 41_121.2),
    (30.0, 150.0, 30_836.8, -1_509.7, 26_322.2),
    (0.0, 0.0, 27_462.3, -2_940.1, -15_388.1),
    (-60.0, -120.0, 16_078.2, 12_982.6, -45_431.7),
    (-80.0, -30.0, 19_236.6, 425.5, -42_951.2),
];

#[test]
fn igrf14_reproduces_the_ncei_calculator_at_the_2010_epoch() {
    let igrf = igrf14().unwrap();
    let mut buffer = WorkspaceBuffer::new(igrf.workspace_degree());
    let mut ws = buffer.workspace().unwrap();
    for (lat, lon, x, y, z) in NCEI_2010 {
        let e = WGS84.to_ecef(lat, lon, 5_000.0);
        let b = igrf
            .sample(
                &mut ws,
                [e.x_m, e.y_m, e.z_m],
                DecimalYear(2010.0),
                Derivatives::Gradient,
            )
            .unwrap()
            .vector;
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        let (sl, cl, so, co) = (la.sin(), la.cos(), lo.sin(), lo.cos());
        let dot = |a: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let ned = [
            dot([-sl * co, -sl * so, cl]),
            dot([-so, co, 0.0]),
            dot([-cl * co, -cl * so, -sl]),
        ];
        for (got, want) in ned.into_iter().zip([x, y, z]) {
            assert!(
                (got - want).abs() <= TOLERANCE_NT,
                "{lat} {lon}: {ned:?} vs {x} {y} {z}"
            );
        }
    }
}

#[test]
fn igrf14_refuses_dates_outside_1900_to_2030() {
    let igrf = igrf14().unwrap();
    let mut buffer = WorkspaceBuffer::new(igrf.workspace_degree());
    let mut ws = buffer.workspace().unwrap();
    for year in [1899.9, 2030.1] {
        assert!(
            igrf.sample(
                &mut ws,
                [6.4e6, 0.0, 0.0],
                DecimalYear(year),
                Derivatives::Gradient
            )
            .is_err()
        );
    }
}
