//! Traffic queries over Surveillance track snapshots.
//!
//! Surveillance owns traffic state: observations, fusion, freshness and the
//! [`TrackSnapshot`] each track publishes. This crate holds no tracks. It
//! answers the questions a cockpit and a planner ask of those snapshots:
//! where a track is relative to ownship ([`relative_to_ownship`]), which
//! tracks are nearest or within a radius ([`traffic_by_range`], [`nearest`],
//! [`within_radius`]), and which come near a route through space and time
//! ([`traffic_along_route`]).
//!
//! Every answer places a track with Surveillance's own
//! [`project_position`] and the [`ProjectionBounds`] stamped on the
//! snapshot, and is refused with a [`TrafficRefusal`] outside them: there is
//! no second freshness policy here. That placement is short-horizon dead
//! reckoning along the last reported velocity. It is not a prediction of
//! where a target will fly, and [`LinearCpa`] is the closest approach of two
//! straight lines, not a forecast of one.
//!
//! `no_std` + `alloc`, depending on `surveillance-core`, `world-core`,
//! `thiserror` and `libm`.
//!
//! [`TrackSnapshot`]: surveillance_core::TrackSnapshot
//! [`project_position`]: surveillance_core::project_position
//! [`ProjectionBounds`]: surveillance_core::ProjectionBounds

#![no_std]

extern crate alloc;

mod fix;
mod proximity;
mod relative;
mod route;

pub use fix::{TrackFix, TrafficRefusal, track_fix};
pub use proximity::{nearest, traffic_by_range, within_radius};
pub use relative::{
    AltitudeReference, LinearCpa, Ownship, RelativeAltitude, RelativeTraffic, relative_to_ownship,
};
pub use route::{RouteEncounter, RouteTrafficQuery, RouteTrafficSample, traffic_along_route};
