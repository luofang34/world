//! The ITS Irregular Terrain Model (Longley–Rice), point-to-point mode.
//!
//! [`Itm`] predicts the basic transmission loss over a terrain profile
//! from 20 MHz to 20 GHz: line-of-sight two-ray loss over rough ground,
//! diffraction (knife-edge and smooth-Earth blend) and troposcatter,
//! joined into a reference attenuation, with time, location and situation
//! variability for one of seven radio climates.
//!
//! # Provenance and notice
//!
//! This module is a port of the NTIA/ITS reference implementation of
//! ITM, version 1.4 (<https://github.com/NTIA/itm>, commit
//! 62168e686e8af4a234cf3f8067d80b543a5d52ef; see
//! [`crate::sources::NTIA_ITM`]). That software was developed by
//! employees of the National Telecommunications and Information
//! Administration (NTIA), an agency of the Federal Government, and under
//! 17 U.S.C. 105 is not subject to copyright protection in the United
//! States; NTIA grants a worldwide royalty-free right to prepare and
//! distribute derivative works. NTIA provides it "AS IS", without warranty
//! of any kind, and it is not intended for any situation where a failure
//! could cause risk of injury or damage to property. Its creation is
//! NTIA's, acknowledged here as its licence asks.
//!
//! Changes, made 2026-11-24 as the licence asks to note: translated from
//! C++ to Rust; point-to-point time/location/situation and
//! confidence/reliability modes only (area mode is not ported); inputs
//! are typed ([`Climate`], [`VariabilityMode`], [`Signal`],
//! [`Percentage`]) instead of integer codes; the terrain profile is read
//! through [`TerrainProfile`] and must be equally spaced instead of being
//! passed as a `pfl` array; errors are [`PropagationError`] values and
//! warnings an [`ItmWarnings`] set; elementary functions come from `libm`.
//! The numerical method, its constants and its order of operations are
//! unchanged.

mod complex;
mod diffraction;
mod line_of_sight;
mod reference;
mod terrain;
mod troposcatter;
mod variability;
mod warnings;

pub use warnings::ItmWarnings;

use crate::error::{PropagationError, finite};
use crate::ground::Ground;
use crate::loss::{Loss, PathLoss, Signal};
use crate::profile::TerrainProfile;
use crate::units::{Percentage, Polarisation};
use complex::Complex;
use terrain::Pfl;
use variability::VariabilityInputs;

/// The seven radio climates of the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Climate {
    /// Equatorial (Congo).
    Equatorial,
    /// Continental subtropical (Sudan).
    ContinentalSubtropical,
    /// Maritime subtropical (west coast of Africa).
    MaritimeSubtropical,
    /// Desert (Sahara).
    Desert,
    /// Continental temperate.
    ContinentalTemperate,
    /// Maritime temperate over land (United Kingdom).
    MaritimeTemperateOverLand,
    /// Maritime temperate over sea.
    MaritimeTemperateOverSea,
}

impl Climate {
    /// Zero-based column of the climate's constants.
    fn index(self) -> usize {
        match self {
            Self::Equatorial => 0,
            Self::ContinentalSubtropical => 1,
            Self::MaritimeSubtropical => 2,
            Self::Desert => 3,
            Self::ContinentalTemperate => 4,
            Self::MaritimeTemperateOverLand => 5,
            Self::MaritimeTemperateOverSea => 6,
        }
    }
}

/// How time, location and situation variability combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VariabilityMode {
    /// One message: all three are folded into situation (confidence).
    SingleMessage,
    /// Accidental: location is folded into situation.
    Accidental,
    /// Mobile: location is folded into time.
    Mobile,
    /// Broadcast: time and location are kept apart.
    Broadcast,
}

/// The model's fixed parameters for a link.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Itm {
    climate: Climate,
    refractivity_n: f64,
    ground: Ground,
    mode: VariabilityMode,
    eliminate_location: bool,
    eliminate_situation: bool,
}

