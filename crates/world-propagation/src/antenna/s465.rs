//! Recommendation ITU-R S.465-6, recommends 2.

use crate::antenna::{AntennaPattern, diameter_over_wavelength, off_axis};
use crate::error::PropagationError;
use crate::units::Frequency;

/// Lowest frequency the pattern covers, Hz.
const MIN_HZ: f64 = 2.0e9;
/// Highest frequency the pattern covers, Hz.
const MAX_HZ: f64 = 31.0e9;
/// Angle beyond which the gain is a constant floor, degrees.
const FLOOR_FROM_DEG: f64 = 48.0;

/// The S.465-6 reference pattern of an earth-station antenna in the
/// fixed-satellite service: 32 − 25 log φ dBi from φ_min to 48°, −10 dBi
/// beyond.
///
/// The main beam inside φ_min is not defined by the Recommendation and is
/// refused. The pre-1993 pattern of Note 4 and the 2.5° φ_min of Note 5
/// are not modelled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct S465 {
    min_angle_deg: f64,
}

impl S465 {
    /// The pattern of a dish `diameter_m` across at `frequency`.
    ///
    /// # Errors
    ///
    /// [`PropagationError::FrequencyOutOfRange`] outside 2 to 31 GHz;
    /// [`PropagationError::InvalidParameter`] for a non-positive diameter.
    pub fn new(diameter_m: f64, frequency: Frequency) -> Result<Self, PropagationError> {
        let d_over_lambda = diameter_over_wavelength(diameter_m, frequency, MIN_HZ, MAX_HZ)?;
        Ok(Self {
            min_angle_deg: min_angle_deg(d_over_lambda),
        })
    }

    /// φ_min, the smallest off-axis angle the pattern defines, degrees.
    pub fn min_angle_deg(&self) -> f64 {
        self.min_angle_deg
    }
}

/// φ_min of recommends 2: the greater of 1° and 100 λ/D for D/λ ≥ 50,
/// the greater of 2° and 114 (D/λ)^−1.09 below.
fn min_angle_deg(d_over_lambda: f64) -> f64 {
    if d_over_lambda >= 50.0 {
        (100.0 / d_over_lambda).max(1.0)
    } else {
        (114.0 * libm::pow(d_over_lambda, -1.09)).max(2.0)
    }
}

/// The side-lobe envelope from φ_min outward, dBi.
pub(crate) fn envelope_dbi(phi_deg: f64) -> f64 {
    if phi_deg < FLOOR_FROM_DEG {
        32.0 - 25.0 * libm::log10(phi_deg)
    } else {
        -10.0
    }
}

impl AntennaPattern for S465 {
    fn gain_dbi(&self, off_axis_deg: f64) -> Result<f64, PropagationError> {
        Ok(envelope_dbi(off_axis(off_axis_deg, self.min_angle_deg)?))
    }
}
