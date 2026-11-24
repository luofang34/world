//! Value-typed single-writer model of the air aloft implementing [`AtmosphereModel`].

use alloc::vec::Vec;
use world_atmosphere::{AtmosphereError, AtmosphereModel, AtmospherePoint, AtmosphereState, Isa};
use world_core::{DecimalYear, GeodeticPosition, Ned};

use crate::{
    AloftError, CloudLayer, SoundingProfile, dry_air_speed_of_sound_m_s, horizontal_distance_m,
};

/// Behavior when no active sounding covers a queried point and epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Return standard ISO 2533 [`Isa`] thermodynamics with still air.
    #[default]
    StandardIsa,
}

/// Capacity bounds and fallback policy for an [`AloftModel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AloftConfig {
    /// Maximum held vertical sounding profiles.
    pub max_soundings: usize,
    /// Maximum held regional/global cloud layers.
    pub max_cloud_layers: usize,
    /// Fallback policy outside sounding coverage.
    pub fallback: FallbackPolicy,
}

impl Default for AloftConfig {
    fn default() -> Self {
        Self {
            max_soundings: 256,
            max_cloud_layers: 256,
            fallback: FallbackPolicy::StandardIsa,
        }
    }
}

/// Full state of the air at a point and instant: thermodynamics, acoustics, wind and turbulence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AloftSample {
    /// Static temperature, pressure and density.
    pub state: AtmosphereState,
    /// Dry-air speed of sound, m/s.
    pub speed_of_sound_m_s: f64,
    /// Wind vector (where the air moves toward) in local north-east-down, m/s.
    pub wind_ned_m_s: Ned,
    /// Peak gust increment above steady wind speed, m/s.
    pub gust_m_s: f64,
    /// Eddy dissipation rate $\varepsilon^{1/3}$ in $\text{m}^{2/3}/\text{s}$.
    pub turbulence_edr: f64,
}

/// Seam for models supplying both thermodynamic state and three-dimensional wind.
pub trait WindModel: AtmosphereModel {
    /// Evaluates the full state of the air and its wind at `point`.
    ///
    /// # Errors
    ///
    /// Returns [`AtmosphereError::OutsideDomain`] when `point.altitude` lies outside model bounds.
    fn sample(&self, point: AtmospherePoint) -> Result<AloftSample, AtmosphereError>;
}

/// Single-writer, value-typed physical model of the air: sounding profiles
/// and cloud layers.
///
/// This is a model of the atmosphere, not a store of weather products:
/// reports, advisories and their lifecycle belong to Airmass. Separate
/// instances represent separate states of the air (for example, a
/// simulator's physical truth and a vehicle's forecast of it).
#[derive(Clone, Debug, Default)]
pub struct AloftModel {
    config: AloftConfig,
    revision: u64,
    soundings: Vec<SoundingProfile>,
    cloud_layers: Vec<CloudLayer>,
}

impl AloftModel {
    /// Creates an empty model with `config`.
    pub fn with_config(config: AloftConfig) -> Self {
        Self {
            config,
            revision: 0,
            soundings: Vec::new(),
            cloud_layers: Vec::new(),
        }
    }

    /// Monotonic model revision incremented on every state mutation.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Inserts or replaces a [`SoundingProfile`] with the same `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AloftError::CapacityExceeded`] if adding a new sounding exceeds `max_soundings`.
    pub fn upsert_sounding(&mut self, profile: SoundingProfile) -> Result<(), AloftError> {
        if let Some(slot) = self.soundings.iter_mut().find(|s| s.id() == profile.id()) {
            *slot = profile;
            self.bump();
            return Ok(());
        }
        if self.soundings.len() >= self.config.max_soundings {
            return Err(AloftError::CapacityExceeded);
        }
        self.soundings.push(profile);
        self.bump();
        Ok(())
    }

    /// Removes the sounding with `id`, returning whether one was held.
    pub fn remove_sounding(&mut self, id: u64) -> bool {
        let before = self.soundings.len();
        self.soundings.retain(|sounding| sounding.id() != id);
        let removed = self.soundings.len() != before;
        if removed {
            self.bump();
        }
        removed
    }

