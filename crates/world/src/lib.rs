//! Two-tier Earth and operational environment models for flight controllers,
//! simulators and operator stations.
//!
//! ## Tier 0 (default features: `atmosphere` + `geomag`)
//!
//! `no_std`, zero-`alloc` and deterministic (`libm` only), suitable for
//! bare-metal flight-control firmware (`thumbv7em-none-eabihf`) and browsers:
//! - [`core`] (`world-core`): WGS-84 geodesy, [`LocalTangentPlane`],
//!   [`normal_gravity`], [`DecimalYear`], [`Provenance`] and [`ValidityWindow`].
//! - [`atmosphere`] (`world-atmosphere`): ISO 2533 [`atmosphere::Isa`] behind
//!   [`atmosphere::AtmosphereModel`].
//! - [`geomag`] (`world-geomag`): NOAA [`geomag::wmm2025`] main magnetic field.
//! - [`StandardWorld`]: zero-allocation physical baseline combining all three.
//!
//! ## Tier 1 (`moving = ["airmass", "surveillance", "notices"]`)
//!
//! `no_std` + `alloc` value-typed stores for perishable, moving operational
//! state — with no globals, threads or sockets so a simulator can hold one
//! [`MovingWorld`] for physical truth and independent [`MovingWorld`] instances
//! for each vehicle's delayed radio/datalink belief.

#![no_std]

pub use world_core as core;
pub use world_core::{
    DatasetId, DecimalYear, Ecef, Ellipsoid, GeodeticPosition, HorizontalDatum, LocalTangentPlane,
    Ned, PositionError, Provenance, Uncertainty, ValidityWindow, WGS84, normal_gravity,
};

#[cfg(feature = "atmosphere")]
pub use world_atmosphere as atmosphere;

#[cfg(feature = "geomag")]
pub use world_geomag as geomag;

#[cfg(feature = "airmass")]
pub use world_airmass as airmass;

#[cfg(feature = "surveillance")]
pub use world_surveillance as surveillance;

#[cfg(feature = "notices")]
pub use world_notices as notices;

#[cfg(all(feature = "atmosphere", feature = "geomag"))]
mod standard;
#[cfg(all(feature = "atmosphere", feature = "geomag"))]
pub use standard::StandardWorld;

#[cfg(feature = "moving")]
mod moving;
#[cfg(feature = "moving")]
pub use moving::MovingWorld;

#[cfg(test)]
mod tests;
