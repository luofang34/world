//! The seam every loss model sits behind.

use crate::error::PropagationError;
use crate::units::{Frequency, Polarisation};

/// The wave a loss is evaluated for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Signal {
    /// Carrier frequency.
    pub frequency: Frequency,
    /// Polarisation; models whose loss does not depend on it ignore it.
    pub polarisation: Polarisation,
}

/// A model's answer: a loss in dB, beyond free-space spreading, plus
/// whatever typed side quantities the model defines (the knife-edge
/// diffraction parameter, an ionospheric group delay).
pub trait Loss {
    /// The loss, dB. Positive attenuates; a negative value is a gain
    /// over free space where the model predicts one.
    fn loss_db(&self) -> f64;
}

/// A propagation-loss model evaluated over a path geometry `G`.
///
/// Each model is a pure function of its own parameters (effective Earth
/// radius, ground constants), the geometry and the signal: no global
/// state, no allocation. The geometry type says what the model needs:
/// endpoint heights and a distance, a terrain profile, an elevation
/// angle, and for statistical models the percentage of time, locations
/// or situations the loss is not exceeded for.
pub trait PathLoss<G: ?Sized> {
    /// The model's answer, with its side quantities.
    type Output: Loss;

    /// The loss over `geometry` for `signal`.
    ///
    /// # Errors
    ///
    /// [`PropagationError`] when the geometry or the signal is outside
    /// what the model defines.
    fn path_loss(&self, geometry: &G, signal: &Signal) -> Result<Self::Output, PropagationError>;
}
