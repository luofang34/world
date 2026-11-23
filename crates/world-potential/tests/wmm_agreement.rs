//! The shared synthesis core, fed WMM2025's own coefficient file, against
//! `world_geomag::wmm2025`: two independent recursions (Cartesian solid
//! harmonics here, NOAA's spherical Legendre recursion there) on the same
//! coefficients.

#![cfg(feature = "alloc")]

use world_core::{DecimalYear, GeodeticPosition, WGS84};
use world_geomag::wmm2025;
use world_potential::{Derivatives, VectorField, WorkspaceBuffer, read_cof};

/// The committed WMM2025 coefficient file `world-geomag` is generated
/// from; its checksum is in `world_geomag::WMM2025`'s provenance.
const WMM_COF: &str = include_str!("../../world-geomag/data/WMM.COF");

/// Both syntheses are exact to rounding; 1e-9 of |B| (~50 µnT) leaves
/// room for the different geodetic-to-geocentric conversions.
const RELATIVE_TOLERANCE: f64 = 1e-9;

/// B in body-fixed axes as geodetic north, east, down.
fn to_ned(b: [f64; 3], lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    let (sl, cl, so, co) = (lat.sin(), lat.cos(), lon.sin(), lon.cos());
    let dot = |a: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    [
        dot([-sl * co, -sl * so, cl]),
        dot([-so, co, 0.0]),
        dot([-cl * co, -cl * so, -sl]),
    ]
}

#[test]
fn the_core_reproduces_wmm2025_including_at_and_near_the_poles() {
    let model = read_cof(WMM_COF, 6_371_200.0).unwrap().model;
    let mut buffer = WorkspaceBuffer::new(model.workspace_degree());
    let mut ws = buffer.workspace().unwrap();
    let latitudes = [
        -90.0, -89.9999, -60.0, -33.3, 0.0, 12.5, 45.0, 80.0, 89.9999, 90.0,
    ];
    let longitudes = [-179.9, -74.7, 0.0, 33.0, 120.0];
    let heights = [-1_000.0, 0.0, 10_000.0, 400_000.0, 850_000.0];
    let mut worst = (0.0f64, 0.0, 0.0, 0.0, 0.0);
    for year in [2025.0, 2027.3, 2030.0] {
        for &lat in &latitudes {
            for &lon in &longitudes {
                for &h in &heights {
                    let reference =
                        wmm2025(GeodeticPosition::wgs84(lat, lon, h), DecimalYear(year)).unwrap();
                    let e = WGS84.to_ecef(lat, lon, h);
                    let b = model
                        .sample(
                            &mut ws,
                            [e.x_m, e.y_m, e.z_m],
                            DecimalYear(year),
                            Derivatives::Gradient,
                        )
                        .unwrap()
                        .vector;
                    let [n, east, d] = to_ned(b, lat, lon);
                    let diff = ((n - reference.north_nt).powi(2)
                        + (east - reference.east_nt).powi(2)
                        + (d - reference.down_nt).powi(2))
                    .sqrt();
                    let relative = diff / reference.total_nt;
                    if relative > worst.0 {
                        worst = (relative, year, lat, lon, h);
                    }
                    assert!(
                        relative <= RELATIVE_TOLERANCE,
                        "{year} {lat} {lon} {h}: {relative:e} ({n}, {east}, {d}) vs {reference:?}"
                    );
                }
            }
        }
    }
    let (r, year, lat, lon, h) = worst;
    eprintln!("worst |ΔB|/|B| against wmm2025: {r:e} at {year} {lat}° {lon}° {h} m");
}
