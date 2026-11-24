//! ITU-R reference antenna radiation patterns as closed-form gain
//! functions of the off-axis angle.
//!
//! - [`S465`]: earth stations in the fixed-satellite service, 2 to 31 GHz,
//!   for coordination and interference assessment (S.465-6).
//! - [`S580`]: the design-objective side-lobe envelope of earth stations
//!   working geostationary satellites (S.580-6), joined to S.465 beyond it.
//! - [`F1245`]: the average pattern of point-to-point fixed-wireless
//!   antennas, 1 to 86 GHz (F.1245-3 recommends 2).
//!
//! All three are rotationally symmetric and take the angle from boresight
//! in degrees, as the Recommendations write them.

mod f1245;
mod s465;
mod s580;

pub use f1245::F1245;
pub use s465::S465;
pub use s580::S580;

use crate::error::{PropagationError, positive};
use crate::units::Frequency;

/// A rotationally symmetric antenna gain pattern.
pub trait AntennaPattern {
    /// Gain relative to an isotropic antenna, dBi, at `off_axis_deg`
    /// degrees from boresight (0 to 180).
    ///
    /// # Errors
    ///
    /// [`PropagationError::AngleOutsidePattern`] for an angle outside 0 to
    /// 180 degrees, or inside a main beam the Recommendation leaves
    /// undefined.
    fn gain_dbi(&self, off_axis_deg: f64) -> Result<f64, PropagationError>;
}

/// Largest off-axis angle, degrees.
const MAX_OFF_AXIS_DEG: f64 = 180.0;

/// `off_axis_deg` if it is within `min_deg` to 180 degrees.
fn off_axis(off_axis_deg: f64, min_deg: f64) -> Result<f64, PropagationError> {
    if (min_deg..=MAX_OFF_AXIS_DEG).contains(&off_axis_deg) {
        Ok(off_axis_deg)
    } else {
        Err(PropagationError::AngleOutsidePattern {
            off_axis_deg,
            min_deg,
            max_deg: MAX_OFF_AXIS_DEG,
        })
    }
}

/// D/λ for a dish of diameter `diameter_m` at `frequency`, after checking
/// the frequency lies in `min_hz..=max_hz`.
fn diameter_over_wavelength(
    diameter_m: f64,
    frequency: Frequency,
    min_hz: f64,
    max_hz: f64,
) -> Result<f64, PropagationError> {
    let frequency_hz = frequency.hz();
    if !(min_hz..=max_hz).contains(&frequency_hz) {
        return Err(PropagationError::FrequencyOutOfRange {
            frequency_hz,
            min_hz,
            max_hz,
        });
    }
    Ok(positive("diameter_m", diameter_m)? / frequency.wavelength_m())
}

#[cfg(test)]
mod tests;
