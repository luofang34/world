//! Recommendation ITU-R P.531-15, *Ionospheric propagation data and
//! prediction methods required for the design of satellite networks and
//! systems*.
//!
//! Evaluates trans-ionospheric propagation effects at frequencies from 30 MHz
//! upward over a single thin-shell ionosphere (`default 400 km` altitude):
//! - thin-shell slant TEC mapping from vertical TEC (`1 TECU = 1e16 el/m²`),
//! - excess group delay `40.3 * STEC / (c * f²)`, group range error, carrier
//!   phase advance, and frequency dispersion `dτ_g / df` (`ps/Hz`),
//! - Faraday rotation along a supplied longitudinal geomagnetic field `B_L`,
//! - D/E-region and auroral ionospheric absorption (§3), and
//! - statistical amplitude scintillation fading from the `S4` index via the
//!   Nakagami-m intensity distribution (§4).

mod absorption;
mod gamma;

#[cfg(test)]
mod tests;

use core::f64::consts::PI;

use crate::error::{PropagationError, finite, non_negative, positive};
use crate::loss::{Loss, PathLoss, Signal};
use crate::units::{Frequency, Percentage, SPEED_OF_LIGHT_M_S};

/// Mean spherical Earth radius used in the thin-shell obliquity factor, m.
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Default single-shell ionospheric effective height, m (400 km; §2.2).
pub const DEFAULT_SHELL_HEIGHT_M: f64 = 400_000.0;

/// Lowest carrier frequency covered by the high-frequency (`1/f²`) model, Hz.
pub const MIN_FREQUENCY_HZ: f64 = 30.0e6;

/// One Total Electron Content Unit (TECU) in electrons per square metre.
pub const TECU_ELECTRONS_PER_M2: f64 = 1.0e16;

/// First-order plasma refractivity constant `e² / (8π² ε₀ m_e)`, m³/s².
const PLASMA_CONSTANT_M3_S2: f64 = 40.3;

/// ITU-R P.531 §2.4 Faraday rotation constant when `f` is in GHz, `N_T` in
/// el/m², and `B_av` in tesla: `θ = 2.36e-14 * B_av * N_T / f_GHz²`.
const FARADAY_CONSTANT: f64 = 2.36e-14;

/// Maximum amplitude scintillation index `S4` accepted by the Nakagami-m model.
const MAX_S4: f64 = 1.5;

/// Geomagnetic latitude zone controlling background and auroral absorption.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IonosphericZone {
    /// Equatorial and low geomagnetic latitudes (`|λ_m| < 20°`).
    LowLatitude,
    /// Middle geomagnetic latitudes (`20° <= |λ_m| < 60°`).
    MidLatitude,
    /// Auroral oval (`60° <= |λ_m| < 75°`, P.531 §3.2 Table 2).
    Auroral,
    /// Polar cap (`|λ_m| >= 75°`).
    PolarCap,
}

impl IonosphericZone {
    /// Classifies a geomagnetic (or approximate geodetic) latitude in degrees.
    pub fn from_latitude_deg(latitude_deg: f64) -> Self {
        let abs_lat = libm::fabs(latitude_deg);
        if abs_lat < 20.0 {
            Self::LowLatitude
        } else if abs_lat < 60.0 {
            Self::MidLatitude
        } else if abs_lat < 75.0 {
            Self::Auroral
        } else {
            Self::PolarCap
        }
    }
}

/// Thin-shell ITU-R P.531 ionospheric propagation model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P531 {
    shell_height_m: f64,
    earth_radius_m: f64,
}

impl Default for P531 {
    fn default() -> Self {
        Self::new()
    }
}

impl P531 {
    /// Model using the standard 400 km ionospheric shell height and 6371 km
    /// mean Earth radius.
    pub const fn new() -> Self {
        Self {
            shell_height_m: DEFAULT_SHELL_HEIGHT_M,
            earth_radius_m: EARTH_RADIUS_M,
        }
    }

