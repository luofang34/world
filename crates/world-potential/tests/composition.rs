//! A global model with a local anomaly overlay, held as `dyn`, through
//! the public API only.

#![cfg(feature = "alloc")]

use world_core::{DecimalYear, ValidityWindow, WGS84};
use world_potential::{
    AnomalyGrid, Derivatives, GridGeometry, MagneticModel, OutsideCoverage, Sum, VectorField,
    WorkspaceBuffer,
};

#[cfg(test)]
fn global() -> Box<dyn VectorField> {
    let window = ValidityWindow {
        from: DecimalYear(2025.0),
        until: DecimalYear(2030.0),
    };
    let dipole = MagneticModel::tilted_dipole(
        6_371_200.0,
        29_400.0,
        80.7,
        -72.7,
        DecimalYear(2025.0),
        window,
    );
    Box::new(dipole.unwrap())
}

/// A uniform 200 nT downward anomaly over 40–42° N, 10–12° E.
#[cfg(test)]
fn overlay() -> AnomalyGrid<Vec<f64>> {
    let geometry = GridGeometry {
        south_deg: 40.0,
        west_deg: 10.0,
        latitude_step_deg: 2.0,
        longitude_step_deg: 2.0,
        rows: 2,
        columns: 2,
    };
    AnomalyGrid::new(
        geometry,
        [0.0, 0.0, 200.0].repeat(4),
        WGS84,
        OutsideCoverage::Zero,
    )
    .unwrap()
}

#[test]
fn an_overlay_adds_on_its_grid_and_nothing_off_it() {
    let field: Box<dyn VectorField> = Box::new(Sum(global(), overlay()));
    let mut buffer = WorkspaceBuffer::new(field.workspace_degree());
    let mut ws = buffer.workspace().unwrap();
    let at = DecimalYear(2026.0);
    for (lat, lon, inside) in [(41.0_f64, 11.0_f64, true), (-20.0, 140.0, false)] {
        let e = WGS84.to_ecef(lat, lon, 1_000.0);
        let p = [e.x_m, e.y_m, e.z_m];
        let sum = field
            .sample(&mut ws, p, at, Derivatives::Gradient)
            .unwrap()
            .vector;
        let base = global()
            .sample(&mut ws, p, at, Derivatives::Gradient)
            .unwrap()
            .vector;
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        let down = [-la.cos() * lo.cos(), -la.cos() * lo.sin(), -la.sin()];
        let anomaly = if inside { 200.0 } else { 0.0 };
        for i in 0..3 {
            // The two parts add exactly; 1e-9 nT allows the overlay's ECEF
            // round trip.
            assert!(
                (sum[i] - base[i] - anomaly * down[i]).abs() < 1e-9,
                "{lat} {lon}"
            );
        }
    }
}

#[test]
fn a_sum_with_an_overlay_reports_no_gradient_rather_than_a_partial_one() {
    let field = Sum(global(), overlay());
    let mut buffer = WorkspaceBuffer::new(field.workspace_degree());
    let mut ws = buffer.workspace().unwrap();
    let e = WGS84.to_ecef(41.0, 11.0, 0.0);
    let s = field
        .sample(
            &mut ws,
            [e.x_m, e.y_m, e.z_m],
            DecimalYear(2026.0),
            Derivatives::Hessian,
        )
        .unwrap();
    assert_eq!(s.gradient, None);
    let alone = global()
        .sample(
            &mut ws,
            [e.x_m, e.y_m, e.z_m],
            DecimalYear(2026.0),
            Derivatives::Hessian,
        )
        .unwrap();
    assert!(alone.gradient.is_some());
}
