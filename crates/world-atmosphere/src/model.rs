//! The seam between consumers of the atmosphere and the models of it.

use core::fmt;

use world_core::DecimalYear;

use crate::Altitude;

/// Where and when the atmosphere is asked about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmospherePoint {
    /// Geodetic latitude, degrees north.
    pub latitude_deg: f64,
    /// Longitude, degrees east.
    pub longitude_deg: f64,
    /// Altitude above mean sea level.
    pub altitude: Altitude,
    /// The date and time.
    pub at: DecimalYear,
}

/// The state of the air at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphereState {
    /// Static air temperature, kelvin.
    pub temperature_k: f64,
    /// Static pressure, pascals.
    pub pressure_pa: f64,
    /// Air density, kg/m³.
    pub density_kg_m3: f64,
}

impl AtmosphereState {
    /// Static air temperature, degrees Celsius.
    pub fn temperature_c(&self) -> f64 {
        self.temperature_k - 273.15
    }
}

/// Why a model gave no state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AtmosphereError {
    /// The altitude is outside the model's domain.
    OutsideDomain {
        /// The requested geopotential altitude, meters.
        geopotential_m: f64,
        /// The lowest geopotential altitude the model covers, meters.
        lowest_m: f64,
        /// The highest geopotential altitude the model covers, meters.
        highest_m: f64,
    },
}

impl fmt::Display for AtmosphereError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideDomain {
                geopotential_m,
                lowest_m,
                highest_m,
            } => write!(
                f,
                "geopotential altitude {geopotential_m} m is outside the model's {lowest_m}..={highest_m} m"
            ),
        }
    }
}

impl core::error::Error for AtmosphereError {}

/// A model of the atmosphere: the state of the air at a point.
pub trait AtmosphereModel {
    /// The state of the air at `point`.
    ///
    /// # Errors
    ///
    /// [`AtmosphereError::OutsideDomain`] for a point the model does not
    /// cover.
    fn state(&self, point: AtmospherePoint) -> Result<AtmosphereState, AtmosphereError>;
}
