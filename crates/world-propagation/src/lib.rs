//! Radio propagation loss models and reference antenna patterns.
//!
//! Every loss model is a pure function behind [`PathLoss`]: it takes the
//! path geometry it needs (a knife edge, two terminals over a smooth
//! Earth, a [`TerrainProfile`]) and a [`Signal`] (frequency and
//! polarisation), and answers with a [`Loss`] in dB plus its typed side
//! quantities. Models hold only their own constants; nothing is global.
//!
//! - [`p526`]: ITU-R P.526-16 diffraction: a single knife edge, a smooth
//!   spherical Earth, and the delta-Bullington method over a terrain
//!   profile.
//! - [`itm`]: the NTIA/ITS Irregular Terrain Model (Longley–Rice) in
//!   point-to-point mode, ported from NTIA's reference implementation.
//! - [`antenna`]: the ITU-R S.465, S.580 and F.1245 reference radiation
//!   patterns as gain functions of off-axis angle.
//!
//! [`sources`] identifies each Recommendation by the checksum of the
//! published text the code follows.
//!
//! `no_std` and allocation-free, with `world-core` and `libm` its only
//! dependencies; `libm` keeps native and wasm32 results bit-identical.

#![no_std]
#![forbid(clippy::indexing_slicing)]

#[cfg(test)]
extern crate std;

pub mod antenna;
mod error;
mod ground;
pub mod itm;
mod loss;
pub mod p526;
mod profile;
pub mod sources;
mod units;

pub use antenna::AntennaPattern;
pub use error::PropagationError;
pub use ground::Ground;
pub use loss::{Loss, PathLoss, Signal};
pub use profile::{ProfilePoint, TerrainProfile};
pub use units::{Frequency, Percentage, Polarisation, SPEED_OF_LIGHT_M_S};
