//! Winds and temperatures along a route through space and time.

use alloc::vec::Vec;
use world_atmosphere::{Altitude, AtmosphereError, AtmospherePoint};
use world_core::{RouteError, RoutePoint, RouteSampling, sample_route};

use crate::{AloftSample, WindModel};

/// The air at one sampled point of a route.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AloftRouteSample {
    /// Where and when the route was sampled.
    pub point: RoutePoint,
    /// The air there and then, or why the model gave none.
    pub air: Result<AloftSample, AtmosphereError>,
}

/// Sample `model` along `route` at every `sampling.interval_micros`.
///
/// Each sample asks the model at the route point's UTC, so a sounding that
/// is not valid when the vehicle gets there does not answer for it. The
/// route's ellipsoidal height is taken as geometric height above mean sea
/// level: the geoid undulation is not applied.
///
/// # Errors
///
/// [`RouteError`] when the route is malformed or needs more samples than
/// `sampling.max_samples`.
pub fn winds_along_route(
    model: &impl WindModel,
    route: &[RoutePoint],
    sampling: RouteSampling,
) -> Result<Vec<AloftRouteSample>, RouteError> {
    Ok(sample_route(route, sampling)?
        .map(|point| AloftRouteSample {
            point,
            air: model.sample(AtmospherePoint {
                latitude_deg: point.position.latitude_deg,
                longitude_deg: point.position.longitude_deg,
                altitude: Altitude::GeometricMsl(point.position.ellipsoidal_height_m),
                at: point.at.decimal_year(),
            }),
        })
        .collect())
}

#[cfg(test)]
mod tests;
