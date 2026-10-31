//! Multi-altitude soundings and surface weather station observations.

use alloc::vec::Vec;
use core::fmt;
use libm::{exp, log, sqrt};
use world_atmosphere::{Altitude, AtmosphereError, AtmosphereState, Isa};
use world_core::{DecimalYear, GeodeticPosition, Ned, Provenance, ValidityWindow};

use crate::FlightCategory;

/// Specific gas constant for dry air, J/(kg·K).
const R_DRY_AIR: f64 = 287.052_87;
/// Ratio of specific heats for dry air.
const GAMMA_DRY_AIR: f64 = 1.4;

/// Speed of sound in dry air at static temperature `temperature_k` (kelvin), m/s.
pub fn dry_air_speed_of_sound_m_s(temperature_k: f64) -> f64 {
    if temperature_k <= 0.0 || !temperature_k.is_finite() {
        0.0
    } else {
        sqrt(GAMMA_DRY_AIR * R_DRY_AIR * temperature_k)
    }
}

/// Errors encountered when constructing or validating weather records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirmassError {
    /// A station code must contain 3 or 4 ASCII alphanumeric characters.
    InvalidStationCode,
    /// Sounding profiles require at least one finite altitude level.
    EmptySounding,
    /// One or more weather quantities were NaN or out of physical bounds.
    NonPhysicalValue,
    /// Store capacity limit was reached.
    CapacityExceeded,
}

impl fmt::Display for AirmassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidStationCode => "station code must be 3 or 4 ASCII alphanumeric bytes",
            Self::EmptySounding => "sounding profile must contain at least one valid level",
            Self::NonPhysicalValue => "weather observation contains non-finite or negative state",
            Self::CapacityExceeded => "airmass store capacity limit exceeded",
        })
    }
}

impl core::error::Error for AirmassError {}

/// Compact 4-character ICAO or WMO station identifier (padded with trailing space for 3-char IDs).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationId([u8; 4]);

impl StationId {
    /// Parses a 3- or 4-character ASCII alphanumeric station identifier, uppercasing ASCII letters.
    ///
    /// # Errors
    ///
    /// Returns [`AirmassError::InvalidStationCode`] if `code` is not 3 or 4 ASCII alphanumeric bytes.
    pub fn parse(code: &str) -> Result<Self, AirmassError> {
        let bytes = code.as_bytes();
        if bytes.len() != 3 && bytes.len() != 4 {
            return Err(AirmassError::InvalidStationCode);
        }
        let mut raw = [b' '; 4];
        for (dst, src) in raw.iter_mut().zip(bytes.iter()) {
            if !src.is_ascii_alphanumeric() {
                return Err(AirmassError::InvalidStationCode);
            }
            *dst = src.to_ascii_uppercase();
        }
        Ok(Self(raw))
    }

    /// Trimmed ASCII string view of the station identifier.
    pub fn as_str(&self) -> &str {
        let len = if self.0.get(3) == Some(&b' ') { 3 } else { 4 };
        core::str::from_utf8(self.0.get(..len).unwrap_or(&[])).unwrap_or_default()
    }
}

impl fmt::Debug for StationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StationId({:?})", self.as_str())
    }
}

impl fmt::Display for StationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One altitude level in an atmospheric sounding or winds/temperatures aloft profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AloftLevel {
    /// Altitude above mean sea level.
    pub altitude: Altitude,
    /// Static air temperature in kelvin.
    pub temperature_k: f64,
    /// Optional static pressure in pascals; when `None`, ISA pressure at `altitude` is used.
    pub pressure_pa: Option<f64>,
    /// Wind vector (direction the air moves toward) in local north-east-down, m/s.
    pub wind_ned_m_s: Ned,
    /// Peak gust increment above steady wind speed, m/s.
    pub gust_m_s: f64,
    /// Eddy dissipation rate $\varepsilon^{1/3}$ in $\text{m}^{2/3}/\text{s}$ (`0.0` for smooth air).
    pub turbulence_edr: f64,
}

