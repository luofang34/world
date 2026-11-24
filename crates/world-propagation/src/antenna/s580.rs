//! Recommendation ITU-R S.580-6, recommends 1 and 2 with Note 5.

use crate::antenna::{AntennaPattern, diameter_over_wavelength, off_axis, s465};
use crate::error::PropagationError;
use crate::units::Frequency;

/// The frequencies of the S.465 pattern S.580 hands over to, Hz.
const MIN_HZ: f64 = 2.0e9;
const MAX_HZ: f64 = 31.0e9;
/// Smallest D/λ the design objective applies to (Note 3).
const MIN_D_OVER_LAMBDA: f64 = 50.0;
/// End of the 29 − 25 log φ objective, degrees.
const OBJECTIVE_UNTIL_DEG: f64 = 20.0;
/// End of the −3.5 dBi bridge of Note 5, degrees.
const BRIDGE_UNTIL_DEG: f64 = 26.3;
/// Gain of the Note 5 bridge, dBi.
const BRIDGE_DBI: f64 = -3.5;

/// The S.580-6 design objective for a new earth-station antenna working a
/// geostationary satellite: 29 − 25 log φ dBi from φ_min to 20°, −3.5 dBi
/// to 26.3° (Note 5), and S.465 beyond.
///
/// It bounds 90 % of side-lobe peaks in directions within 3° of the
/// geostationary arc. The main beam inside φ_min is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct S580 {
    min_angle_deg: f64,
}

impl S580 {
    /// The objective for a dish `diameter_m` across at `frequency`.
    ///
    /// # Errors
    ///
    /// [`PropagationError::FrequencyOutOfRange`] outside the 2 to 31 GHz
    /// of the S.465 pattern it continues into;
    /// [`PropagationError::InvalidParameter`] for a non-positive diameter
    /// or D/λ below 50, where Note 3 leaves the objective unset.
    pub fn new(diameter_m: f64, frequency: Frequency) -> Result<Self, PropagationError> {
        let d_over_lambda = diameter_over_wavelength(diameter_m, frequency, MIN_HZ, MAX_HZ)?;
        if d_over_lambda < MIN_D_OVER_LAMBDA {
            return Err(PropagationError::InvalidParameter {
                name: "diameter_over_wavelength",
                value: d_over_lambda,
            });
        }
        Ok(Self {
            min_angle_deg: (100.0 / d_over_lambda).max(1.0),
        })
    }

    /// φ_min: the greater of 1° and 100 λ/D, degrees.
    pub fn min_angle_deg(&self) -> f64 {
        self.min_angle_deg
    }
}

impl AntennaPattern for S580 {
    fn gain_dbi(&self, off_axis_deg: f64) -> Result<f64, PropagationError> {
        let phi = off_axis(off_axis_deg, self.min_angle_deg)?;
        Ok(if phi <= OBJECTIVE_UNTIL_DEG {
            29.0 - 25.0 * libm::log10(phi)
        } else if phi <= BRIDGE_UNTIL_DEG {
            BRIDGE_DBI
        } else {
            s465::envelope_dbi(phi)
        })
    }
}
