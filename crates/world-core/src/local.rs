//! The local north-east-down frame around a geodetic origin.

use core::fmt;

use libm::{cos, sin};

use crate::{Ecef, GeodeticPosition};

/// A displacement in a local north-east-down frame, meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ned {
    /// Toward true north along the origin's horizontal plane.
    pub north_m: f64,
    /// Toward east along the origin's horizontal plane.
    pub east_m: f64,
    /// Along the origin's ellipsoid normal, positive down.
    pub down_m: f64,
}

/// Why a position cannot anchor or be expressed in a local frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionError {
    /// Latitude outside [-90, 90] or a non-finite coordinate.
    NotWellFormed,
    /// The position is stated on a different datum from the frame's origin.
    DatumMismatch,
}

impl fmt::Display for PositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotWellFormed => "the position is not a well-formed geodetic position",
            Self::DatumMismatch => "the position is on a different datum from the frame origin",
        })
    }
}

impl core::error::Error for PositionError {}

/// The tangent plane at a geodetic origin: north-east-down axes on the
/// plane normal to the origin's datum ellipsoid. Conversions go through
/// ECEF on that ellipsoid, so they are exact to rounding at any distance,
/// not a flat-earth approximation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalTangentPlane {
    origin: GeodeticPosition,
    origin_ecef: Ecef,
    /// Rows: the north, east and down axes in ECEF.
    axes: [[f64; 3]; 3],
}

impl LocalTangentPlane {
    /// The frame at `origin`.
    ///
    /// # Errors
    ///
    /// [`PositionError::NotWellFormed`] for an origin that is not
    /// [`GeodeticPosition::is_well_formed`].
    pub fn new(origin: GeodeticPosition) -> Result<Self, PositionError> {
        if !origin.is_well_formed() {
            return Err(PositionError::NotWellFormed);
        }
        let ellipsoid = origin.datum.ellipsoid();
        let origin_ecef = ellipsoid.to_ecef(
            origin.latitude_deg,
            origin.longitude_deg,
            origin.ellipsoidal_height_m,
        );
        let (lat, lon) = (
            origin.latitude_deg.to_radians(),
            origin.longitude_deg.to_radians(),
        );
        let (sl, cl, so, co) = (sin(lat), cos(lat), sin(lon), cos(lon));
        Ok(Self {
            origin,
            origin_ecef,
            axes: [
                [-sl * co, -sl * so, cl],
                [-so, co, 0.0],
                [-cl * co, -cl * so, -sl],
            ],
        })
    }

    /// The frame's origin.
    pub fn origin(&self) -> GeodeticPosition {
        self.origin
    }

    /// The geodetic position at `offset` from the origin, on the origin's
    /// datum.
    pub fn to_geodetic(&self, offset: Ned) -> GeodeticPosition {
        let d = [offset.north_m, offset.east_m, offset.down_m];
        let [n, e, down] = self.axes;
        let point = Ecef {
            x_m: self.origin_ecef.x_m + n[0] * d[0] + e[0] * d[1] + down[0] * d[2],
            y_m: self.origin_ecef.y_m + n[1] * d[0] + e[1] * d[1] + down[1] * d[2],
            z_m: self.origin_ecef.z_m + n[2] * d[0] + e[2] * d[1] + down[2] * d[2],
        };
        let (latitude_deg, longitude_deg, ellipsoidal_height_m) =
            self.origin.datum.ellipsoid().from_ecef(point);
        GeodeticPosition {
            latitude_deg,
            longitude_deg,
            ellipsoidal_height_m,
            datum: self.origin.datum,
        }
    }

    /// The offset of `position` from the origin.
    ///
    /// # Errors
    ///
    /// [`PositionError::NotWellFormed`] for a malformed position and
    /// [`PositionError::DatumMismatch`] for one on another datum.
    pub fn to_ned(&self, position: GeodeticPosition) -> Result<Ned, PositionError> {
        if !position.is_well_formed() {
            return Err(PositionError::NotWellFormed);
        }
        if position.datum != self.origin.datum {
            return Err(PositionError::DatumMismatch);
        }
        let point = position.datum.ellipsoid().to_ecef(
            position.latitude_deg,
            position.longitude_deg,
            position.ellipsoidal_height_m,
        );
        let d = [
            point.x_m - self.origin_ecef.x_m,
            point.y_m - self.origin_ecef.y_m,
            point.z_m - self.origin_ecef.z_m,
        ];
        let along = |axis: [f64; 3]| axis[0] * d[0] + axis[1] * d[1] + axis[2] * d[2];
        Ok(Ned {
            north_m: along(self.axes[0]),
            east_m: along(self.axes[1]),
            down_m: along(self.axes[2]),
        })
    }
}

#[cfg(test)]
mod tests;
