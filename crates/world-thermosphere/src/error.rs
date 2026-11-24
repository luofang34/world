//! Why a model gave no state.

use core::fmt;

/// Why a model gave no state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThermosphereError {
    /// An input is not finite or is outside the range it is defined on.
    InvalidInput {
        /// The input, as the field of [`crate::MsisInput`] that carries it.
        quantity: &'static str,
        /// The value given.
        value: f64,
    },
    /// The altitude is below the model's lower boundary.
    BelowModel {
        /// The requested geodetic altitude, kilometers.
        altitude_km: f64,
        /// The lowest geodetic altitude the model covers, kilometers.
        lowest_km: f64,
    },
    /// Below 72.5 km the model scales the fully mixed nitrogen density,
    /// which [`crate::Effect::DiffusiveEquilibriumDeparture`] being off
    /// leaves undefined.
    MixingSwitchedOff {
        /// The requested geodetic altitude, kilometers.
        altitude_km: f64,
    },
    /// The inputs are individually valid but the model's result is not a
    /// finite number.
    NotFinite,
}

impl fmt::Display for ThermosphereError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { quantity, value } => {
                write!(f, "{quantity} = {value} is not a valid model input")
            }
            Self::BelowModel {
                altitude_km,
                lowest_km,
            } => write!(
                f,
                "altitude {altitude_km} km is below the model's lowest {lowest_km} km"
            ),
            Self::MixingSwitchedOff { altitude_km } => write!(
                f,
                "altitude {altitude_km} km is below 72.5 km, where the model needs the mixing term that is switched off"
            ),
            Self::NotFinite => f.write_str("the model's result is not finite for these inputs"),
        }
    }
}

impl core::error::Error for ThermosphereError {}
