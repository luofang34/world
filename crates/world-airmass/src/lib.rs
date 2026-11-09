//! Perishable weather fields, multi-layer winds and temperatures aloft,
//! surface station observations and aviation hazard volumes.
//!
//! [`AirmassStore`] is a value-typed, single-writer weather store with no
//! global state, sockets, threads or clocks. It implements
//! [`AtmosphereModel`] so consumers written against `world-atmosphere`
//! evaluate real soundings when present and fall back cleanly to [`Isa`]
//! outside local coverage, while [`WindModel`] adds three-dimensional wind,
//! gusts, speed of sound and eddy-dissipation turbulence.
//!
//! `no_std` + `alloc`, depending only on `world-core`, `world-atmosphere` and
//! `libm`.
//!
//! [`AtmosphereModel`]: world_atmosphere::AtmosphereModel
//! [`Isa`]: world_atmosphere::Isa

#![no_std]

extern crate alloc;

mod cloud;
mod hazard;
mod sounding;
mod store;

#[cfg(test)]
mod tests;

pub use cloud::{CloudGenus, CloudLayer, FlightCategory};
pub use hazard::{
    HazardGeometry, HazardKind, IcingSeverity, TurbulenceSeverity, WeatherHazard,
    horizontal_distance_m, point_in_polygon,
};
pub use sounding::{
    AirmassError, AloftLevel, SoundingProfile, StationId, SurfaceObservation,
    dry_air_speed_of_sound_m_s,
};
pub use store::{AirmassConfig, AirmassSample, AirmassStore, FallbackPolicy, WindModel};
