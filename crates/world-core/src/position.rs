//! Where on the Earth a question is asked.

/// The reference frame a geodetic position is stated in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HorizontalDatum {
    /// The World Geodetic System 1984 ellipsoid and frame.
    Wgs84,
}

/// A geodetic position: latitude, longitude and height above the datum's
/// ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeodeticPosition {
    /// Geodetic latitude, degrees north, in [-90, 90].
    pub latitude_deg: f64,
    /// Longitude, degrees east.
    pub longitude_deg: f64,
    /// Height above the datum's ellipsoid, meters.
    pub ellipsoidal_height_m: f64,
    /// The datum the coordinates are stated in.
    pub datum: HorizontalDatum,
}

impl GeodeticPosition {
    /// A WGS-84 position.
    pub const fn wgs84(latitude_deg: f64, longitude_deg: f64, ellipsoidal_height_m: f64) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            ellipsoidal_height_m,
            datum: HorizontalDatum::Wgs84,
        }
    }

    /// Whether the latitude is in [-90, 90] and every coordinate is finite.
    pub fn is_well_formed(&self) -> bool {
        (-90.0..=90.0).contains(&self.latitude_deg)
            && self.longitude_deg.is_finite()
            && self.ellipsoidal_height_m.is_finite()
    }
}

#[cfg(test)]
mod tests;
