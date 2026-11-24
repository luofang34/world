//! Physical model of the air aloft: sounding profiles, the wind and
//! temperature they give at a point and along a route, and cloud layers.
//!
//! [`AloftModel`] is a value-typed, single-writer model with no global
//! state, sockets, threads or clocks. It implements [`AtmosphereModel`] so
//! consumers written against `world-atmosphere` evaluate real soundings when
//! present and fall back cleanly to [`Isa`] outside local coverage, while
//! [`WindModel`] adds three-dimensional wind, gusts, speed of sound and
//! eddy-dissipation turbulence. [`winds_along_route`] asks either along a
//! route through space and time.
//!
//! It models the atmosphere a vehicle flies in; it holds no weather
//! products. Station reports, advisories, their revisions and expiry are
//! Airmass state.
//!
//! `no_std` + `alloc`, depending only on `world-core`, `world-atmosphere` and
//! `libm`.
//!
//! [`AtmosphereModel`]: world_atmosphere::AtmosphereModel
//! [`Isa`]: world_atmosphere::Isa

#![no_std]

extern crate alloc;

mod cloud;
mod distance;
mod model;
mod route;
mod sounding;

pub use cloud::{CloudGenus, CloudLayer};
pub use distance::horizontal_distance_m;
pub use model::{AloftConfig, AloftModel, AloftSample, FallbackPolicy, WindModel};
pub use route::{AloftRouteSample, winds_along_route};
pub use sounding::{AloftError, AloftLevel, SoundingProfile, dry_air_speed_of_sound_m_s};
