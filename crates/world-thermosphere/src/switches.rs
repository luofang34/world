//! Which terms of a model are evaluated.

/// One switchable term of the MSIS formulation, numbered as the
/// reference implementation numbers its switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Effect {
    /// The F10.7 effect on the mean.
    SolarFluxMean = 1,
    /// The time-independent latitude structure.
    TimeIndependent = 2,
    /// The symmetrical annual variation.
    SymmetricalAnnual = 3,
    /// The symmetrical semiannual variation.
    SymmetricalSemiannual = 4,
    /// The asymmetrical annual variation.
    AsymmetricalAnnual = 5,
    /// The asymmetrical semiannual variation.
    AsymmetricalSemiannual = 6,
    /// The diurnal tide.
    Diurnal = 7,
    /// The semidiurnal tide.
    Semidiurnal = 8,
    /// Geomagnetic activity, from the daily Ap or the Ap history.
    GeomagneticActivity = 9,
    /// Every universal-time and longitude effect.
    UniversalTimeAndLongitude = 10,
    /// The longitudinal variation.
    Longitudinal = 11,
    /// The universal-time and mixed universal-time and longitude terms.
    UniversalTimeMixed = 12,
    /// The mixed geomagnetic, universal-time and longitude terms.
    GeomagneticMixed = 13,
    /// The terdiurnal tide.
    Terdiurnal = 14,
    /// Departures from diffusive equilibrium: turbopause mixing and the
    /// chemistry corrections.
    DiffusiveEquilibriumDeparture = 15,
    /// Every variation of the exospheric temperature.
    ExosphericTemperature = 16,
    /// Every variation of the temperature at the 120 km lower boundary.
    LowerBoundaryTemperature = 17,
    /// Every variation of the lower-thermosphere temperature nodes.
    LowerThermosphereTemperature = 18,
    /// Every variation of the temperature-gradient shape parameter.
    TemperatureGradient = 19,
    /// Every variation of the mesosphere temperature nodes.
    MesosphereTemperature = 20,
    /// Every variation of the densities at the lower boundary.
    LowerBoundaryDensity = 21,
    /// Every variation of the stratosphere and troposphere temperature
    /// nodes.
    StratosphereTemperature = 22,
    /// The turbopause scale-height variation. The reference lists the
    /// switch; no term of NRLMSISE-00 reads it.
    TurbopauseScaleHeight = 23,
}

impl Effect {
    /// Every effect, in the reference's switch order: entry `i` is the
    /// reference's switch `i + 1`.
    pub const ALL: [Self; 23] = [
        Self::SolarFluxMean,
        Self::TimeIndependent,
        Self::SymmetricalAnnual,
        Self::SymmetricalSemiannual,
        Self::AsymmetricalAnnual,
        Self::AsymmetricalSemiannual,
        Self::Diurnal,
        Self::Semidiurnal,
        Self::GeomagneticActivity,
        Self::UniversalTimeAndLongitude,
        Self::Longitudinal,
        Self::UniversalTimeMixed,
        Self::GeomagneticMixed,
        Self::Terdiurnal,
        Self::DiffusiveEquilibriumDeparture,
        Self::ExosphericTemperature,
        Self::LowerBoundaryTemperature,
        Self::LowerThermosphereTemperature,
        Self::TemperatureGradient,
        Self::MesosphereTemperature,
        Self::LowerBoundaryDensity,
        Self::StratosphereTemperature,
        Self::TurbopauseScaleHeight,
    ];

    /// The reference's number for the switch, 1 to 23.
    pub const fn number(self) -> u8 {
        self as u8
    }
}

/// The setting of one [`Effect`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    /// The term and its cross terms are omitted (reference value 0).
    Off,
    /// The term is evaluated (reference value 1).
    On,
    /// The term itself is omitted but the cross terms it scales in other
    /// effects are kept (reference value 2).
    CrossTermsOnly,
}

/// A setting for every [`Effect`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Switches {
    /// Indexed by the effect's number; entry 0 is not an effect.
    settings: [Switch; 24],
}

/// The switches as the factors the formulation multiplies by: `sw` scales
/// a term, `swc` its cross terms in other effects.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Factors {
    pub(crate) sw: [f64; 24],
    pub(crate) swc: [f64; 24],
}

impl Switches {
    /// Every effect on: the model as published.
    pub const STANDARD: Self = Self {
        settings: [Switch::On; 24],
    };

    /// These switches with `effect` set to `switch`.
    #[must_use]
    pub const fn with(mut self, effect: Effect, switch: Switch) -> Self {
        self.settings[effect as usize] = switch;
        self
    }

    /// The setting of `effect`.
    pub const fn get(&self, effect: Effect) -> Switch {
        self.settings[effect as usize]
    }

    /// The multiplying factors. With `ap_history` the geomagnetic term,
    /// when on, is flagged -1: the reference's marker for the Ap-history
    /// formulation.
    pub(crate) fn factors(&self, ap_history: bool) -> Factors {
        let mut factors = Factors {
            sw: [0.0; 24],
            swc: [0.0; 24],
        };
        for (index, setting) in self.settings.iter().enumerate() {
            let (sw, swc) = match setting {
                Switch::Off => (0.0, 0.0),
                Switch::On => (1.0, 1.0),
                Switch::CrossTermsOnly => (0.0, 1.0),
            };
            factors.sw[index] = sw;
            factors.swc[index] = swc;
        }
        let geomagnetic = Effect::GeomagneticActivity as usize;
        if ap_history && factors.sw[geomagnetic] == 1.0 {
            factors.sw[geomagnetic] = -1.0;
        }
        factors
    }
}

impl Default for Switches {
    fn default() -> Self {
        Self::STANDARD
    }
}

#[cfg(test)]
mod tests;