/// Vertical sounding profile anchored to a location (or global when `location` is `None`).
#[derive(Clone, Debug, PartialEq)]
pub struct SoundingProfile {
    id: u64,
    location: Option<GeodeticPosition>,
    radius_m: f64,
    valid: ValidityWindow,
    provenance: Option<Provenance>,
    levels: Vec<AloftLevel>,
}

impl SoundingProfile {
    /// Builds a validated sounding sorted by ascending geopotential altitude.
    ///
    /// # Errors
    ///
    /// Returns [`AirmassError`] when `levels` is empty or contains non-physical temperatures/pressures.
    pub fn new(
        id: u64,
        location: Option<GeodeticPosition>,
        radius_m: f64,
        valid: ValidityWindow,
        provenance: Option<Provenance>,
        mut levels: Vec<AloftLevel>,
    ) -> Result<Self, AirmassError> {
        if levels.is_empty() {
            return Err(AirmassError::EmptySounding);
        }
        if !radius_m.is_finite() || radius_m < 0.0 {
            return Err(AirmassError::NonPhysicalValue);
        }
        if let Some(pos) = location {
            if !pos.is_well_formed() {
                return Err(AirmassError::NonPhysicalValue);
            }
        }
        for lvl in &levels {
            let h = lvl.altitude.geopotential_m();
            if !h.is_finite() || !lvl.temperature_k.is_finite() || lvl.temperature_k <= 0.0 {
                return Err(AirmassError::NonPhysicalValue);
            }
            if let Some(p) = lvl.pressure_pa {
                if !p.is_finite() || p <= 0.0 {
                    return Err(AirmassError::NonPhysicalValue);
                }
            }
        }
        levels.sort_by(|a, b| {
            a.altitude
                .geopotential_m()
                .total_cmp(&b.altitude.geopotential_m())
        });
        Ok(Self {
            id,
            location,
            radius_m,
            valid,
            provenance,
            levels,
        })
    }

    /// Producer-assigned sounding identifier.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Anchor location, or `None` for a global profile.
    pub const fn location(&self) -> Option<GeodeticPosition> {
        self.location
    }

    /// Horizontal validity radius around `location`, meters.
    pub const fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// Temporal validity window.
    pub const fn valid(&self) -> ValidityWindow {
        self.valid
    }

    /// Dataset provenance, if published.
    pub const fn provenance(&self) -> Option<Provenance> {
        self.provenance
    }

    /// Sorted vertical levels.
    pub fn levels(&self) -> &[AloftLevel] {
        &self.levels
    }

