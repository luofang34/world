//! Diffraction over a smooth spherical Earth (P.526-16 §3.1.1 and §3.2),
//! for 10 MHz and above.

use crate::error::{PropagationError, non_negative, positive};
use crate::ground::{Ground, validate_ground};
use crate::loss::{Loss, PathLoss, Signal};
use crate::units::Polarisation;

/// Lowest frequency §3 covers; below it the Recommendation sends the
/// caller to the LFMF-SmoothEarth residue-series program of P.368.
pub const MIN_FREQUENCY_HZ: f64 = 10.0e6;

/// Effective Earth radius to take when no other information is
/// available (§1), m.
pub const DEFAULT_EFFECTIVE_EARTH_RADIUS_M: f64 = 8_500_000.0;

/// The two terminals over a smooth Earth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalPath {
    /// Great-circle distance between the terminals, m.
    pub distance_m: f64,
    /// Transmitting antenna height above the smooth surface, m.
    pub tx_height_m: f64,
    /// Receiving antenna height above the smooth surface, m.
    pub rx_height_m: f64,
}

/// Diffraction over a smooth spherical Earth of a given effective radius,
/// at any distance (§3.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothEarth {
    effective_earth_radius_m: f64,
    ground: Ground,
}

impl SmoothEarth {
    /// A smooth Earth of effective radius `effective_earth_radius_m` and
    /// surface `ground`.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] for a non-positive radius or
    /// permittivity, or a negative conductivity.
    pub fn new(effective_earth_radius_m: f64, ground: Ground) -> Result<Self, PropagationError> {
        positive("effective_earth_radius_m", effective_earth_radius_m)?;
        validate_ground(ground)?;
        Ok(Self {
            effective_earth_radius_m,
            ground,
        })
    }

    /// The effective Earth radius, m.
    pub fn effective_earth_radius_m(&self) -> f64 {
        self.effective_earth_radius_m
    }

    /// The surface's electrical constants.
    pub fn ground(&self) -> Ground {
        self.ground
    }
}

/// A smooth-Earth diffraction loss.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothEarthLoss {
    /// The diffraction loss, dB (eq. 25, or the negated eq. 13 beyond the
    /// marginal line-of-sight distance).
    pub loss_db: f64,
    /// Whether the path is shorter than the marginal line-of-sight
    /// distance of eq. (21).
    pub line_of_sight: bool,
}

impl Loss for SmoothEarthLoss {
    fn loss_db(&self) -> f64 {
        self.loss_db
    }
}

impl PathLoss<TerminalPath> for SmoothEarth {
    type Output = SmoothEarthLoss;

    fn path_loss(
        &self,
        geometry: &TerminalPath,
        signal: &Signal,
    ) -> Result<SmoothEarthLoss, PropagationError> {
        spherical_earth_loss(geometry, self.effective_earth_radius_m, self.ground, signal)
    }
}

/// §3.2, in self-consistent units (m) apart from the practical-unit
/// first-term formulae it calls.
pub(crate) fn spherical_earth_loss(
    path: &TerminalPath,
    ae_m: f64,
    ground: Ground,
    signal: &Signal,
) -> Result<SmoothEarthLoss, PropagationError> {
    let frequency_hz = signal.frequency.hz();
    if frequency_hz < MIN_FREQUENCY_HZ {
        return Err(PropagationError::FrequencyOutOfRange {
            frequency_hz,
            min_hz: MIN_FREQUENCY_HZ,
            max_hz: f64::INFINITY,
        });
    }
    let d = positive("distance_m", path.distance_m)?;
    let h1 = non_negative("tx_height_m", path.tx_height_m)?;
    let h2 = non_negative("rx_height_m", path.rx_height_m)?;
    positive("tx_height_m + rx_height_m", h1 + h2)?;
    let first_term = |radius_m: f64| -first_term_field_db(d, h1, h2, radius_m, signal, ground);

    let d_los = libm::sqrt(2.0 * ae_m) * (libm::sqrt(h1) + libm::sqrt(h2)); // (21)
    if d >= d_los {
        return Ok(SmoothEarthLoss {
            loss_db: first_term(ae_m),
            line_of_sight: false,
        });
    }
    let c = (h1 - h2) / (h1 + h2); // (22d)
    let m = d * d / (4.0 * ae_m * (h1 + h2)); // (22e)
    let cosine_argument = 1.5 * c * libm::sqrt(3.0 * m / libm::pow(m + 1.0, 3.0));
    let b = 2.0
        * libm::sqrt((m + 1.0) / (3.0 * m))
        * libm::cos(
            core::f64::consts::FRAC_PI_3 + libm::acos(cosine_argument.clamp(-1.0, 1.0)) / 3.0,
        ); // (22c)
    let d1 = 0.5 * d * (1.0 + b); // (22a)
    let d2 = d - d1; // (22b)
    let clearance = ((h1 - d1 * d1 / (2.0 * ae_m)) * d2 + (h2 - d2 * d2 / (2.0 * ae_m)) * d1) / d; // (22)
    let required = 0.552 * libm::sqrt(d1 * d2 * signal.frequency.wavelength_m() / d); // (23)
    if clearance > required {
        return Ok(SmoothEarthLoss {
            loss_db: 0.0,
            line_of_sight: true,
        });
    }
    let root_sum = libm::sqrt(h1) + libm::sqrt(h2);
    let marginal_radius_m = 0.5 * (d / root_sum) * (d / root_sum); // (24)
    let marginal_loss = first_term(marginal_radius_m).max(0.0);
    Ok(SmoothEarthLoss {
        loss_db: (1.0 - clearance / required) * marginal_loss, // (25)
        line_of_sight: true,
    })
}

