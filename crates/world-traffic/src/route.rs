//! Traffic near a route through space and time.

use alloc::vec::Vec;

use surveillance_core::{TrackId, TrackKey, TrackSnapshot};
use world_core::{Ned, RouteError, RoutePoint, RouteSampling, sample_route};

use crate::{Ownship, RelativeAltitude, TrackFix, relative_to_ownship};

/// What to look for along a route.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteTrafficQuery {
    /// How densely the route is sampled, and the most samples allowed.
    pub sampling: RouteSampling,
    /// Horizontal radius around each sample inside which a track is
    /// reported, meters.
    pub radius_m: f64,
}

/// A track placed near one route sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteEncounter {
    /// Stable engine-assigned track identity.
    pub id: TrackId,
    /// The track's primary over-the-air identity.
    pub key: TrackKey,
    /// The track's dead-reckoned position at the sample's instant.
    pub fix: TrackFix,
    /// Horizontal range from the route sample, meters.
    pub horizontal_range_m: f64,
    /// Geometric altitude relative to the route sample, when the track
    /// reports a current geometric altitude.
    pub relative_altitude: Option<RelativeAltitude>,
}

/// The traffic picture at one route sample.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteTrafficSample {
    /// Where and when the route was sampled.
    pub point: RoutePoint,
    /// Tracks placed within the query radius at this instant, nearest
    /// first.
    pub encounters: Vec<RouteEncounter>,
    /// Tracks that exist but cannot be placed at this instant inside their
    /// producer's bounds. A sample with unplaced tracks and no encounters is
    /// unknown, not clear.
    pub unplaced_tracks: usize,
}

/// Compare `tracks` against `route` at each sampled instant.
///
/// Each track is dead-reckoned to the sample's monotonic time with its own
/// producer's bounds, which reach seconds past its last report. Samples
/// beyond that horizon therefore place nothing and count every track in
/// [`RouteTrafficSample::unplaced_tracks`]: this answers "what is near the
/// part of the route I am about to fly", and says so when asked about more.
/// It is never a forecast of where traffic will be later along the route.
///
/// # Errors
///
/// [`RouteError`] when the route is malformed or needs more samples than
/// `query.sampling.max_samples`.
pub fn traffic_along_route<T: AsRef<TrackSnapshot>>(
    route: &[RoutePoint],
    tracks: &[T],
    query: RouteTrafficQuery,
) -> Result<Vec<RouteTrafficSample>, RouteError> {
    Ok(sample_route(route, query.sampling)?
        .map(|point| sample_traffic(point, tracks, query.radius_m))
        .collect())
}

fn sample_traffic<T: AsRef<TrackSnapshot>>(
    point: RoutePoint,
    tracks: &[T],
    radius_m: f64,
) -> RouteTrafficSample {
    // A route states no pressure altitude and no velocity at a sample, so
    // only geometric altitudes compare and no closest approach is derived.
    let here = Ownship {
        position: point.position,
        velocity_ned_m_s: Ned {
            north_m: 0.0,
            east_m: 0.0,
            down_m: 0.0,
        },
        pressure_altitude_ft: None,
    };
    let mut encounters = Vec::new();
    let mut unplaced_tracks = 0_usize;
    for track in tracks {
        let snapshot = track.as_ref();
        if snapshot.ownship_shadow {
            continue;
        }
        match relative_to_ownship(&here, snapshot, point.at.monotonic_micros) {
            Ok(relative) if relative.horizontal_range_m <= radius_m => {
                encounters.push(RouteEncounter {
                    id: relative.id,
                    key: relative.key,
                    fix: relative.fix,
                    horizontal_range_m: relative.horizontal_range_m,
                    relative_altitude: relative.relative_altitude,
                });
            }
            Ok(_) => {}
            Err(_) => unplaced_tracks = unplaced_tracks.saturating_add(1),
        }
    }
    encounters.sort_by(|a, b| {
        a.horizontal_range_m
            .total_cmp(&b.horizontal_range_m)
            .then(a.id.cmp(&b.id))
    });
    RouteTrafficSample {
        point,
        encounters,
        unplaced_tracks,
    }
}

#[cfg(test)]
mod tests;
