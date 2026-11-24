//! Earth and operational environment models for experimental flight-control
//! prototypes, simulators and operator stations, and the umbrella over the
//! domain libraries that own traffic and weather state.
//!
//! > **Warning — Experimental Prototype (Not for Navigation or Flight):**
//! > This crate is an unverified research and simulation prototype and must
//! > never be used for real-world navigation, flight planning, traffic
//! > separation, or operational flight control.
//!
//! ## Tier 0 (default features: `atmosphere` + `geomag`)
//!
//! `no_std`, zero-`alloc` and deterministic (`libm` only), suitable for
//! bare-metal flight-control firmware (`thumbv7em-none-eabihf`) and browsers:
//! - [`core`] (`world-core`): WGS-84 geodesy, [`LocalTangentPlane`],
//!   [`normal_gravity`], [`DecimalYear`], [`WorldInstant`], [`RoutePoint`],
//!   [`Provenance`] and [`ValidityWindow`].
//! - [`atmosphere`] (`world-atmosphere`): ISO 2533 [`atmosphere::Isa`] behind
//!   [`atmosphere::AtmosphereModel`].
//! - [`geomag`] (`world-geomag`): NOAA [`geomag::wmm2025`] main magnetic field.
//! - [`StandardWorld`]: zero-allocation physical baseline combining all three.
//!
//! ## Tier 1 (`moving = ["surveillance", "airmass", "notices", "aloft"]`)
//!
//! `no_std` + `alloc`. Traffic and weather state belong to the dedicated
//! libraries, re-exported here unchanged; this workspace adds queries over
//! their snapshots and never a second copy of their state:
//! - `surveillance`: `world::surveillance` is `surveillance-core`
//!   (observations, the single-writer engine, tracks, deltas, snapshots) and
//!   `world::traffic` answers ownship-relative and along-route questions of
//!   its snapshots.
//! - `airmass`: `world::airmass` is `airmass-core` (weather ingress, the
//!   store, immutable snapshots) and `world::weather` answers station and
//!   advisory questions of its snapshots.
//! - `aloft`: `world::aloft` is the physical model of the air (soundings,
//!   winds and temperatures at a point and along a route, cloud layers).
//! - `notices`: `world::notices` holds perishable airspace notices.
//! - `MovingWorld` composes the four on one caller-driven clock.
//!
//! Nothing here opens a socket, reads a clock or spawns a task: turning a
//! radio reception, a provider response or a simulator state into a
//! `TrafficObservation` or a `WeatherIngress` is a host adapter's job.

#![no_std]

#[cfg(feature = "moving")]
extern crate alloc;

pub use world_core as core;
pub use world_core::{
    ClockCorrespondence, DatasetId, DecimalYear, Ecef, Ellipsoid, GeodeticPosition,
    HorizontalDatum, LocalTangentPlane, Ned, PositionError, Provenance, RoutePoint, RouteSampling,
    Uncertainty, ValidityWindow, WGS84, WorldInstant, normal_gravity,
};

#[cfg(feature = "atmosphere")]
pub use world_atmosphere as atmosphere;

#[cfg(feature = "geomag")]
pub use world_geomag as geomag;

#[cfg(feature = "aloft")]
pub use world_aloft as aloft;

#[cfg(feature = "surveillance")]
pub use surveillance_core as surveillance;
#[cfg(feature = "surveillance")]
pub use world_traffic as traffic;

#[cfg(feature = "airmass")]
pub use airmass_core as airmass;
#[cfg(feature = "airmass")]
pub use world_weather as weather;

#[cfg(feature = "notices")]
pub use world_notices as notices;

#[cfg(any(feature = "airmass", feature = "surveillance"))]
pub mod clock;

#[cfg(all(feature = "atmosphere", feature = "geomag"))]
mod standard;
#[cfg(all(feature = "atmosphere", feature = "geomag"))]
pub use standard::StandardWorld;

#[cfg(feature = "moving")]
mod moving;
#[cfg(feature = "moving")]
pub use moving::{AdvanceReport, MovingWorld, MovingWorldConfig, MovingWorldError};

#[cfg(test)]
mod tests;