impl Itm {
    /// The model for `climate`, sea-level surface refractivity
    /// `refractivity_n` (N-units, 250 to 400), `ground` constants and
    /// variability `mode`.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] for a refractivity outside
    /// 250 to 400 N-units, a relative permittivity below 1 or a
    /// non-positive conductivity.
    pub fn new(
        climate: Climate,
        refractivity_n: f64,
        ground: Ground,
        mode: VariabilityMode,
    ) -> Result<Self, PropagationError> {
        let invalid = |name, value| Err(PropagationError::InvalidParameter { name, value });
        if !(250.0..=400.0).contains(&refractivity_n) {
            return invalid("refractivity_n", refractivity_n);
        }
        let eps = ground.relative_permittivity;
        if !(1.0..).contains(&eps) || !eps.is_finite() {
            return invalid("relative_permittivity", eps);
        }
        let sigma = ground.conductivity_s_m;
        if sigma.is_nan() || sigma <= 0.0 || !sigma.is_finite() {
            return invalid("conductivity_s_m", sigma);
        }
        Ok(Self {
            climate,
            refractivity_n,
            ground,
            mode,
            eliminate_location: false,
            eliminate_situation: false,
        })
    }

    /// Drops location variability, as when the receiver's location is
    /// known (the reference code's `mdvar` + 10).
    pub fn without_location_variability(mut self) -> Self {
        self.eliminate_location = true;
        self
    }

    /// Drops direct situation variability, as interference studies should
    /// (the reference code's `mdvar` + 20).
    pub fn without_situation_variability(mut self) -> Self {
        self.eliminate_situation = true;
        self
    }
}

/// A point-to-point path over an equally spaced terrain profile, with the
/// quantiles the loss is not exceeded for.
#[derive(Clone, Copy, Debug)]
pub struct ItmPath<'a, P: ?Sized> {
    /// The terrain, first sample under the transmitter, last under the
    /// receiver, all at the same spacing.
    pub profile: &'a P,
    /// Transmitter structural height above ground, m (0.5 to 3000).
    pub tx_height_m: f64,
    /// Receiver structural height above ground, m (0.5 to 3000).
    pub rx_height_m: f64,
    /// Percentage of time.
    pub time: Percentage,
    /// Percentage of locations.
    pub location: Percentage,
    /// Percentage of situations.
    pub situation: Percentage,
}

impl<'a, P: ?Sized> ItmPath<'a, P> {
    /// A path queried by confidence and reliability, as the reference
    /// code's CR entry points map them: reliability is time, location is
    /// the median, confidence is situation.
    pub fn confidence_reliability(
        profile: &'a P,
        tx_height_m: f64,
        rx_height_m: f64,
        confidence: Percentage,
        reliability: Percentage,
    ) -> Self {
        let median = Percentage::new(50.0).unwrap_or(reliability);
        Self {
            profile,
            tx_height_m,
            rx_height_m,
            time: reliability,
            location: median,
            situation: confidence,
        }
    }
}

/// The mechanism that sets the reference attenuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropagationMode {
    /// Within the smooth-Earth line-of-sight distance.
    LineOfSight,
    /// Beyond it, on the diffraction line.
    Diffraction,
    /// Far enough beyond it that forward scatter dominates.
    Troposcatter,
}

/// The model's answer and the intermediate values the reference code
/// reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItmLoss {
    /// Basic transmission loss A, dB: free space plus [`Self::loss_db`].
    pub basic_transmission_loss_db: f64,
    /// ITM's free-space loss 32.45 + 20 log f(MHz) + 20 log d(km), dB.
    pub free_space_loss_db: f64,
    /// Reference (median) attenuation relative to free space, dB.
    pub reference_attenuation_db: f64,
    /// The mechanism that set the reference attenuation.
    pub mode: PropagationMode,
    /// Terminal horizon elevation angles (tx, rx), rad.
    pub horizon_angle_rad: [f64; 2],
    /// Terminal horizon distances (tx, rx), m.
    pub horizon_distance_m: [f64; 2],
    /// Terminal effective heights (tx, rx), m.
    pub effective_height_m: [f64; 2],
    /// Surface refractivity at the path's mean height, N-units.
    pub surface_refractivity_n: f64,
    /// Terrain irregularity Δh, m.
    pub terrain_irregularity_m: f64,
    /// Path length, m.
    pub distance_m: f64,
    /// Inputs or results near the edges of the model's validity.
    pub warnings: ItmWarnings,
}

impl Loss for ItmLoss {
    /// Attenuation relative to free space at the requested quantiles, dB.
    fn loss_db(&self) -> f64 {
        self.basic_transmission_loss_db - self.free_space_loss_db
    }
}

