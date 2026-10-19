//! Altitude, with the vertical reference it is measured in.

/// The Earth radius that relates geometric and geopotential altitude in
/// the standard atmosphere, meters (ISO 2533).
const STANDARD_EARTH_RADIUS_M: f64 = 6_356_766.0;

/// An altitude above mean sea level, stating which kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Altitude {
    /// Geopotential altitude, meters: the height in a uniform field of
    /// standard gravity that has the same potential. The standard
    /// atmosphere is defined in it.
    Geopotential(f64),
    /// Geometric height above mean sea level, meters.
    GeometricMsl(f64),
}

impl Altitude {
    /// The geopotential altitude, meters.
    pub fn geopotential_m(self) -> f64 {
        match self {
            Self::Geopotential(meters) => meters,
            Self::GeometricMsl(meters) => {
                STANDARD_EARTH_RADIUS_M * meters / (STANDARD_EARTH_RADIUS_M + meters)
            }
        }
    }
}

#[cfg(test)]
mod tests;
