//! Shared vocabulary for world models.
//!
//! A world model answers a question about the Earth at a place and a time,
//! from a published dataset. This crate names those parts once so every
//! model states them the same way: where ([`GeodeticPosition`]), when
//! ([`DecimalYear`]), from what ([`DatasetId`], [`Provenance`]), for which
//! dates ([`ValidityWindow`]) and how well ([`Uncertainty`]).
//!
//! `no_std`, allocation-free and dependency-free, so a model built on it
//! runs in flight-controller firmware as well as on a host.

#![no_std]

mod dataset;
mod position;
mod time;
mod uncertainty;

pub use dataset::{DatasetId, Provenance, ValidityWindow};
pub use position::{GeodeticPosition, HorizontalDatum};
pub use time::DecimalYear;
pub use uncertainty::Uncertainty;
