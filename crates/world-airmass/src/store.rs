//! Value-typed single-writer airmass store implementing [`AtmosphereModel`].

use alloc::vec::Vec;
use world_atmosphere::{AtmosphereError, AtmosphereModel, AtmospherePoint, AtmosphereState, Isa};
use world_core::{DecimalYear, GeodeticPosition, Ned};

use crate::{
    AirmassError, CloudLayer, SoundingProfile, StationId, SurfaceObservation, WeatherHazard,
    dry_air_speed_of_sound_m_s, horizontal_distance_m,
};

/// Behavior when no active sounding covers a queried point and epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Return standard ISO 2533 [`Isa`] thermodynamics with still air.
    #[default]
    StandardIsa,
}

/// Capacity bounds and fallback policy for an [`AirmassStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirmassConfig {
    /// Maximum stored vertical sounding profiles.
    pub max_soundings: usize,
    /// Maximum stored surface station observations.
    pub max_observations: usize,
    /// Maximum stored regional/global cloud layers.
    pub max_cloud_layers: usize,
    /// Maximum stored weather hazard volumes.
    pub max_hazards: usize,
    /// Fallback policy outside sounding coverage.
    pub fallback: FallbackPolicy,
}

impl Default for AirmassConfig {
    fn default() -> Self {
        Self {
            max_soundings: 256,
            max_observations: 1_024,
            max_cloud_layers: 256,
            max_hazards: 512,
            fallback: FallbackPolicy::StandardIsa,
        }
    }
}

/// Full airmass state at a point and instant: thermodynamics, acoustics, wind and turbulence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirmassSample {
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
    /// Evaluates full airmass state and wind at `point`.
    ///
    /// # Errors
    ///
    /// Returns [`AtmosphereError::OutsideDomain`] when `point.altitude` lies outside model bounds.
    fn sample(&self, point: AtmospherePoint) -> Result<AirmassSample, AtmosphereError>;
}

/// Single-writer, value-typed store of perishable weather products.
///
/// Separate instances represent separate knowledge states (for example, simulator
/// physical truth vs. an aircraft's delayed FIS-B/datalink weather belief).
#[derive(Clone, Debug, Default)]
pub struct AirmassStore {
    config: AirmassConfig,
    revision: u64,
    soundings: Vec<SoundingProfile>,
    observations: Vec<SurfaceObservation>,
    cloud_layers: Vec<CloudLayer>,
    hazards: Vec<WeatherHazard>,
}

impl AirmassStore {
    /// Creates an empty store with `config`.
    pub fn with_config(config: AirmassConfig) -> Self {
        Self {
            config,
            revision: 0,
            soundings: Vec::new(),
            observations: Vec::new(),
            cloud_layers: Vec::new(),
            hazards: Vec::new(),
        }
    }

    /// Monotonic store revision incremented on every state mutation.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Inserts or replaces a [`SoundingProfile`] with the same `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AirmassError::CapacityExceeded`] if adding a new sounding exceeds `max_soundings`.
    pub fn upsert_sounding(&mut self, profile: SoundingProfile) -> Result<(), AirmassError> {
        if let Some(slot) = self.soundings.iter_mut().find(|s| s.id() == profile.id()) {
            *slot = profile;
            self.bump();
            return Ok(());
        }
        if self.soundings.len() >= self.config.max_soundings {
            return Err(AirmassError::CapacityExceeded);
        }
        self.soundings.push(profile);
        self.bump();
        Ok(())
    }

    /// Inserts or replaces the latest [`SurfaceObservation`] for `obs.station` if not older.
    ///
    /// # Errors
    ///
    /// Returns [`AirmassError::CapacityExceeded`] when `max_observations` would be exceeded.
    pub fn upsert_observation(&mut self, obs: SurfaceObservation) -> Result<bool, AirmassError> {
        if let Some(slot) = self
            .observations
            .iter_mut()
            .find(|existing| existing.station == obs.station)
        {
            if obs.observed_at.value() < slot.observed_at.value() {
                return Ok(false);
            }
            *slot = obs;
            self.bump();
            return Ok(true);
        }
        if self.observations.len() >= self.config.max_observations {
            return Err(AirmassError::CapacityExceeded);
        }
        self.observations.push(obs);
        self.bump();
        Ok(true)
    }

