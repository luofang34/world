//! Models of the atmosphere a vehicle flies in.
//!
//! [`Isa`] is the ISO 2533 International Standard Atmosphere: temperature,
//! pressure and density as functions of altitude alone. It implements
//! [`AtmosphereModel`], the seam a weather field (a forecast or an
//! observed sounding, varying with place and time) implements in turn, so
//! a consumer written against the trait takes either.
//!
//! Altitude is typed: an [`Altitude`] states whether it is geopotential or
//! geometric above mean sea level, so a height above the ellipsoid cannot
//! be passed by mistake.
//!
//! `no_std` and allocation-free, with `world-core` and `libm` its only
//! dependencies.

#![no_std]

mod altitude;
mod isa;
mod model;

pub use altitude::Altitude;
pub use isa::Isa;
pub use model::{AtmosphereError, AtmosphereModel, AtmospherePoint, AtmosphereState};
