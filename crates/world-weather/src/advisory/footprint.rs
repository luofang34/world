//! Point tests against an advisory's horizontal footprint.

use airmass_core::{AdvisoryLine, AdvisoryPolygon, AdvisoryPosition};
use libm::{cos, hypot};
use world_core::GeodeticPosition;

/// Mean Earth radius in metres (IUGG), for distances across a footprint.
const EARTH_MEAN_RADIUS_M: f64 = 6_371_008.8;

/// A vertex in degrees, its longitude taken the short way round from
/// `point` so a footprint across the antimeridian stays in one piece.
fn degrees_near(vertex: AdvisoryPosition, point: GeodeticPosition) -> (f64, f64) {
    let latitude = f64::from(vertex.latitude_e6()) * 1e-6;
    let mut delta = f64::from(vertex.longitude_e6()) * 1e-6 - point.longitude_deg;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    (latitude, delta)
}

/// Even-odd test over all rings, so a ring inside another is a hole.
pub(super) fn inside_polygon(polygon: &AdvisoryPolygon, point: GeodeticPosition) -> bool {
    let mut inside = false;
    for ring in polygon.rings() {
        for edge in ring.positions().windows(2) {
            let [from, to] = edge else { continue };
            let (lat_a, lon_a) = degrees_near(*from, point);
            let (lat_b, lon_b) = degrees_near(*to, point);
            let (north_a, north_b) = (lat_a - point.latitude_deg, lat_b - point.latitude_deg);
            if (north_a > 0.0) != (north_b > 0.0) {
                let east_at_crossing = lon_a - north_a * (lon_b - lon_a) / (north_b - north_a);
                if east_at_crossing >= 0.0 {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// Shortest distance from `point` to any segment of `line`, meters.
pub(super) fn distance_to_line_m(line: &AdvisoryLine, point: GeodeticPosition) -> Option<f64> {
    let east_scale = cos(point.latitude_deg.to_radians());
    let local = |vertex: AdvisoryPosition| {
        let (latitude, east_deg) = degrees_near(vertex, point);
        (
            east_deg.to_radians() * east_scale * EARTH_MEAN_RADIUS_M,
            (latitude - point.latitude_deg).to_radians() * EARTH_MEAN_RADIUS_M,
        )
    };
    line.positions()
        .windows(2)
        .filter_map(|segment| {
            let [from, to] = segment else { return None };
            let (ax, ay) = local(*from);
            let (bx, by) = local(*to);
            let (dx, dy) = (bx - ax, by - ay);
            let length_sq = dx * dx + dy * dy;
            let along = if length_sq > 0.0 {
                (-(ax * dx + ay * dy) / length_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            Some(hypot(ax + dx * along, ay + dy * along))
        })
        .min_by(f64::total_cmp)
}
