//! Horizontal distance between two geodetic positions.

use libm::hypot;
use world_core::{GeodeticPosition, LocalTangentPlane};

/// Tangent-plane horizontal distance in meters between `a` and `b` on the same datum.
pub fn horizontal_distance_m(a: GeodeticPosition, b: GeodeticPosition) -> Option<f64> {
    let origin = GeodeticPosition {
        ellipsoidal_height_m: 0.0,
        ..a
    };
    let target = GeodeticPosition {
        ellipsoidal_height_m: 0.0,
        ..b
    };
    let plane = LocalTangentPlane::new(origin).ok()?;
    let ned = plane.to_ned(target).ok()?;
    Some(hypot(ned.north_m, ned.east_m))
}
