//! The NRLMSISE-00 empirical model of the atmosphere.
//!
//! J. M. Picone, A. E. Hedin, D. P. Drob and A. C. Aikin, "NRLMSISE-00
//! empirical model of the atmosphere: Statistical comparisons and
//! scientific issues", J. Geophys. Res. 107(A12), 1468, 2002,
//! doi:10.1029/2002JA009430.
//!
//! The arithmetic follows the reference implementation term for term, in
//! its units (km, cm⁻³, g/cm³) and with its rounded constants, so that its
//! published test values reproduce; only the public result is SI. Where
//! the reference keeps values between calls, this evaluation starts each
//! call from the reference's initial state.

mod call;
mod coefficients;
mod globe;
mod globe_lower;
mod lower;
mod profile;
mod spline;
mod thermosphere;

use world_core::{DatasetId, Provenance};

use crate::{MsisInput, NumberDensities, Switches, ThermosphereError, ThermosphereState};

use call::Call;
use lower::{ZN2_TOP, below_mesopause};
use thermosphere::{AMU_G, gts7};

/// The reference implementation the parameters are transcribed from and
/// the arithmetic follows: the committed `data/nrlmsise-00_data.c` is its
/// data file, unmodified.
pub const NRLMSISE00: Provenance = Provenance {
    dataset: DatasetId {
        name: "NRLMSISE-00",
        edition: "NRLMSISE-00.DIST17 (Official Beta Release 2.0), as the C implementation at commit bc9a2fe of 2019-07-09",
    },
    publisher: "U.S. Naval Research Laboratory (Picone, Hedin, Drob); C implementation by Dominik Brodowski",
    source_url: "https://git.linta.de/~brodo/nrlmsise-00.git",
    source_file: "nrlmsise-00_data.c",
    source_sha256: "d0b3022f3c3e7ffdf703cc0e0d02339dfee01c0bf1520b29c68d5f4afd8784d5",
    terms: "public domain: the model is a work of the U.S. Government, and the C implementation was placed in the public domain by its author on that condition (statement of 2016-06-16, recorded with the copy in the U.S. Naval Research Laboratory's Tracker Component Library)",
};

/// The lowest geodetic altitude the model is fitted to, km.
const LOWEST_KM: f64 = 0.0;
/// Number density per cubic centimeter to per cubic meter.
const PER_CM3_TO_PER_M3: f64 = 1.0E6;

/// The model's result in the reference's order and units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reference {
    /// He, O, N2, O2, Ar number densities, cm⁻³; mass density, g/cm³; H,
    /// N and anomalous O number densities, cm⁻³.
    pub(crate) d: [f64; 9],
    /// Exospheric temperature and temperature at the altitude, K.
    pub(crate) t: [f64; 2],
}

/// The NRLMSISE-00 model with a choice of switches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Nrlmsise00 {
    switches: Switches,
}

impl Nrlmsise00 {
    /// The model as published: every effect on.
    pub const STANDARD: Self = Self::new(Switches::STANDARD);

    /// The model with `switches`.
    pub const fn new(switches: Switches) -> Self {
        Self { switches }
    }

    /// The switches the model evaluates with.
    pub const fn switches(&self) -> Switches {
        self.switches
    }

    /// The state of the atmosphere for `input`.
    ///
    /// One evaluation gives both of the reference's entry points: `GTD7`
    /// is [`ThermosphereState::mass_density_kg_m3`] and `GTD7D` is
    /// [`ThermosphereState::drag_mass_density_kg_m3`]; every other output
    /// is common to the two.
    ///
    /// # Errors
    ///
    /// [`ThermosphereError::InvalidInput`] for a non-finite or
    /// out-of-range input, [`ThermosphereError::BelowModel`] below the
    /// ellipsoid, [`ThermosphereError::MixingSwitchedOff`] below 72.5 km
    /// with [`crate::Effect::DiffusiveEquilibriumDeparture`] off, and
    /// [`ThermosphereError::NotFinite`] when the formulation overflows.
    pub fn evaluate(&self, input: &MsisInput) -> Result<ThermosphereState, ThermosphereError> {
        let Reference { d, t } = self.reference(input)?;
        let per_m3 = |cm3: f64| cm3 * PER_CM3_TO_PER_M3;
        // g/cm³ to kg/m³, in the two steps the reference takes.
        let kg_m3 = |g_cm3: f64| g_cm3 * PER_CM3_TO_PER_M3 / 1000.0;
        let drag = AMU_G
            * (4.0 * d[0]
                + 16.0 * d[1]
                + 28.0 * d[2]
                + 32.0 * d[3]
                + 40.0 * d[4]
                + d[6]
                + 14.0 * d[7]
                + 16.0 * d[8]);
        let state = ThermosphereState {
            number_density: NumberDensities {
                helium_m3: per_m3(d[0]),
                atomic_oxygen_m3: per_m3(d[1]),
                molecular_nitrogen_m3: per_m3(d[2]),
                molecular_oxygen_m3: per_m3(d[3]),
                argon_m3: per_m3(d[4]),
                atomic_hydrogen_m3: per_m3(d[6]),
                atomic_nitrogen_m3: per_m3(d[7]),
                anomalous_oxygen_m3: per_m3(d[8]),
            },
            mass_density_kg_m3: kg_m3(d[5]),
            drag_mass_density_kg_m3: kg_m3(drag),
            exospheric_temperature_k: t[0],
            temperature_k: t[1],
        };
        Ok(state)
    }

    /// The reference's `GTD7` for `input`, in its units.
    pub(crate) fn reference(&self, input: &MsisInput) -> Result<Reference, ThermosphereError> {
        input.validate()?;
        let call = Call::new(input, &self.switches);
        if call.alt < LOWEST_KM {
            return Err(ThermosphereError::BelowModel {
                altitude_km: call.alt,
                lowest_km: LOWEST_KM,
            });
        }
        // Below 72.5 km the thermospheric part is evaluated at 72.5 km and
        // continued downward.
        let upper = gts7(
            &call,
            if call.alt > ZN2_TOP {
                call.alt
            } else {
                ZN2_TOP
            },
        );
        let result = if call.alt >= ZN2_TOP {
            Reference {
                d: upper.d,
                t: [upper.tinf, upper.tz],
            }
        } else {
            let dm28 = upper.dm28.ok_or(ThermosphereError::MixingSwitchedOff {
                altitude_km: call.alt,
            })?;
            let (d, tz) = below_mesopause(&call, &upper, dm28);
            Reference {
                d,
                t: [upper.tinf, tz],
            }
        };
        if result.d.iter().chain(&result.t).all(|v| v.is_finite()) {
            Ok(result)
        } else {
            Err(ThermosphereError::NotFinite)
        }
    }
}

#[cfg(test)]
mod tests;
