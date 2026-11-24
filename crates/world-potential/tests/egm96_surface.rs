//! EGM96 to degree 36 at the Earth's surface against the WGS-84 normal
//! field: the difference is the gravity disturbance and deflection of the
//! vertical, which have known physical bounds.

#![cfg(feature = "alloc")]

use world_core::{GeodeticPosition, WGS84, normal_gravity};
use world_potential::{Derivatives, WorkspaceBuffer, egm96_degree_36};

/// WGS-84 Earth rotation rate, rad/s (NIMA TR8350.2 table 3.1).
const OMEGA: f64 = 7.292_115e-5;
/// The Earth's gravity disturbance stays within ±300 mGal even at full
/// resolution; a degree-36 field is smoother still.
const DISTURBANCE_MPS2: f64 = 3e-3;
/// Deflections of the vertical are under one arcminute almost everywhere.
const DEFLECTION_RAD: f64 = 60.0 / 206_264.8;

#[test]
fn gravity_differs_from_normal_gravity_by_a_physical_disturbance() {
    let model = egm96_degree_36().unwrap();
    let mut buffer = WorkspaceBuffer::new(36);
    let mut ws = buffer.workspace().unwrap();
    for lat in (-90..=90).step_by(15) {
        for lon in (-180..180).step_by(30) {
            let (lat, lon) = (f64::from(lat), f64::from(lon));
            let e = WGS84.to_ecef(lat, lon, 0.0);
            let g = model
                .gravitation(&mut ws, [e.x_m, e.y_m, e.z_m], Derivatives::Hessian)
                .unwrap();
            let a = g.acceleration_mps2;
            let gravity = [
                a[0] + OMEGA * OMEGA * e.x_m,
                a[1] + OMEGA * OMEGA * e.y_m,
                a[2],
            ];
            let magnitude = gravity.iter().map(|v| v * v).sum::<f64>().sqrt();
            let normal = normal_gravity(GeodeticPosition::wgs84(lat, lon, 0.0)).unwrap();
            assert!(
                (magnitude - normal).abs() < DISTURBANCE_MPS2,
                "{lat} {lon}: {magnitude} vs {normal}"
            );
            let (la, lo) = (lat.to_radians(), lon.to_radians());
            let down = [-la.cos() * lo.cos(), -la.cos() * lo.sin(), -la.sin()];
            let cos_angle = gravity.iter().zip(down).map(|(g, d)| g * d).sum::<f64>() / magnitude;
            assert!(cos_angle.min(1.0).acos() < DEFLECTION_RAD, "{lat} {lon}");
            // A truncated series is harmonic everywhere, so its trace is
            // rounding at any radius.
            let t = g.gradient_per_s2.unwrap();
            assert!(t.trace().abs() < 1e-12 * t.zz.abs());
        }
    }
}
