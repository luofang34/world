//! The Earth's main magnetic field from the World Magnetic Model.
//!
//! [`wmm2025`] evaluates WMM2025 at a WGS-84 [`GeodeticPosition`] and a
//! [`DecimalYear`], and gives the field in local north-east-down components
//! with its declination, inclination, intensity and the model's published
//! one-sigma uncertainty. A date outside the model's validity window is
//! refused with a typed error, not extrapolated.
//!
//! The crate is `no_std`, allocation-free and deterministic: the same
//! inputs give the same bits on every target. Its dependencies are
//! `world-core` and `libm`. [`WMM2025`] names the coefficient file the
//! model was generated from and its checksum.
//!
//! [`GeodeticPosition`]: world_core::GeodeticPosition
//! [`DecimalYear`]: world_core::DecimalYear

#![no_std]

mod coefficients;
mod error;
mod field;
mod model;

pub use error::GeomagError;
pub use field::{FieldUncertainty, GeomagModel, MagneticField, WMM2025};
pub use model::wmm2025;
