//! Empirical models of the upper atmosphere.
//!
//! [`Nrlmsise00`] is the NRLMSISE-00 model of Picone, Hedin, Drob and Aikin
//! (2002): the number densities of He, O, N2, O2, Ar, H, N and anomalous
//! oxygen, the total mass density and the temperature from the ground to
//! the exosphere, as functions of position, day of year, time of day and
//! solar and geomagnetic activity ([`MsisInput`]). Each term of the model
//! can be switched off through [`Switches`].
//!
//! A model is a pure function of its inputs: it holds no state between
//! calls, reads no files and allocates nothing. The activity indices are
//! the caller's to supply; nothing here looks them up.
//!
//! Outputs are SI ([`ThermosphereState`]). The reference implementation the
//! parameters and the arithmetic come from is named, with its checksum and
//! terms, by [`NRLMSISE00`].
//!
//! `no_std` and allocation-free, with `world-core` and `libm` its only
//! dependencies. Every transcendental function goes through `libm`, so a
//! result is the same bits on a host, in firmware and in a browser.

#![no_std]

#[cfg(test)]
extern crate std;

mod error;
mod input;
mod math;
mod nrlmsise00;
mod state;
mod switches;

pub use error::ThermosphereError;
pub use input::{ApHistory, GeomagneticActivity, MsisInput};
pub use nrlmsise00::{NRLMSISE00, Nrlmsise00};
pub use state::{NumberDensities, ThermosphereState};
pub use switches::{Effect, Switch, Switches};
