//! Single-writer value-typed surveillance track store.

use alloc::vec::Vec;
use core::fmt;
use world_core::{GeodeticPosition, Ned};

use crate::{
    ProjectionBounds, RelativeTarget, TrackId, TrackKey, TrackObservation, TrackPhase,
    TrackSnapshot, project_geodetic, relative_target,
};

/// Freshness, coasting and expiration windows in monotonic microseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreshnessPolicy {
    /// Window during which an authoritative source blocks lower-ranked overwrites.
    pub fresh_window_us: u64,
    /// Silence threshold after which an `Active` track transitions to `Coasting`.
    pub coast_after_us: u64,
    /// Silence threshold after which a track is removed as expired.
    pub expire_after_us: u64,
}

impl Default for FreshnessPolicy {
    fn default() -> Self {
        Self {
            fresh_window_us: 15_000_000,
            coast_after_us: 15_000_000,
            expire_after_us: 60_000_000,
        }
    }
}

/// Configuration of a [`SurveillanceStore`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurveillanceConfig {
    /// Maximum concurrent tracks retained in the store.
    pub max_tracks: usize,
    /// Freshness, coasting and expiration timings.
    pub freshness: FreshnessPolicy,
    /// Dead-reckoning extrapolation bounds.
    pub projection: ProjectionBounds,
}

impl Default for SurveillanceConfig {
    fn default() -> Self {
        Self {
            max_tracks: 1_024,
            freshness: FreshnessPolicy::default(),
            projection: ProjectionBounds::default(),
        }
    }
}

/// Result of submitting a [`TrackObservation`] to [`SurveillanceStore::ingest`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IngestOutcome {
    /// A new track was allocated.
    Created(TrackId),
    /// An existing track's kinematic state was updated.
    Updated(TrackId),
    /// Observation was ignored because a fresher or higher-authority fix already governs the track.
    IgnoredLowerAuthority(TrackId),
}

/// Why an observation could not be ingested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurveillanceError {
    /// Geodetic coordinates or velocity components were NaN or out of bounds.
    MalformedKinematics,
    /// Adding a new track would exceed `max_tracks`.
    CapacityExceeded,
}

impl fmt::Display for SurveillanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MalformedKinematics => "track observation contains malformed coordinates",
            Self::CapacityExceeded => "surveillance store track capacity exceeded",
        })
    }
}

impl core::error::Error for SurveillanceError {}

/// Value-typed single-writer multi-source surveillance store.
#[derive(Clone, Debug, Default)]
pub struct SurveillanceStore {
    config: SurveillanceConfig,
    revision: u64,
    next_id: u64,
    ownship_keys: Vec<TrackKey>,
    tracks: Vec<TrackSnapshot>,
}

impl SurveillanceStore {
    /// Creates an empty store with `config`.
    pub fn with_config(config: SurveillanceConfig) -> Self {
        Self {
            config,
            revision: 0,
            next_id: 1,
            ownship_keys: Vec::new(),
            tracks: Vec::new(),
        }
    }

    /// Monotonic snapshot revision incremented on every store change.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Current tracks in deterministic insertion/ID order.
    pub fn tracks(&self) -> &[TrackSnapshot] {
        &self.tracks
    }

    /// Looks up a track by [`TrackId`].
    pub fn track(&self, id: TrackId) -> Option<&TrackSnapshot> {
        self.tracks.iter().find(|t| t.id == id)
    }

    /// Looks up a track by external [`TrackKey`].
    pub fn track_by_key(&self, key: TrackKey) -> Option<&TrackSnapshot> {
        self.tracks.iter().find(|t| t.key == key)
    }

    /// Configures ownship keys so matching tracks are labeled `ownship_shadow = true`.
    pub fn set_ownship_keys(&mut self, keys: &[TrackKey]) {
        self.ownship_keys.clear();
        self.ownship_keys.extend_from_slice(keys);
        for track in &mut self.tracks {
            track.ownship_shadow = self.ownship_keys.contains(&track.key);
        }
        self.bump();
    }

