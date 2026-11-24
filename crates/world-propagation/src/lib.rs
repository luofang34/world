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
//! - [`p531`]: ITU-R P.531-15 ionospheric slant TEC, group delay, phase
//!   advance, dispersion, Faraday rotation, absorption, and scintillation.
//! - [`p618`]: ITU-R P.618-14 Earth-space slant-path attenuation (gaseous,
//!   cloud, rain, scintillation, and XPD) over caller-supplied,
//!   SHA-256-verified atmospheric grids.
//! - [`antenna`]: the ITU-R S.465, S.580 and F.1245 reference radiation
//!   patterns, plus (with feature `alloc`) MSI Planet and NSMA WG16.99
//!   tabulated antenna pattern readers.
//!
//! [`sources`] identifies each Recommendation by the checksum of the
//! published text the code follows.
//!
//! `no_std` and allocation-free by default, with `world-core` and `libm` its
//! only dependencies; enabling `alloc` adds only `extern crate alloc` for
//! tabulated antenna file parsing. `libm` keeps native and `wasm32` results
//! bit-identical.

#![no_std]
#![forbid(unsafe_code)]
#![forbid(clippy::indexing_slicing)]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod antenna;
mod error;
mod ground;
pub mod itm;
mod loss;
pub mod p526;
pub mod p531;
pub mod p618;
mod profile;
pub mod sources;
mod units;

pub use antenna::AntennaPattern;
#[cfg(feature = "alloc")]
pub use antenna::{
    AntennaMetadata, CutPlane, PatternCut, PatternPolarisation, PatternSample,
    TabulatedAntennaPattern,
};
pub use error::{PatternParseErrorKind, PropagationError};
pub use ground::Ground;
pub use loss::{Loss, PathLoss, Signal};
pub use profile::{ProfilePoint, TerrainProfile};
pub use units::{Frequency, Percentage, Polarisation, SPEED_OF_LIGHT_M_S};