    /// Model with a caller-specified thin-shell height `shell_height_m` (for
    /// example, 350 km to 400 km).
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] if `shell_height_m` is not
    /// finite and strictly positive.
    pub fn with_shell_height_m(shell_height_m: f64) -> Result<Self, PropagationError> {
        Ok(Self {
            shell_height_m: positive("shell_height_m", shell_height_m)?,
            earth_radius_m: EARTH_RADIUS_M,
        })
    }

    /// Thin-shell effective height, m.
    pub const fn shell_height_m(&self) -> f64 {
        self.shell_height_m
    }

    /// Mean spherical Earth radius used in obliquity mapping, m.
    pub const fn earth_radius_m(&self) -> f64 {
        self.earth_radius_m
    }

    /// Geometric thin-shell obliquity factor `M(E) = STEC / VTEC` at elevation
    /// `elevation_deg` in `[0, 90]`.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] if `elevation_deg` is outside
    /// `[0, 90]` or non-finite.
    pub fn obliquity_factor(&self, elevation_deg: f64) -> Result<f64, PropagationError> {
        check_elevation(elevation_deg)?;
        if elevation_deg == 90.0 {
            return Ok(1.0);
        }
        let cos_el = libm::cos(elevation_deg.to_radians());
        let ratio = (self.earth_radius_m / (self.earth_radius_m + self.shell_height_m)) * cos_el;
        Ok(1.0 / libm::sqrt(1.0 - ratio * ratio))
    }
}

/// Slant Earth-space path inputs for [`P531`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P531Path {
    /// Apparent elevation angle at the ground terminal, degrees (`0..=90`).
    pub elevation_deg: f64,
    /// Vertical Total Electron Content at the ionospheric pierce point, TECU
    /// (`1 TECU = 1e16 el/m²`).
    pub vtec_tecu: f64,
    /// Optional path-averaged longitudinal geomagnetic flux density `B_L`,
    /// tesla (positive when parallel to the wave vector).
    pub b_longitudinal_t: Option<f64>,
    /// Optional intensity scintillation index `S4` at the carrier frequency
    /// (`0.0..=1.5`).
    pub scintillation_s4: Option<f64>,
    /// Percentage of time the statistical attenuation/fade is exceeded.
    pub percentage: Percentage,
    /// Geomagnetic latitude regime for background/auroral absorption.
    pub zone: IonosphericZone,
}

impl P531Path {
    /// Mid-latitude path with no geomagnetic field or scintillation input.
    pub const fn new(elevation_deg: f64, vtec_tecu: f64, percentage: Percentage) -> Self {
        Self {
            elevation_deg,
            vtec_tecu,
            b_longitudinal_t: None,
            scintillation_s4: None,
            percentage,
            zone: IonosphericZone::MidLatitude,
        }
    }

    /// Sets the path-averaged longitudinal geomagnetic field, tesla.
    pub const fn with_geomagnetic_field_t(mut self, b_longitudinal_t: f64) -> Self {
        self.b_longitudinal_t = Some(b_longitudinal_t);
        self
    }

    /// Sets the intensity scintillation index `S4`.
    pub const fn with_scintillation_s4(mut self, s4: f64) -> Self {
        self.scintillation_s4 = Some(s4);
        self
    }

    /// Sets the geomagnetic latitude zone.
    pub const fn with_zone(mut self, zone: IonosphericZone) -> Self {
        self.zone = zone;
        self
    }
}

/// Ionospheric propagation loss and dispersive / Faraday side quantities.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P531Loss {
    /// Thin-shell slant-to-vertical TEC obliquity factor (`>= 1.0`).
    pub obliquity_factor: f64,
    /// Slant Total Electron Content along the path, TECU.
    pub slant_tec_tecu: f64,
    /// Excess ionospheric group delay `40.3 * STEC / (c * f²)`, s.
    pub group_delay_s: f64,
    /// Equivalent group range error `c * group_delay_s`, m.
    pub group_range_error_m: f64,
    /// Carrier phase advance `2π * f * group_delay_s`, rad.
    pub phase_advance_rad: f64,
    /// Group-delay dispersion `dτ_g / df = -2 * group_delay_s / f`, ps/Hz.
    pub dispersion_ps_per_hz: f64,
    /// Faraday polarization rotation angle, rad (`None` if `B_L` was omitted).
    pub faraday_rotation_rad: Option<f64>,
    /// D/E-region or auroral absorption attenuation, dB.
    pub absorption_db: f64,
    /// Scintillation index `S4` used (0.0 when omitted).
    pub scintillation_s4: f64,
    /// Peak-to-peak amplitude scintillation `27.5 * S4^1.26`, dB (P.531 §4.3).
    pub scintillation_peak_to_peak_db: f64,
    /// Statistical scintillation fade depth exceeded for `percentage`, dB.
    pub scintillation_fade_db: f64,
    /// Total ionospheric attenuation (`absorption_db + scintillation_fade_db`), dB.
    pub total_loss_db: f64,
}

