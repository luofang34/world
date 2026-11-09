//! Aviation weather advisories and convective/icing/turbulence hazard volumes.

use alloc::vec::Vec;
use libm::hypot;
use world_core::{GeodeticPosition, LocalTangentPlane, Provenance, ValidityWindow};

/// Category of an operational weather hazard volume.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum HazardKind {
    /// Non-convective SIGMET (severe icing, severe turbulence, volcanic ash, dust).
    Sigmet,
    /// Convective SIGMET (embedded thunderstorms, squall lines, hail).
    ConvectiveSigmet,
    /// AIRMET or G-AIRMET (IFR/mountain obscuration, moderate icing/turbulence, LLWS).
    Airmet,
    /// Center Weather Advisory.
    CenterWeatherAdvisory,
    /// Precipitation / radar reflectivity cell.
    RadarCell {
        /// Peak radar reflectivity in dBZ.
        peak_dbz: f32,
    },
}

/// Icing intensity reported or forecast inside a hazard volume.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum IcingSeverity {
    /// No icing reported.
    #[default]
    None,
    /// Trace icing.
    Trace,
    /// Light icing.
    Light,
    /// Moderate icing.
    Moderate,
    /// Severe icing.
    Severe,
}

/// Turbulence intensity reported or forecast inside a hazard volume.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TurbulenceSeverity {
    /// Smooth air.
    #[default]
    None,
    /// Light turbulence.
    Light,
    /// Moderate turbulence.
    Moderate,
    /// Severe turbulence.
    Severe,
    /// Extreme turbulence.
    Extreme,
}

/// Horizontal footprint of a weather hazard volume on the datum ellipsoid.
#[derive(Clone, Debug, PartialEq)]
pub enum HazardGeometry {
    /// Circle centered at `center` with `radius_m` horizontal radius.
    Circle {
        /// Geodetic center.
        center: GeodeticPosition,
        /// Horizontal radius in meters.
        radius_m: f64,
    },
    /// Closed polygon given by ordered vertices on the same datum.
    Polygon {
        /// Ordered polygon boundary vertices (at least three).
        vertices: Vec<GeodeticPosition>,
    },
}

/// A three-dimensional, time-bounded weather hazard volume.
#[derive(Clone, Debug, PartialEq)]
pub struct WeatherHazard {
    /// Stable hazard identifier within the producer's store.
    pub id: u64,
    /// Advisory or radar cell category.
    pub kind: HazardKind,
    /// Horizontal footprint.
    pub geometry: HazardGeometry,
    /// Base altitude above mean sea level, meters.
    pub floor_msl_m: f64,
    /// Top altitude above mean sea level, meters.
    pub ceiling_msl_m: f64,
    /// Validity interval.
    pub valid: ValidityWindow,
    /// Icing severity inside the volume.
    pub icing: IcingSeverity,
    /// Turbulence severity inside the volume.
    pub turbulence: TurbulenceSeverity,
    /// Optional dataset provenance.
    pub provenance: Option<Provenance>,
}

impl WeatherHazard {
    /// Whether `position` at `altitude_msl_m` and `at` falls inside this hazard.
    pub fn contains(
        &self,
        position: GeodeticPosition,
        altitude_msl_m: f64,
        at: world_core::DecimalYear,
    ) -> bool {
        if !self.valid.contains(at) {
            return false;
        }
        if !(self.floor_msl_m..=self.ceiling_msl_m).contains(&altitude_msl_m) {
            return false;
        }
        match &self.geometry {
            HazardGeometry::Circle { center, radius_m } => {
                if *radius_m < 0.0 {
                    return false;
                }
                horizontal_distance_m(*center, position).is_some_and(|d| d <= *radius_m)
            }
            HazardGeometry::Polygon { vertices } => point_in_polygon(position, vertices),
        }
    }
}

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

/// Even-odd ray-casting test in the local north-east tangent plane at `point`.
pub fn point_in_polygon(point: GeodeticPosition, vertices: &[GeodeticPosition]) -> bool {
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
        let crosses_north = (yi > 0.0) != (yj > 0.0);
        if crosses_north {
            let denom = yj - yi;
            if denom.abs() > 1e-12 {
                let x_intersect = xi + (0.0 - yi) * (xj - xi) / denom;
                if x_intersect >= 0.0 {
                    inside = !inside;
                }
            }
        }
    }
    inside
}
