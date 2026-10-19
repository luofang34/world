//! The ISO 2533 International Standard Atmosphere.

use libm::{exp, pow};

use crate::{Altitude, AtmosphereError, AtmosphereModel, AtmospherePoint, AtmosphereState};

/// Standard gravity, m/s² (ISO 2533).
const G0: f64 = 9.806_65;
/// Specific gas constant of dry air, J/(kg·K) (ISO 2533).
const R: f64 = 287.052_87;
/// Sea-level pressure, Pa.
const P0: f64 = 101_325.0;
/// The lowest geopotential altitude the standard tabulates, meters.
const LOWEST_M: f64 = -5_000.0;
/// The highest geopotential altitude the standard tabulates, meters.
const HIGHEST_M: f64 = 80_000.0;

/// `(base geopotential altitude m, base temperature K, lapse rate K/m)`
/// for each layer, from ISO 2533 table 4. The first layer extends below
/// sea level.
const LAYERS: [(f64, f64, f64); 7] = [
    (0.0, 288.15, -0.0065),
    (11_000.0, 216.65, 0.0),
    (20_000.0, 216.65, 0.001),
    (32_000.0, 228.65, 0.0028),
    (47_000.0, 270.65, 0.0),
    (51_000.0, 270.65, -0.0028),
    (71_000.0, 214.65, -0.002),
];

/// The International Standard Atmosphere (ISO 2533), from -5 km to 80 km
/// geopotential altitude. It varies with altitude only.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Isa;

impl Isa {
    /// The standard atmosphere at `altitude`.
    ///
    /// # Errors
    ///
    /// [`AtmosphereError::OutsideDomain`] outside -5 km..=80 km
    /// geopotential, or for a non-finite altitude.
    pub fn at(&self, altitude: Altitude) -> Result<AtmosphereState, AtmosphereError> {
        let h = altitude.geopotential_m();
        if !(LOWEST_M..=HIGHEST_M).contains(&h) {
            return Err(AtmosphereError::OutsideDomain {
                geopotential_m: h,
                lowest_m: LOWEST_M,
                highest_m: HIGHEST_M,
            });
        }
        let mut base_pressure = P0;
        let mut state = (LAYERS[0].1, P0);
        for (index, &(base, base_temperature, lapse)) in LAYERS.iter().enumerate() {
            let top = LAYERS.get(index + 1).map_or(HIGHEST_M, |next| next.0);
            let reach = h.min(top);
            state = layer(base_pressure, base, base_temperature, lapse, reach);
            if h <= top {
                break;
            }
            base_pressure = state.1;
        }
        let (temperature_k, pressure_pa) = state;
        Ok(AtmosphereState {
            temperature_k,
            pressure_pa,
            density_kg_m3: pressure_pa / (R * temperature_k),
        })
    }
}

/// Temperature and pressure at geopotential altitude `h` inside one layer.
fn layer(base_pressure: f64, base: f64, base_temperature: f64, lapse: f64, h: f64) -> (f64, f64) {
    if lapse == 0.0 {
        let pressure = base_pressure * exp(-G0 * (h - base) / (R * base_temperature));
        (base_temperature, pressure)
    } else {
        let temperature = base_temperature + lapse * (h - base);
        let pressure = base_pressure * pow(temperature / base_temperature, -G0 / (lapse * R));
        (temperature, pressure)
    }
}

impl AtmosphereModel for Isa {
    fn state(&self, point: AtmospherePoint) -> Result<AtmosphereState, AtmosphereError> {
        self.at(point.altitude)
    }
}

#[cfg(test)]
mod tests;