impl Loss for P531Loss {
    fn loss_db(&self) -> f64 {
        self.total_loss_db
    }
}

impl PathLoss<P531Path> for P531 {
    type Output = P531Loss;

    fn path_loss(
        &self,
        geometry: &P531Path,
        signal: &Signal,
    ) -> Result<P531Loss, PropagationError> {
        let f_hz = signal.frequency.hz();
        if f_hz < MIN_FREQUENCY_HZ {
            return Err(PropagationError::FrequencyOutOfRange {
                frequency_hz: f_hz,
                min_hz: MIN_FREQUENCY_HZ,
                max_hz: f64::INFINITY,
            });
        }
        let vtec_tecu = non_negative("vtec_tecu", geometry.vtec_tecu)?;
        let obliquity_factor = self.obliquity_factor(geometry.elevation_deg)?;
        let slant_tec_tecu = vtec_tecu * obliquity_factor;
        let stec_el_m2 = slant_tec_tecu * TECU_ELECTRONS_PER_M2;

        let group_delay_s = PLASMA_CONSTANT_M3_S2 * stec_el_m2 / (SPEED_OF_LIGHT_M_S * f_hz * f_hz);
        let group_range_error_m = SPEED_OF_LIGHT_M_S * group_delay_s;
        let phase_advance_rad = 2.0 * PI * f_hz * group_delay_s;
        let dispersion_ps_per_hz = -2.0 * group_delay_s * 1.0e12 / f_hz;

        let faraday_rotation_rad = match geometry.b_longitudinal_t {
            Some(b) => {
                let b_val = finite("b_longitudinal_t", b)?;
                let f_ghz = signal.frequency.ghz();
                Some(FARADAY_CONSTANT * b_val * stec_el_m2 / (f_ghz * f_ghz))
            }
            None => None,
        };

        let s4 = match geometry.scintillation_s4 {
            Some(val) => check_s4(val)?,
            None => 0.0,
        };
        let scintillation_peak_to_peak_db = if s4 > 0.0 {
            27.5 * libm::pow(s4.min(1.0), 1.26)
        } else {
            0.0
        };
        let scintillation_fade_db = gamma::nakagami_fade_db(s4, geometry.percentage.fraction());
        let absorption_db = absorption::ionospheric_absorption_db(
            geometry.zone,
            geometry.elevation_deg,
            f_hz,
            vtec_tecu,
            geometry.percentage,
        );
        let total_loss_db = absorption_db + scintillation_fade_db;

        Ok(P531Loss {
            obliquity_factor,
            slant_tec_tecu,
            group_delay_s,
            group_range_error_m,
            phase_advance_rad,
            dispersion_ps_per_hz,
            faraday_rotation_rad,
            absorption_db,
            scintillation_s4: s4,
            scintillation_peak_to_peak_db,
            scintillation_fade_db,
            total_loss_db,
        })
    }
}

/// Scales a weak-to-moderate scintillation index `S4` from `reference_freq`
/// to `target_freq` via ITU-R P.531 §4.1 (`S4 ∝ f^-1.5`).
///
/// # Errors
///
/// [`PropagationError::InvalidParameter`] if `s4_ref` is outside `[0, 1.5]`.
pub fn scale_s4_by_frequency(
    s4_ref: f64,
    reference_freq: Frequency,
    target_freq: Frequency,
) -> Result<f64, PropagationError> {
    let s4 = check_s4(s4_ref)?;
    let scaled = s4 * libm::pow(reference_freq.hz() / target_freq.hz(), 1.5);
    Ok(scaled.min(MAX_S4))
}

fn check_elevation(elevation_deg: f64) -> Result<f64, PropagationError> {
    if elevation_deg.is_finite() && (0.0..=90.0).contains(&elevation_deg) {
        Ok(elevation_deg)
    } else {
        Err(PropagationError::InvalidParameter {
            name: "elevation_deg",
            value: elevation_deg,
        })
    }
}

fn check_s4(s4: f64) -> Result<f64, PropagationError> {
    if s4.is_finite() && (0.0..=MAX_S4).contains(&s4) {
        Ok(s4)
    } else {
        Err(PropagationError::InvalidParameter {
            name: "scintillation_s4",
            value: s4,
        })
    }
}