/// 20 log₁₀(E/E₀) of eq. (13) by the first residue-series term, with the
/// practical-unit forms (11a), (12a), (14a) and (15a) the Recommendation
/// gives: d in km, heights in m, aₑ in km, f in MHz.
pub(crate) fn first_term_field_db(
    d_m: f64,
    h1_m: f64,
    h2_m: f64,
    ae_m: f64,
    signal: &Signal,
    ground: Ground,
) -> f64 {
    let f_mhz = signal.frequency.mhz();
    let ae_km = ae_m * 1.0e-3;
    let k = surface_admittance(ae_km, f_mhz, signal.polarisation, ground);
    let k2 = k * k;
    let k4 = k2 * k2;
    let beta = (1.0 + 1.6 * k2 + 0.67 * k4) / (1.0 + 4.5 * k2 + 1.53 * k4); // (16)
    let x = 2.188 * beta * libm::cbrt(f_mhz) * libm::pow(ae_km, -2.0 / 3.0) * d_m * 1.0e-3; // (14a)
    let y_per_m = 9.575e-3 * beta * libm::pow(f_mhz, 2.0 / 3.0) * libm::pow(ae_km, -1.0 / 3.0); // (15a)
    let gain_floor = 2.0 + 20.0 * libm::log10(k);
    let height_gain = |h_m: f64| height_gain_db(beta * y_per_m * h_m).max(gain_floor);
    distance_term_db(x) + height_gain(h1_m) + height_gain(h2_m)
}

/// The normalised surface admittance K of eqs (11a) and (12a).
fn surface_admittance(ae_km: f64, f_mhz: f64, polarisation: Polarisation, ground: Ground) -> f64 {
    let eps = ground.relative_permittivity;
    let loss_term = 18_000.0 * ground.conductivity_s_m / f_mhz;
    let loss_term2 = loss_term * loss_term;
    let horizontal = 0.36
        * libm::cbrt(1.0 / (ae_km * f_mhz))
        * libm::pow((eps - 1.0) * (eps - 1.0) + loss_term2, -0.25);
    match polarisation {
        Polarisation::Horizontal => horizontal,
        Polarisation::Vertical => horizontal * libm::sqrt(eps * eps + loss_term2),
    }
}

/// The distance term F(X) of eqs (17a) and (17b), dB.
fn distance_term_db(x: f64) -> f64 {
    if x >= 1.6 {
        11.0 + 10.0 * libm::log10(x) - 17.6 * x
    } else {
        -20.0 * libm::log10(x) - 5.6488 * libm::pow(x, 1.425)
    }
}

/// The height-gain term G(Y) of eqs (18) and (18a), dB, from B = βY.
fn height_gain_db(b: f64) -> f64 {
    if b > 2.0 {
        17.6 * libm::sqrt(b - 1.1) - 5.0 * libm::log10(b - 1.1) - 8.0
    } else {
        20.0 * libm::log10(b + 0.1 * b * b * b)
    }
}