    /// Ingests one [`TrackObservation`] evaluated at caller monotonic time `now_us`.
    ///
    /// # Errors
    ///
    /// Returns [`SurveillanceError`] for malformed coordinates or exceeded capacity.
    pub fn ingest(
        &mut self,
        obs: TrackObservation,
        now_us: u64,
    ) -> Result<IngestOutcome, SurveillanceError> {
        if !obs.position.is_well_formed()
            || !obs.velocity_ned_m_s.north_m.is_finite()
            || !obs.velocity_ned_m_s.east_m.is_finite()
            || !obs.velocity_ned_m_s.down_m.is_finite()
        {
            return Err(SurveillanceError::MalformedKinematics);
        }
        let is_ownship = self.ownship_keys.contains(&obs.key);
        let fresh_window = self.config.freshness.fresh_window_us;
        let coast_after = self.config.freshness.coast_after_us;

        if let Some(existing) = self.tracks.iter_mut().find(|t| t.key == obs.key) {
            let id = existing.id;
            if !accepts_update(existing, &obs, now_us, fresh_window) {
                if existing.callsign.is_none() && obs.callsign.is_some() {
                    existing.callsign = obs.callsign;
                    self.bump();
                }
                return Ok(IngestOutcome::IgnoredLowerAuthority(id));
            }
            apply_observation(existing, &obs, now_us, coast_after, is_ownship);
            self.bump();
            return Ok(IngestOutcome::Updated(id));
        }

        if self.tracks.len() >= self.config.max_tracks {
            return Err(SurveillanceError::CapacityExceeded);
        }
        let id = TrackId(if self.next_id == 0 { 1 } else { self.next_id });
        self.next_id = id.0.wrapping_add(1);
        let phase = if now_us.saturating_sub(obs.observed_at_us) > coast_after {
            TrackPhase::Coasting
        } else {
            TrackPhase::Active
        };
        self.tracks.push(TrackSnapshot {
            id,
            key: obs.key,
            phase,
            source: obs.source,
            delivery: obs.delivery,
            observed_at_us: obs.observed_at_us,
            position: obs.position,
            velocity_ned_m_s: obs.velocity_ned_m_s,
            heading_rad: obs.heading_rad,
            callsign: obs.callsign,
            category: obs.category,
            on_ground: obs.on_ground,
            quality: obs.quality,
            ownship_shadow: is_ownship,
        });
        self.bump();
        Ok(IngestOutcome::Created(id))
    }

    /// Advances track lifecycle to `now_us`, coasting stale tracks and removing expired ones.
    /// Returns the number of expired tracks removed.
    pub fn advance_time(&mut self, now_us: u64) -> usize {
        let coast_after = self.config.freshness.coast_after_us;
        let expire_after = self.config.freshness.expire_after_us;
        let mut changed = false;
        let before = self.tracks.len();
        self.tracks.retain_mut(|track| {
            let age = now_us.saturating_sub(track.observed_at_us);
            if age > expire_after {
                return false;
            }
            let next_phase = if age > coast_after {
                TrackPhase::Coasting
            } else {
                TrackPhase::Active
            };
            if track.phase != next_phase {
                track.phase = next_phase;
                changed = true;
            }
            true
        });
        let removed = before.saturating_sub(self.tracks.len());
        if removed > 0 || changed {
            self.bump();
        }
        removed
    }

    /// Projects track `id` to `now_us` using configured [`ProjectionBounds`].
    pub fn project_position(&self, id: TrackId, now_us: u64) -> Option<GeodeticPosition> {
        let track = self.track(id)?;
        project_geodetic(track, now_us, self.config.projection)
    }

    /// Computes relative geometry and CPA metrics for all non-ownship tracks at `now_us`,
    /// sorted by ascending three-dimensional slant range.
    pub fn relative_to_ownship(
        &self,
        ownship: GeodeticPosition,
        ownship_vel_ned_m_s: Ned,
        now_us: u64,
    ) -> Vec<RelativeTarget> {
        let mut out = Vec::with_capacity(self.tracks.len());
        for track in &self.tracks {
            if track.ownship_shadow {
                continue;
            }
            if let Some(rel) = relative_target(
                ownship,
                ownship_vel_ned_m_s,
                track,
                now_us,
                self.config.projection,
            ) {
                out.push(rel);
            }
        }
        out.sort_by(|a, b| a.slant_range_m.total_cmp(&b.slant_range_m));
        out
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

fn accepts_update(
    existing: &TrackSnapshot,
    candidate: &TrackObservation,
    now_us: u64,
    fresh_window_us: u64,
) -> bool {
    let existing_fresh = now_us.saturating_sub(existing.observed_at_us) <= fresh_window_us;
    if !existing_fresh {
        return candidate.observed_at_us >= existing.observed_at_us;
    }
    let cand_rank = candidate.source.authority_rank();
    let curr_rank = existing.source.authority_rank();
    if cand_rank > curr_rank {
        true
    } else if cand_rank < curr_rank {
        false
    } else {
        candidate.observed_at_us >= existing.observed_at_us
    }
}

fn apply_observation(
    slot: &mut TrackSnapshot,
    obs: &TrackObservation,
    now_us: u64,
    coast_after_us: u64,
    is_ownship: bool,
) {
    slot.source = obs.source;
    slot.delivery = obs.delivery;
    slot.observed_at_us = obs.observed_at_us;
    slot.position = obs.position;
    slot.velocity_ned_m_s = obs.velocity_ned_m_s;
    slot.heading_rad = obs.heading_rad.or(slot.heading_rad);
    slot.callsign = obs.callsign.or(slot.callsign);
    if obs.category != crate::EmitterCategory::Unknown {
        slot.category = obs.category;
    }
    slot.on_ground = obs.on_ground;
    slot.quality = obs.quality;
    slot.ownship_shadow = is_ownship;
    slot.phase = if now_us.saturating_sub(obs.observed_at_us) > coast_after_us {
        TrackPhase::Coasting
    } else {
        TrackPhase::Active
    };
}
