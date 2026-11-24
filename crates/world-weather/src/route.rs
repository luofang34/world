//! Advisories along a route through space and time.

use alloc::vec::Vec;

use airmass_core::{UtcTime, WeatherSnapshot};
use world_core::{RouteError, RoutePoint, RouteSampling, sample_route};

use crate::{AdvisoryHit, AdvisoryQuery, QueryAltitude, advisories_at};

const FEET_PER_METRE: f64 = 1.0 / 0.3048;

/// The advisories that apply, or may apply, at one sampled route point.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteAdvisorySample<'a> {
    /// Where and when the route was sampled.
    pub point: RoutePoint,
    /// The advisories there and then; see [`advisories_at`].
    pub hits: Vec<AdvisoryHit<'a>>,
}

/// Ask [`advisories_at`] along `route` at every `sampling.interval_micros`.
///
/// Each sample is asked at the route point's UTC, so an advisory that ends
/// before the vehicle arrives does not answer for that part of the route.
/// The route's ellipsoidal height is taken as altitude above mean sea level
/// (the geoid undulation is not applied); a route states no pressure
/// altitude or height above ground, so a band on a flight level or above
/// ground comes back unresolved.
///
/// # Errors
///
/// [`RouteError`] when the route is malformed or needs more samples than
/// `sampling.max_samples`.
pub fn advisories_along_route<'a>(
    snapshot: &'a WeatherSnapshot,
    route: &[RoutePoint],
    sampling: RouteSampling,
    line_half_width_m: f64,
) -> Result<Vec<RouteAdvisorySample<'a>>, RouteError> {
    Ok(sample_route(route, sampling)?
        .map(|point| {
            let query = AdvisoryQuery {
                position: point.position,
                altitude: QueryAltitude {
                    msl_ft: Some(point.position.ellipsoidal_height_m * FEET_PER_METRE),
                    ..QueryAltitude::default()
                },
                at: UtcTime::from_unix_millis(point.at.utc_unix_micros.div_euclid(1_000)),
                line_half_width_m,
            };
            RouteAdvisorySample {
                point,
                hits: advisories_at(snapshot, &query),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests;