    /// The held sounding with `id`.
    pub fn sounding(&self, id: u64) -> Option<&SoundingProfile> {
        self.soundings.iter().find(|sounding| sounding.id() == id)
    }

    /// Inserts or replaces a [`CloudLayer`] by `layer.id`.
    ///
    /// # Errors
    ///
    /// Returns [`AloftError::CapacityExceeded`] when `max_cloud_layers` would be exceeded.
    pub fn upsert_cloud_layer(&mut self, layer: CloudLayer) -> Result<(), AloftError> {
        if let Some(slot) = self.cloud_layers.iter_mut().find(|c| c.id == layer.id) {
            *slot = layer;
            self.bump();
            return Ok(());
        }
        if self.cloud_layers.len() >= self.config.max_cloud_layers {
            return Err(AloftError::CapacityExceeded);
        }
        self.cloud_layers.push(layer);
        self.bump();
        Ok(())
    }

    /// Removes all soundings and cloud layers whose `valid.until` precedes `now`.
    pub fn prune_expired(&mut self, now: DecimalYear) -> usize {
        let before = self.soundings.len() + self.cloud_layers.len();
        let keep = |until: DecimalYear| until.value() >= now.value();
        self.soundings.retain(|s| keep(s.valid().until));
        self.cloud_layers.retain(|c| keep(c.valid.until));
        let removed = before.saturating_sub(self.soundings.len() + self.cloud_layers.len());
        if removed > 0 {
            self.bump();
        }
        removed
    }

    /// Returns the highest-coverage cloud layer covering `position` at `at`.
    pub fn cloud_at(&self, position: GeodeticPosition, at: DecimalYear) -> Option<CloudLayer> {
        self.cloud_layers
            .iter()
            .filter(|layer| {
                layer.valid.contains(at)
                    && match layer.center {
                        None => true,
                        Some(center) => horizontal_distance_m(center, position)
                            .is_some_and(|d| d <= layer.radius_m),
                    }
            })
            .copied()
            .max_by(|a, b| a.coverage.total_cmp(&b.coverage))
    }

    fn select_sounding(&self, point: AtmospherePoint) -> Option<&SoundingProfile> {
        let query_pos = GeodeticPosition::wgs84(point.latitude_deg, point.longitude_deg, 0.0);
        let mut best_local: Option<(&SoundingProfile, f64)> = None;
        let mut best_global: Option<&SoundingProfile> = None;

        for profile in &self.soundings {
            if !profile.valid().contains(point.at) {
                continue;
            }
            match profile.location() {
                Some(loc) => {
                    if let Some(dist) = horizontal_distance_m(loc, query_pos) {
                        if dist <= profile.radius_m() {
                            let replace = match best_local {
                                None => true,
                                Some((_, best_d)) => dist < best_d,
                            };
                            if replace {
                                best_local = Some((profile, dist));
                            }
                        }
                    }
                }
                None => {
                    best_global = Some(profile);
                }
            }
        }
        best_local.map(|(p, _)| p).or(best_global)
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

impl AtmosphereModel for AloftModel {
    fn state(&self, point: AtmospherePoint) -> Result<AtmosphereState, AtmosphereError> {
        Ok(self.sample(point)?.state)
    }
}

impl WindModel for AloftModel {
    fn sample(&self, point: AtmospherePoint) -> Result<AloftSample, AtmosphereError> {
        if let Some(sounding) = self.select_sounding(point) {
            let (state, wind_ned_m_s, gust_m_s, turbulence_edr) =
                sounding.interpolate(point.altitude)?;
            return Ok(AloftSample {
                state,
                speed_of_sound_m_s: dry_air_speed_of_sound_m_s(state.temperature_k),
                wind_ned_m_s,
                gust_m_s,
                turbulence_edr,
            });
        }
        let state = Isa.at(point.altitude)?;
        Ok(AloftSample {
            state,
            speed_of_sound_m_s: dry_air_speed_of_sound_m_s(state.temperature_k),
            wind_ned_m_s: Ned {
                north_m: 0.0,
                east_m: 0.0,
                down_m: 0.0,
            },
            gust_m_s: 0.0,
            turbulence_edr: 0.0,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
