//! Where a track is at one instant, inside the bounds its producer declared.

use surveillance_core::{
    PositionStatus, ProjectionBasis, TrackId, TrackPhase, TrackSnapshot, Wgs84Position,
    project_position,
};
use thiserror::Error;

const METRES_PER_NAUTICAL_MILE: f64 = 1_852.0;
const MICROS_PER_HOUR: f64 = 3_600_000_000.0;

/// A track's position at one instant, as Surveillance places it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackFix {
    /// Position at the queried instant.
    pub position: Wgs84Position,
    /// Barometric pressure altitude in feet at the queried instant, absent
    /// when the track reports none.
    pub pressure_altitude_ft: Option<i32>,
    /// Whether the position is the reported one or the reported one
    /// dead-reckoned along the reported velocity.
    pub basis: ProjectionBasis,
    /// Age of the reported position at the queried instant, microseconds.
    pub position_age_micros: u64,
    /// Whether the dead-reckoned leg reached the producer's distance limit,
    /// so the position lags where the reported velocity would put it.
    pub distance_limited: bool,
}

/// Why a track cannot be placed or compared at the queried instant.
#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum TrafficRefusal {
    /// The track has no position.
    #[error("track {id:?} has no position")]
    NoPosition {
        /// The track.
        id: TrackId,
    },
    /// The snapshot carries no projection bounds, so how long its position
    /// holds cannot be established.
    #[error("track {id:?} declares no projection bounds")]
    NoProjectionBounds {
        /// The track.
        id: TrackId,
    },
    /// The queried instant is before the position was observed.
    #[error("track {id:?} was observed {early_micros} us after the queried instant")]
    BeforeObservation {
        /// The track.
        id: TrackId,
        /// How far the queried instant precedes the observation.
        early_micros: u64,
    },
    /// The track coasts or has been silent past the producer's coast window.
    #[error(
        "track {id:?} is coasting: silent {silent_micros} us, coast window {coast_after_micros} us"
    )]
    Coasting {
        /// The track.
        id: TrackId,
        /// Time since the newest observation.
        silent_micros: u64,
        /// The producer's coast window.
        coast_after_micros: u64,
    },
    /// The position is older than the producer's projection window.
    #[error("track {id:?} position is {age_micros} us old; the window is {limit_micros} us")]
    PositionStale {
        /// The track.
        id: TrackId,
        /// Age of the reported position.
        age_micros: u64,
        /// The producer's projection window.
        limit_micros: u64,
    },
    /// The track is a rebroadcast shadow of ownship, not an intruder.
    #[error("track {id:?} is an ownship shadow")]
    OwnshipShadow {
        /// The track.
        id: TrackId,
    },
    /// The ownship or route position is malformed or not on WGS 84.
    #[error("the reference position is malformed or not on WGS 84")]
    ReferenceNotUsable,
}

/// Place `snapshot` at `at_micros` on the producer's monotonic clock.
///
/// The position comes from [`project_position`]. This function adds only the
/// refusals that function leaves to its caller: it returns the reported
/// position unchanged past its bounds, and a caller comparing positions at
/// one instant must not take that for a current one.
///
/// # Errors
///
/// [`TrafficRefusal`] when the track has no position or declared bounds,
/// when `at_micros` precedes the observation, when the track coasts, or when
/// the position is older than the producer's projection window.
pub fn track_fix(snapshot: &TrackSnapshot, at_micros: u64) -> Result<TrackFix, TrafficRefusal> {
    let id = snapshot.id;
    let position = snapshot
        .position
        .as_ref()
        .ok_or(TrafficRefusal::NoPosition { id })?;
    let bounds = snapshot
        .projection_bounds
        .ok_or(TrafficRefusal::NoProjectionBounds { id })?;
    let observed_at = position.observed_at_micros();
    if at_micros < observed_at {
        return Err(TrafficRefusal::BeforeObservation {
            id,
            early_micros: observed_at - at_micros,
        });
    }
    let silent_micros = at_micros.saturating_sub(snapshot.last_observed_at_micros);
    if snapshot.phase != TrackPhase::Active || silent_micros > bounds.coast_after_micros {
        return Err(TrafficRefusal::Coasting {
            id,
            silent_micros,
            coast_after_micros: bounds.coast_after_micros,
        });
    }
    let age_micros = position.age_micros(at_micros);
    if snapshot.position_status != PositionStatus::Current
        || age_micros > bounds.extrapolate_max_micros
    {
        return Err(TrafficRefusal::PositionStale {
            id,
            age_micros,
            limit_micros: bounds.extrapolate_max_micros,
        });
    }
    let projection =
        project_position(snapshot, at_micros).ok_or(TrafficRefusal::NoPosition { id })?;
    let leg_m = snapshot
        .velocity
        .as_ref()
        .and_then(|velocity| velocity.value.ground_speed_kt)
        .map_or(0.0, |ground_speed_kt| {
            ground_speed_kt * age_micros as f64 * METRES_PER_NAUTICAL_MILE / MICROS_PER_HOUR
        });
    Ok(TrackFix {
        position: projection.position,
        pressure_altitude_ft: projection.pressure_altitude_ft,
        basis: projection.basis,
        position_age_micros: age_micros,
        distance_limited: projection.basis == ProjectionBasis::Extrapolated
            && leg_m > bounds.extrapolate_max_distance_m as f64,
    })
}

#[cfg(test)]
pub(crate) mod tests;
