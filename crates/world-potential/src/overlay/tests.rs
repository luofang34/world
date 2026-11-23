use std::vec::Vec;

use super::*;
use crate::workspace_len;
use world_core::WGS84;

/// A 3 × 3 grid at 1° spacing from (10° N, 20° E) whose down component is
/// the node's row + 10 × column, north and east zero.
fn grid(outside: OutsideCoverage) -> AnomalyGrid<Vec<f64>> {
    let geometry = GridGeometry {
        south_deg: 10.0,
        west_deg: 20.0,
        latitude_step_deg: 1.0,
        longitude_step_deg: 1.0,
        rows: 3,
        columns: 3,
    };
    let mut values = Vec::new();
    for row in 0..3 {
        for col in 0..3 {
            values.extend([0.0, 0.0, row as f64 + 10.0 * col as f64]);
        }
    }
    AnomalyGrid::new(geometry, values, WGS84, outside).unwrap()
}

#[test]
fn bilinear_interpolation_is_exact_at_nodes_and_linear_between() {
    let g = grid(OutsideCoverage::Zero);
    assert_eq!(g.anomaly_ned(10.0, 20.0), Some([0.0, 0.0, 0.0]));
    assert_eq!(g.anomaly_ned(12.0, 22.0), Some([0.0, 0.0, 22.0]));
    // Bilinear weights of exact binary fractions: rounding only.
    let mid = g.anomaly_ned(10.5, 21.25).unwrap();
    assert!((mid[2] - 13.0).abs() < 1e-12);
    // Longitudes wrap: 380° is 20°.
    assert_eq!(g.anomaly_ned(11.0, 380.0), Some([0.0, 0.0, 1.0]));
    assert_eq!(g.anomaly_ned(9.99, 21.0), None);
    assert_eq!(g.anomaly_ned(11.0, 22.01), None);
}

#[test]
fn a_down_anomaly_points_toward_the_centre_in_body_axes() {
    let g = grid(OutsideCoverage::Zero);
    let mut buffer = std::vec![0.0; workspace_len(0)];
    let mut ws = Workspace::new(&mut buffer, 0).unwrap();
    let e = WGS84.to_ecef(11.0, 22.0, 0.0);
    let s = g
        .sample(
            &mut ws,
            [e.x_m, e.y_m, e.z_m],
            DecimalYear(2026.0),
            Derivatives::Hessian,
        )
        .unwrap();
    let (lat, lon) = (11.0_f64.to_radians(), 22.0_f64.to_radians());
    let down = [-lat.cos() * lon.cos(), -lat.cos() * lon.sin(), -lat.sin()];
    // The ECEF round trip moves the point by nanodegrees, so the
    // interpolated 21.0 and the rotation agree to 1e-9 absolute.
    for i in 0..3 {
        assert!(
            (s.vector[i] - 21.0 * down[i]).abs() < 1e-9,
            "{:?}",
            s.vector
        );
    }
    assert_eq!(s.gradient, None);
}

#[test]
fn off_the_grid_is_zero_or_refused_as_configured() {
    let mut buffer = std::vec![0.0; workspace_len(0)];
    let mut ws = Workspace::new(&mut buffer, 0).unwrap();
    let e = WGS84.to_ecef(-40.0, 100.0, 1_000.0);
    let p = [e.x_m, e.y_m, e.z_m];
    let at = DecimalYear(2026.0);
    let zero = grid(OutsideCoverage::Zero)
        .sample(&mut ws, p, at, Derivatives::Gradient)
        .unwrap();
    assert_eq!(zero.vector, [0.0; 3]);
    assert_eq!(
        grid(OutsideCoverage::Refuse).sample(&mut ws, p, at, Derivatives::Gradient),
        Err(FieldError::OutsideCoverage)
    );
}

#[test]
fn a_spherical_body_uses_geocentric_latitude() {
    let moon = Ellipsoid {
        semi_major_m: 1_737_400.0,
        flattening: 0.0,
    };
    let geometry = GridGeometry {
        south_deg: -1.0,
        west_deg: -1.0,
        latitude_step_deg: 2.0,
        longitude_step_deg: 2.0,
        rows: 2,
        columns: 2,
    };
    let g = AnomalyGrid::new(
        geometry,
        [0.0, 0.0, 5.0].repeat(4),
        moon,
        OutsideCoverage::Refuse,
    )
    .unwrap();
    let mut buffer = std::vec![0.0; workspace_len(0)];
    let mut ws = Workspace::new(&mut buffer, 0).unwrap();
    let s = g
        .sample(
            &mut ws,
            [1_737_400.0 + 100.0, 0.0, 0.0],
            DecimalYear(2026.0),
            Derivatives::Gradient,
        )
        .unwrap();
    // A uniform grid on the equator: the rotation is exact to rounding.
    assert!((s.vector[0] + 5.0).abs() < 1e-12 && s.vector[1].abs() < 1e-12);
}

#[test]
fn malformed_grids_are_refused() {
    let mut geometry = grid(OutsideCoverage::Zero).geometry();
    assert_eq!(
        AnomalyGrid::new(geometry, [0.0; 26], WGS84, OutsideCoverage::Zero),
        Err(FieldError::InvalidModel)
    );
    geometry.latitude_step_deg = 0.0;
    assert_eq!(
        AnomalyGrid::new(geometry, [0.0; 27], WGS84, OutsideCoverage::Zero),
        Err(FieldError::InvalidModel)
    );
}
