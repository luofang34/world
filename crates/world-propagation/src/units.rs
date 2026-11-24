//! The quantities every propagation model is asked about.

use crate::error::{PropagationError, positive};

/// Speed of light in vacuum, m/s (exact, SI).
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// A carrier frequency: finite and strictly positive.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Frequency {
    hz: f64,
}

impl Frequency {
    /// A frequency in hertz.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] for a non-finite or
    /// non-positive value.
    pub fn from_hz(hz: f64) -> Result<Self, PropagationError> {
        Ok(Self {
            hz: positive("frequency_hz", hz)?,
        })
    }

    /// A frequency in megahertz.
    ///
    /// # Errors
    ///
    /// As [`Frequency::from_hz`].
    pub fn from_mhz(mhz: f64) -> Result<Self, PropagationError> {
        Self::from_hz(mhz * 1.0e6)
    }

    /// A frequency in gigahertz.
    ///
    /// # Errors
    ///
    /// As [`Frequency::from_hz`].
    pub fn from_ghz(ghz: f64) -> Result<Self, PropagationError> {
        Self::from_hz(ghz * 1.0e9)
    }

    /// The frequency in hertz.
    pub fn hz(self) -> f64 {
        self.hz
    }

    /// The frequency in megahertz, the unit of the ITU-R practical-unit
    /// formulae.
    pub fn mhz(self) -> f64 {
        self.hz * 1.0e-6
    }

    /// The frequency in gigahertz.
    pub fn ghz(self) -> f64 {
        self.hz * 1.0e-9
    }

    /// The free-space wavelength, m.
    pub fn wavelength_m(self) -> f64 {
        SPEED_OF_LIGHT_M_S / self.hz
    }
}

/// Linear polarisation of the wave relative to the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Polarisation {
    /// Electric field parallel to the ground.
    Horizontal,
    /// Electric field in the vertical plane of the path.
    Vertical,
}

/// A percentage strictly between 0 and 100: the fraction of time,
/// locations or situations a statistical model's loss is not exceeded for.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Percentage {
    percent: f64,
}

impl Percentage {
    /// A percentage in (0, 100).
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] outside that open interval
    /// or for NaN; 0 % and 100 % are the tails no statistical model
    /// defines.
    pub fn new(percent: f64) -> Result<Self, PropagationError> {
        if percent > 0.0 && percent < 100.0 {
            Ok(Self { percent })
        } else {
            Err(PropagationError::InvalidParameter {
                name: "percentage",
                value: percent,
            })
        }
    }

    /// The value in percent.
    pub fn percent(self) -> f64 {
        self.percent
    }

    /// The value as a fraction in (0, 1).
    pub fn fraction(self) -> f64 {
        self.percent / 100.0
    }
}

#[cfg(test)]
mod tests;
