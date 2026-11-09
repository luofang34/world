//! Lateral shapes and vertical altitude bands for operational airspace notices.

use alloc::vec::Vec;
use libm::hypot;
use world_core::{GeodeticPosition, LocalTangentPlane};

use crate::NoticeError;

const FEET_TO_METERS: f64 = 0.3048;

/// Vertical boundary reference of an operational airspace notice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoticeAltitude {
    /// Ground or water surface.
    Surface,
    /// Geometric height above mean sea level, meters.
    GeometricMsl(f64),
    /// Standard-pressure flight level in hundreds of feet (e.g. `180` = FL180).
    FlightLevel(u16),
}

impl NoticeAltitude {
    /// Approximate altitude above mean sea level in meters under standard pressure.
    pub fn approximate_msl_m(self) -> f64 {
        match self {
            Self::Surface => -500.0,
            Self::GeometricMsl(m) => m,
            Self::FlightLevel(fl) => f64::from(fl) * 100.0 * FEET_TO_METERS,
        }
    }
}

/// Inclusive vertical interval `[floor, ceiling]` of an airspace restriction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalBand {
    /// Lower vertical limit.
    pub floor: NoticeAltitude,
    /// Upper vertical limit.
    pub ceiling: NoticeAltitude,
}

impl VerticalBand {
    /// Whether `altitude_msl_m` falls inside `[floor, ceiling]`.
    pub fn contains_msl_m(&self, altitude_msl_m: f64) -> bool {
        let lo = self.floor.approximate_msl_m();
        let hi = self.ceiling.approximate_msl_m();
        altitude_msl_m.is_finite() && (lo..=hi).contains(&altitude_msl_m)
    }
}

/// Horizontal boundary of a TFR or ad-hoc restricted volume on the datum ellipsoid.
#[derive(Clone, Debug, PartialEq)]
pub enum LateralShape {
    /// Circular boundary around `center` with `radius_m` horizontal radius.
    Circle {
        /// Center point on WGS-84.
        center: GeodeticPosition,
        /// Horizontal radius in meters.
        radius_m: f64,
    },
    /// Closed polygon defined by ordered vertices.
    Polygon {
        /// Ordered boundary vertices (at least three).
        vertices: Vec<GeodeticPosition>,
    },
}

impl LateralShape {
    /// Validates coordinates and vertex count.
    ///
    /// # Errors
    ///
    /// Returns [`NoticeError::InvalidGeometry`] when malformed.
    pub fn validate(&self) -> Result<(), NoticeError> {
        match self {
            Self::Circle { center, radius_m } => {
                if !center.is_well_formed() || !radius_m.is_finite() || *radius_m <= 0.0 {
                    return Err(NoticeError::InvalidGeometry);
                }
                Ok(())
            }
            Self::Polygon { vertices } => {
                if vertices.len() < 3 || vertices.iter().any(|v| !v.is_well_formed()) {
                    return Err(NoticeError::InvalidGeometry);
                }
                Ok(())
            }
        }
    }

    /// Whether `position` falls horizontally inside this shape.
    pub fn contains(&self, position: GeodeticPosition) -> bool {
        match self {
            Self::Circle { center, radius_m } => {
                horizontal_distance_m(*center, position).is_some_and(|d| d <= *radius_m)
            }
            Self::Polygon { vertices } => point_in_polygon(position, vertices),
        }
    }

    /// Whether any part of this shape may lie within `search_radius_m` of `position`
    /// (exact for circles; conservative vertex-or-interior check for polygons).
    pub fn within_radius(&self, position: GeodeticPosition, search_radius_m: f64) -> bool {
        if search_radius_m < 0.0 || !search_radius_m.is_finite() {
            return false;
        }
        match self {
            Self::Circle { center, radius_m } => horizontal_distance_m(*center, position)
                .is_some_and(|d| d <= *radius_m + search_radius_m),
            Self::Polygon { vertices } => {
                if point_in_polygon(position, vertices) {
                    return true;
                }
                vertices.iter().any(|v| {
                    horizontal_distance_m(*v, position).is_some_and(|d| d <= search_radius_m)
                })
            }
        }
    }
}

fn horizontal_distance_m(a: GeodeticPosition, b: GeodeticPosition) -> Option<f64> {
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

fn point_in_polygon(point: GeodeticPosition, vertices: &[GeodeticPosition]) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let origin = GeodeticPosition {
        ellipsoidal_height_m: 0.0,
        ..point
    };
    let Ok(plane) = LocalTangentPlane::new(origin) else {
        return false;
    };
    let mut local_pts = Vec::with_capacity(vertices.len());
    for v in vertices {
        let flat = GeodeticPosition {
            ellipsoidal_height_m: 0.0,
            ..*v
        };
        let Ok(ned) = plane.to_ned(flat) else {
            return false;
        };
        local_pts.push((ned.east_m, ned.north_m));
    }
    let mut inside = false;
    let count = local_pts.len();
    for (idx, &(xi, yi)) in local_pts.iter().enumerate() {
        let prev_idx = if idx == 0 { count - 1 } else { idx - 1 };
        let Some(&(xj, yj)) = local_pts.get(prev_idx) else {
            return false;
        };
        if (yi > 0.0) != (yj > 0.0) {
            let denom = yj - yi;
            if denom.abs() > 1e-12 {
                let x_intersect = xi - yi * (xj - xi) / denom;
                if x_intersect >= 0.0 {
                    inside = !inside;
                }
            }
        }
    }
    inside
}
