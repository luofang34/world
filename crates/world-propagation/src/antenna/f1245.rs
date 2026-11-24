//! Recommendation ITU-R F.1245-3, recommends 2.

use crate::antenna::{AntennaPattern, diameter_over_wavelength, off_axis};
use crate::error::{PropagationError, finite};
use crate::units::Frequency;

/// Lowest frequency the pattern covers, Hz.
const MIN_HZ: f64 = 1.0e9;
/// Top of the lower band (recommends 2.1.1 and 2.2.1), Hz.
const LOWER_BAND_MAX_HZ: f64 = 70.0e9;
/// Highest frequency the pattern covers, Hz.
const MAX_HZ: f64 = 86.0e9;
/// D/λ above which the first side lobe is modelled (recommends 2.1).
const LARGE_ANTENNA_D_OVER_LAMBDA: f64 = 100.0;

/// The F.1245-3 average radiation pattern of a point-to-point
/// fixed-wireless antenna: a parabolic main beam, the first side lobe G₁
/// and a 25 log φ side-lobe decay to a floor at 48° (120° above 70 GHz).
///
/// Annex 1's generalised patterns and Note 7's circular-polarisation
/// advantage are not modelled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct F1245 {
    d_over_lambda: f64,
    max_gain_dbi: f64,
    first_side_lobe_dbi: f64,
    main_beam_until_deg: f64,
    floor_from_deg: f64,
    upper_band: bool,
}

impl F1245 {
    /// The pattern of a dish `diameter_m` across at `frequency`, with the
    /// maximum gain from Note 2: 20 log(D/λ) ≈ G_max − 7.7.
    ///
    /// # Errors
    ///
    /// As [`F1245::with_max_gain`].
    pub fn new(diameter_m: f64, frequency: Frequency) -> Result<Self, PropagationError> {
        let d_over_lambda = diameter_over_wavelength(diameter_m, frequency, MIN_HZ, MAX_HZ)?;
        Self::build(
            d_over_lambda,
            20.0 * libm::log10(d_over_lambda) + 7.7,
            frequency,
        )
    }

    /// The pattern of a dish `diameter_m` across at `frequency` whose
    /// maximum gain `max_gain_dbi` is known.
    ///
    /// # Errors
    ///
    /// [`PropagationError::FrequencyOutOfRange`] outside 1 to 86 GHz;
    /// [`PropagationError::InvalidParameter`] for a non-positive diameter,
    /// a maximum gain not above the first side lobe G₁, or an antenna so
    /// small its main beam would reach the far-out floor.
    pub fn with_max_gain(
        diameter_m: f64,
        frequency: Frequency,
        max_gain_dbi: f64,
    ) -> Result<Self, PropagationError> {
        let d_over_lambda = diameter_over_wavelength(diameter_m, frequency, MIN_HZ, MAX_HZ)?;
        Self::build(
            d_over_lambda,
            finite("max_gain_dbi", max_gain_dbi)?,
            frequency,
        )
    }

    fn build(
        d_over_lambda: f64,
        max_gain_dbi: f64,
        frequency: Frequency,
    ) -> Result<Self, PropagationError> {
        let first_side_lobe_dbi = 2.0 + 15.0 * libm::log10(d_over_lambda);
        let headroom = max_gain_dbi - first_side_lobe_dbi;
        if headroom.is_nan() || headroom <= 0.0 {
            return Err(PropagationError::InvalidParameter {
                name: "max_gain_dbi",
                value: max_gain_dbi,
            });
        }
        let upper_band = frequency.hz() > LOWER_BAND_MAX_HZ;
        let floor_from_deg = if upper_band { 120.0 } else { 48.0 };
        let main_beam_until_deg = 20.0 / d_over_lambda * libm::sqrt(headroom);
        if main_beam_until_deg >= floor_from_deg {
            return Err(PropagationError::InvalidParameter {
                name: "diameter_over_wavelength",
                value: d_over_lambda,
            });
        }
        Ok(Self {
            d_over_lambda,
            max_gain_dbi,
            first_side_lobe_dbi,
            main_beam_until_deg,
            floor_from_deg,
            upper_band,
        })
    }

    /// D/λ.
    pub fn diameter_over_wavelength(&self) -> f64 {
        self.d_over_lambda
    }

    /// G_max, dBi.
    pub fn max_gain_dbi(&self) -> f64 {
        self.max_gain_dbi
    }

    /// G₁ = 2 + 15 log(D/λ), dBi.
    pub fn first_side_lobe_dbi(&self) -> f64 {
        self.first_side_lobe_dbi
    }

    /// φ_m = (20 λ/D) √(G_max − G₁), where the main beam meets G₁, degrees.
    pub fn main_beam_until_deg(&self) -> f64 {
        self.main_beam_until_deg
    }

    /// φ_r = 12.02 (D/λ)^−0.6, degrees.
    pub fn first_side_lobe_until_deg(&self) -> f64 {
        12.02 * libm::pow(self.d_over_lambda, -0.6)
    }

    /// Where the far-out floor starts: 48°, or 120° above 70 GHz.
    pub fn floor_from_deg(&self) -> f64 {
        self.floor_from_deg
    }

    fn side_lobe_dbi(&self, phi_deg: f64) -> f64 {
        let decay = 25.0 * libm::log10(phi_deg);
        if self.d_over_lambda > LARGE_ANTENNA_D_OVER_LAMBDA {
            if phi_deg < self.first_side_lobe_until_deg() {
                self.first_side_lobe_dbi
            } else {
                29.0 - decay
            }
        } else {
            39.0 - 5.0 * libm::log10(self.d_over_lambda) - decay
        }
    }

    fn floor_dbi(&self) -> f64 {
        let large = self.d_over_lambda > LARGE_ANTENNA_D_OVER_LAMBDA;
        match (large, self.upper_band) {
            (true, false) => -13.0,
            (true, true) => -23.0,
            (false, false) => -3.0 - 5.0 * libm::log10(self.d_over_lambda),
            (false, true) => -13.0 - 5.0 * libm::log10(self.d_over_lambda),
        }
    }
}

impl AntennaPattern for F1245 {
    fn gain_dbi(&self, off_axis_deg: f64) -> Result<f64, PropagationError> {
        let phi = off_axis(off_axis_deg, 0.0)?;
        Ok(if phi < self.main_beam_until_deg {
            let x = self.d_over_lambda * phi;
            self.max_gain_dbi - 2.5e-3 * x * x
        } else if phi < self.floor_from_deg {
            self.side_lobe_dbi(phi)
        } else {
            self.floor_dbi()
        })
    }
}