impl<P: TerrainProfile + ?Sized> PathLoss<ItmPath<'_, P>> for Itm {
    type Output = ItmLoss;

    fn path_loss(
        &self,
        geometry: &ItmPath<'_, P>,
        signal: &Signal,
    ) -> Result<ItmLoss, PropagationError> {
        let mut warnings = ItmWarnings::empty();
        let h_m = [geometry.tx_height_m, geometry.rx_height_m];
        let f_mhz = check_inputs(h_m, signal, &mut warnings)?;
        let pfl = Pfl::new(geometry.profile)?;
        let h_sys = terrain::system_height_m(&pfl);
        let (z_g, gamma_e, n_s) = self.initialise(f_mhz, h_sys, signal.polarisation);
        let path = terrain::quick_pfl(&pfl, gamma_e, h_m);
        let (a_ref, mode) =
            reference::reference_attenuation(&path, h_m, f_mhz, z_g, gamma_e, n_s, &mut warnings)?;
        let a_fs = 32.45 + 20.0 * libm::log10(f_mhz) + 20.0 * libm::log10(path.d_m / 1000.0);
        let quantiles =
            [geometry.time, geometry.location, geometry.situation].map(Percentage::fraction);
        let inputs = VariabilityInputs {
            quantiles,
            h_e_m: path.h_e_m,
            delta_h_m: path.delta_h_m,
            f_mhz,
            d_m: path.d_m,
            climate: self.climate,
            mode: self.mode,
            eliminate_location: self.eliminate_location,
            eliminate_situation: self.eliminate_situation,
        };
        let (attenuation, extreme) = variability::variability(a_ref, &inputs);
        if extreme {
            warnings.insert(ItmWarnings::EXTREME_VARIABILITIES);
        }
        let loss = finite("basic_transmission_loss_db", attenuation + a_fs)?;
        Ok(ItmLoss {
            basic_transmission_loss_db: loss,
            free_space_loss_db: a_fs,
            reference_attenuation_db: a_ref,
            mode,
            horizon_angle_rad: path.theta_hzn,
            horizon_distance_m: path.d_hzn_m,
            effective_height_m: path.h_e_m,
            surface_refractivity_n: n_s,
            terrain_irregularity_m: path.delta_h_m,
            distance_m: path.d_m,
            warnings,
        })
    }
}

impl Itm {
    /// Ground impedance, effective-Earth curvature and surface
    /// refractivity ([TN101 eqs 4.3, 4.4]; `InitializePointToPoint`).
    fn initialise(
        &self,
        f_mhz: f64,
        h_sys_m: f64,
        polarisation: Polarisation,
    ) -> (Complex, f64, f64) {
        // Curvature of the actual Earth, about 1 / 6370 km.
        const GAMMA_A: f64 = 157.0e-9;
        let n_s = if h_sys_m == 0.0 {
            self.refractivity_n
        } else {
            self.refractivity_n * libm::exp(-h_sys_m / 9460.0)
        };
        let gamma_e = GAMMA_A * (1.0 - 0.04665 * libm::exp(n_s / 179.3));
        let ep_r = Complex::new(
            self.ground.relative_permittivity,
            18_000.0 * self.ground.conductivity_s_m / f_mhz,
        );
        let horizontal = (ep_r - Complex::new(1.0, 0.0)).sqrt();
        let z_g = match polarisation {
            Polarisation::Horizontal => horizontal,
            Polarisation::Vertical => horizontal / ep_r,
        };
        (z_g, gamma_e, n_s)
    }
}

/// Terminal heights and frequency against the model's limits
/// (`ValidateInputs`); returns the frequency in MHz.
fn check_inputs(
    h_m: [f64; 2],
    signal: &Signal,
    warnings: &mut ItmWarnings,
) -> Result<f64, PropagationError> {
    let names = ["tx_height_m", "rx_height_m"];
    let flags = [
        ItmWarnings::TX_TERMINAL_HEIGHT,
        ItmWarnings::RX_TERMINAL_HEIGHT,
    ];
    for ((&h, name), flag) in h_m.iter().zip(names).zip(flags) {
        if !(1.0..=1000.0).contains(&h) {
            warnings.insert(flag);
        }
        if !(0.5..=3000.0).contains(&h) {
            return Err(PropagationError::InvalidParameter { name, value: h });
        }
    }
    let f_mhz = signal.frequency.mhz();
    if !(40.0..=10_000.0).contains(&f_mhz) {
        warnings.insert(ItmWarnings::FREQUENCY);
    }
    if !(20.0..=20_000.0).contains(&f_mhz) {
        return Err(PropagationError::FrequencyOutOfRange {
            frequency_hz: signal.frequency.hz(),
            min_hz: 20.0e6,
            max_hz: 20.0e9,
        });
    }
    Ok(f_mhz)
}

#[cfg(test)]
mod tests;
