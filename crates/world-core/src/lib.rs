//! Shared vocabulary for world models.
//!
//! A world model answers a question about the Earth at a place and a time,
//! from a published dataset. This crate names those parts once so every
//! model states them the same way: where ([`GeodeticPosition`]), when
//! ([`DecimalYear`]), from what ([`DatasetId`], [`Provenance`]), for which
//! dates ([`ValidityWindow`]) and how well ([`Uncertainty`]). A
//! [`LocalTangentPlane`] relates geodetic positions to the north-east-down
//! frame a vehicle is simulated or estimated in, on the datum's ellipsoid.
//!
//! `no_std` and allocation-free, with `libm` its only dependency, so a
//! model built on it runs in flight-controller firmware as well as on a
//! host.

#![no_std]

mod dataset;
mod ellipsoid;
mod local;
mod position;
mod time;
mod uncertainty;

pub use dataset::{DatasetId, Provenance, ValidityWindow};
pub use ellipsoid::{Ecef, Ellipsoid, WGS84};
pub use local::{LocalTangentPlane, Ned, PositionError};
pub use position::{GeodeticPosition, HorizontalDatum};
pub use time::DecimalYear;
pub use uncertainty::Uncertainty;
