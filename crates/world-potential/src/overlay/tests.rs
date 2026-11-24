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

/// Two rows of four columns 90° apart from 180° W, whose value in column
/// `k` is `10 k`: the span is a full circle, so column 3 and column 0 are
/// neighbours across the ±180° seam.
fn global_geometry() -> GridGeometry {
    GridGeometry {
        south_deg: 0.0,
        west_deg: -180.0,
        latitude_step_deg: 10.0,
        longitude_step_deg: 90.0,
        rows: 2,
        columns: 4,
    }
}

fn column_values(per_node: usize) -> Vec<f64> {
    let mut values = Vec::new();
    for _row in 0..2 {
        for col in 0..4 {
            values.extend((0..per_node).map(|k| 10.0 * col as f64 + 100.0 * k as f64));
        }
    }
    values
}

#[test]
fn a_grid_spanning_360_degrees_interpolates_across_the_seam() {
    let g = AnomalyGrid::new(
        global_geometry(),
        column_values(3),
        WGS84,
        OutsideCoverage::Refuse,
    )
    .unwrap();
    assert!(g.is_global_longitude());
    // Halfway from column 3 (east of 90°E, value 30) round to column 0
    // (value 0) is 135°E, and the same point reads as -225°.
    for lon in [135.0, -225.0] {
        let v = g.anomaly_ned(5.0, lon).unwrap();
        assert!((v[0] - 15.0).abs() < 1e-12, "{lon}: {v:?}");
        assert!((v[1] - 115.0).abs() < 1e-12 && (v[2] - 215.0).abs() < 1e-12);
    }
    // 1° either side of the seam differs by the two cells' slopes only.
    let (west, east) = (g.anomaly_ned(5.0, 179.0), g.anomaly_ned(5.0, -179.0));
    assert!((west.unwrap()[0] - 30.0 / 90.0).abs() < 1e-12);
    assert!((east.unwrap()[0] - 10.0 / 90.0).abs() < 1e-12);
    // Latitude still has edges.
    assert_eq!(g.anomaly_ned(10.5, 135.0), None);
}

#[test]
fn a_grid_short_of_a_full_circle_does_not_wrap() {
    let mut geometry = global_geometry();
    geometry.longitude_step_deg = 89.0;
    let g = AnomalyGrid::new(geometry, column_values(3), WGS84, OutsideCoverage::Zero).unwrap();
    assert!(!g.is_global_longitude());
    // Past the last column (267° east of the west edge) is off the grid.
    assert_eq!(g.anomaly_ned(5.0, 90.5), None);
}

#[test]
fn the_accessors_return_what_the_grid_was_built_from() {
    let values = column_values(3);
    let g = AnomalyGrid::new(
        global_geometry(),
        values.as_slice(),
        WGS84,
        OutsideCoverage::Zero,
    )
    .unwrap();
    let geometry = g.geometry();
    assert_eq!(geometry.origin_deg(), (0.0, -180.0));
    assert_eq!(geometry.spacing_deg(), (10.0, 90.0));
    assert_eq!(geometry.dimensions(), (2, 4));
    assert_eq!(g.values(), values.as_slice());
    assert_eq!(g.outside(), OutsideCoverage::Zero);
    assert_eq!(g.shape(), WGS84);
}

#[test]
fn a_scalar_grid_shares_the_interpolation_and_the_seam() {
    let g = ScalarAnomalyGrid::new(
        global_geometry(),
        column_values(1),
        WGS84,
        OutsideCoverage::Refuse,
    )
    .unwrap();
    assert!(g.is_global_longitude());
    assert!((g.anomaly(5.0, 135.0).unwrap() - 15.0).abs() < 1e-12);
    // 20° is 200° east of the west edge: 20/90 of the way from column 2.
    assert!((g.anomaly(5.0, 20.0).unwrap() - (20.0 + 10.0 * 20.0 / 90.0)).abs() < 1e-9);
    // The vector grid's first component takes the same path.
    let vector = AnomalyGrid::new(
        global_geometry(),
        column_values(3),
        WGS84,
        OutsideCoverage::Refuse,
    )
    .unwrap();
    for lon in [-170.0, -40.0, 0.0, 91.0, 179.5] {
        assert_eq!(
            g.anomaly(5.0, lon),
            vector.anomaly_ned(5.0, lon).map(|v| v[0])
        );
    }
    assert_eq!(g.values().len(), 8);
}

#[test]
fn a_scalar_grid_samples_a_position_and_keeps_the_off_grid_policy() {
    let on = WGS84.to_ecef(5.0, 20.0, 0.0);
    let off = WGS84.to_ecef(50.0, 20.0, 0.0);
    for (outside, expected) in [
        (OutsideCoverage::Zero, Ok(0.0)),
        (OutsideCoverage::Refuse, Err(FieldError::OutsideCoverage)),
    ] {
        let g =
            ScalarAnomalyGrid::new(global_geometry(), column_values(1), WGS84, outside).unwrap();
        let v = g.sample([on.x_m, on.y_m, on.z_m]).unwrap();
        assert!((v - g.anomaly(5.0, 20.0).unwrap()).abs() < 1e-6);
        assert_eq!(g.sample([off.x_m, off.y_m, off.z_m]), expected);
        assert_eq!(
            g.sample([f64::NAN, 0.0, 0.0]),
            Err(FieldError::InvalidPosition)
        );
    }
    let wrong = ScalarAnomalyGrid::new(
        global_geometry(),
        column_values(3),
        WGS84,
        OutsideCoverage::Zero,
    );
    assert_eq!(wrong.err(), Some(FieldError::InvalidModel));
}