    /// Inserts or replaces a [`CloudLayer`] by `layer.id`.
    ///
    /// # Errors
    ///
    /// Returns [`AirmassError::CapacityExceeded`] when `max_cloud_layers` would be exceeded.
    pub fn upsert_cloud_layer(&mut self, layer: CloudLayer) -> Result<(), AirmassError> {
        if let Some(slot) = self.cloud_layers.iter_mut().find(|c| c.id == layer.id) {
            *slot = layer;
            self.bump();
            return Ok(());
        }
        if self.cloud_layers.len() >= self.config.max_cloud_layers {
            return Err(AirmassError::CapacityExceeded);
        }
        self.cloud_layers.push(layer);
        self.bump();
        Ok(())
    }

    /// Inserts or replaces a [`WeatherHazard`] by `hazard.id`.
    ///
    /// # Errors
    ///
    /// Returns [`AirmassError::CapacityExceeded`] when `max_hazards` would be exceeded.
    pub fn upsert_hazard(&mut self, hazard: WeatherHazard) -> Result<(), AirmassError> {
        if let Some(slot) = self.hazards.iter_mut().find(|h| h.id == hazard.id) {
            *slot = hazard;
            self.bump();
            return Ok(());
        }
        if self.hazards.len() >= self.config.max_hazards {
            return Err(AirmassError::CapacityExceeded);
        }
        self.hazards.push(hazard);
        self.bump();
        Ok(())
    }

    /// Removes all weather items whose `valid.until` precedes `now`.
    pub fn prune_expired(&mut self, now: DecimalYear) -> usize {
        let before = self.soundings.len()
            + self.observations.len()
            + self.cloud_layers.len()
            + self.hazards.len();
        let keep = |until: DecimalYear| until.value() >= now.value();
        self.soundings.retain(|s| keep(s.valid().until));
        self.observations.retain(|o| keep(o.valid.until));
        self.cloud_layers.retain(|c| keep(c.valid.until));
        self.hazards.retain(|h| keep(h.valid.until));
        let after = self.soundings.len()
            + self.observations.len()
            + self.cloud_layers.len()
            + self.hazards.len();
        let removed = before.saturating_sub(after);
        if removed > 0 {
            self.bump();
        }
        removed
    }

    /// Looks up the latest active observation for `station` at `at`.
    pub fn observation(&self, station: StationId, at: DecimalYear) -> Option<&SurfaceObservation> {
        self.observations
            .iter()
            .find(|obs| obs.station == station && obs.valid.contains(at))
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

    /// Returns all active [`WeatherHazard`] volumes containing `position` and `altitude_msl_m` at `at`.
    pub fn active_hazards_at(
        &self,
        position: GeodeticPosition,
        altitude_msl_m: f64,
        at: DecimalYear,
    ) -> Vec<&WeatherHazard> {
        self.hazards
            .iter()
            .filter(|hazard| hazard.contains(position, altitude_msl_m, at))
            .collect()
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

impl AtmosphereModel for AirmassStore {
    fn state(&self, point: AtmospherePoint) -> Result<AtmosphereState, AtmosphereError> {
        Ok(self.sample(point)?.state)
    }
}

impl WindModel for AirmassStore {
    fn sample(&self, point: AtmospherePoint) -> Result<AirmassSample, AtmosphereError> {
        if let Some(sounding) = self.select_sounding(point) {
            let (state, wind_ned_m_s, gust_m_s, turbulence_edr) =
                sounding.interpolate(point.altitude)?;
            return Ok(AirmassSample {
                state,
                speed_of_sound_m_s: dry_air_speed_of_sound_m_s(state.temperature_k),
                wind_ned_m_s,
                gust_m_s,
                turbulence_edr,
            });
        }
        let state = Isa.at(point.altitude)?;
        Ok(AirmassSample {
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