    /// Interpolates thermodynamic state, wind, gust and turbulence at `altitude`.
    pub fn interpolate(
        &self,
        altitude: Altitude,
    ) -> Result<(AtmosphereState, Ned, f64, f64), AtmosphereError> {
        let h = altitude.geopotential_m();
        let isa_state = Isa.at(altitude)?;
        let Some(first) = self.levels.first() else {
            return Ok((
                isa_state,
                Ned {
                    north_m: 0.0,
                    east_m: 0.0,
                    down_m: 0.0,
                },
                0.0,
                0.0,
            ));
        };
        let Some(last) = self.levels.last() else {
            return Ok((
                isa_state,
                Ned {
                    north_m: 0.0,
                    east_m: 0.0,
                    down_m: 0.0,
                },
                0.0,
                0.0,
            ));
        };
        if h <= first.altitude.geopotential_m() {
            return level_to_state(first, altitude, isa_state);
        }
        if h >= last.altitude.geopotential_m() {
            return level_to_state(last, altitude, isa_state);
        }
        for pair in self.levels.windows(2) {
            let (Some(lo), Some(hi)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            let h_lo = lo.altitude.geopotential_m();
            let h_hi = hi.altitude.geopotential_m();
            if h >= h_lo && h <= h_hi {
                let span = h_hi - h_lo;
                let t = if span.abs() <= 1e-9 {
                    0.0
                } else {
                    (h - h_lo) / span
                };
                return blend_levels(lo, hi, t, isa_state);
            }
        }
        level_to_state(last, altitude, isa_state)
    }
}

fn resolve_level_pressure(level: &AloftLevel) -> Result<f64, AtmosphereError> {
    match level.pressure_pa {
        Some(p) => Ok(p),
        None => Ok(Isa.at(level.altitude)?.pressure_pa),
    }
}

fn level_to_state(
    level: &AloftLevel,
    altitude: Altitude,
    isa_at_query: AtmosphereState,
) -> Result<(AtmosphereState, Ned, f64, f64), AtmosphereError> {
    let level_isa = Isa.at(level.altitude)?;
    let delta_t = level.temperature_k - level_isa.temperature_k;
    let temperature_k = (isa_at_query.temperature_k + delta_t).max(1.0);
    let pressure_pa = match level.pressure_pa {
        Some(p) if (altitude.geopotential_m() - level.altitude.geopotential_m()).abs() < 1.0 => p,
        Some(p) => p * (isa_at_query.pressure_pa / level_isa.pressure_pa),
        None => isa_at_query.pressure_pa,
    };
    let density_kg_m3 = pressure_pa / (R_DRY_AIR * temperature_k);
    Ok((
        AtmosphereState {
            temperature_k,
            pressure_pa,
            density_kg_m3,
        },
        level.wind_ned_m_s,
        level.gust_m_s,
        level.turbulence_edr,
    ))
}

fn blend_levels(
    lo: &AloftLevel,
    hi: &AloftLevel,
    t: f64,
    isa_at_query: AtmosphereState,
) -> Result<(AtmosphereState, Ned, f64, f64), AtmosphereError> {
    let lerp = |a: f64, b: f64| a + (b - a) * t;
    let temperature_k = lerp(lo.temperature_k, hi.temperature_k).max(1.0);
    let p_lo = resolve_level_pressure(lo)?;
    let p_hi = resolve_level_pressure(hi)?;
    let pressure_pa = if p_lo > 0.0 && p_hi > 0.0 {
        exp(lerp(log(p_lo), log(p_hi)))
    } else {
        isa_at_query.pressure_pa
    };
    let density_kg_m3 = pressure_pa / (R_DRY_AIR * temperature_k);
    let wind = Ned {
        north_m: lerp(lo.wind_ned_m_s.north_m, hi.wind_ned_m_s.north_m),
        east_m: lerp(lo.wind_ned_m_s.east_m, hi.wind_ned_m_s.east_m),
        down_m: lerp(lo.wind_ned_m_s.down_m, hi.wind_ned_m_s.down_m),
    };
    Ok((
        AtmosphereState {
            temperature_k,
            pressure_pa,
            density_kg_m3,
        },
        wind,
        lerp(lo.gust_m_s, hi.gust_m_s),
        lerp(lo.turbulence_edr, hi.turbulence_edr),
    ))
}

/// Surface weather report (METAR / SPECI / AWOS equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceObservation {
    /// Reporting station identifier.
    pub station: StationId,
    /// Station coordinates, when known.
    pub position: Option<GeodeticPosition>,
    /// Observation timestamp.
    pub observed_at: DecimalYear,
    /// Validity interval for operational display/use.
    pub valid: ValidityWindow,
    /// Surface wind vector (direction air moves toward), m/s.
    pub wind_ned_m_s: Ned,
    /// Peak surface gust increment above steady wind, m/s.
    pub gust_m_s: f64,
    /// Prevailing horizontal visibility, meters.
    pub visibility_m: f64,
    /// Lowest broken/overcast ceiling above ground level, meters.
    pub ceiling_agl_m: Option<f64>,
    /// Ambient air temperature, kelvin.
    pub temperature_k: f64,
    /// Dewpoint temperature, kelvin.
    pub dewpoint_k: f64,
    /// Altimeter setting (QNH), pascals.
    pub altimeter_pa: f64,
    /// Derived or reported flight category.
    pub flight_category: FlightCategory,
}
