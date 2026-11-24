//! A single knife-edge obstacle (P.526-16 §4.1).

use crate::error::{PropagationError, finite, positive};
use crate::loss::{Loss, PathLoss, Signal};
use crate::p526::fresnel::knife_edge_field;

/// The ν below which eq. (31) is not used and the loss is taken as zero.
pub const APPROXIMATION_CUTOFF: f64 = -0.78;

/// The dimensionless diffraction parameter ν of eq. (26):
/// h √(2/λ · (1/d₁ + 1/d₂)), all lengths in metres.
///
/// # Errors
///
/// [`PropagationError::InvalidParameter`] for a non-finite height or a
/// non-positive distance or wavelength.
pub fn diffraction_parameter(
    height_m: f64,
    d1_m: f64,
    d2_m: f64,
    wavelength_m: f64,
) -> Result<f64, PropagationError> {
    let h = finite("height_m", height_m)?;
    let d1 = positive("d1_m", d1_m)?;
    let d2 = positive("d2_m", d2_m)?;
    let lambda = positive("wavelength_m", wavelength_m)?;
    Ok(h * libm::sqrt(2.0 / lambda * (1.0 / d1 + 1.0 / d2)))
}

/// J(ν), the Fresnel–Kirchhoff knife-edge loss of eq. (30), dB.
///
/// Returned as the Recommendation's Fig. 9 draws it: for an edge below
/// the line of sight (ν < 0) it oscillates about 0 dB and is negative
/// (a gain over free space) where the edge reflects in phase.
pub fn knife_edge_loss_db(nu: f64) -> f64 {
    -20.0 * libm::log10(knife_edge_field(nu))
}

/// J(ν) by the approximation of eq. (31), dB: zero for
/// ν ≤ [`APPROXIMATION_CUTOFF`], as the Bullington method of §4.5.1
/// applies it.
pub fn knife_edge_loss_approx_db(nu: f64) -> f64 {
    if nu > APPROXIMATION_CUTOFF {
        let v = nu - 0.1;
        6.9 + 20.0 * libm::log10(libm::sqrt(v * v + 1.0) + v)
    } else {
        0.0
    }
}

/// Which form of J(ν) a [`KnifeEdge`] evaluates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KnifeEdgeMethod {
    /// Eq. (30) from the Fresnel integrals.
    #[default]
    FresnelIntegral,
    /// Eq. (31), zero for ν ≤ −0.78.
    Approximation,
}

/// The geometry of one knife edge (Fig. 8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnifeEdgeGeometry {
    /// Height of the edge above the straight line joining the two
    /// terminals, m; negative when the edge is below it.
    pub height_m: f64,
    /// Distance from the transmitter to the edge, m.
    pub d1_m: f64,
    /// Distance from the edge to the receiver, m.
    pub d2_m: f64,
}

/// Diffraction over a single knife edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct KnifeEdge {
    /// Which form of J(ν) to evaluate.
    pub method: KnifeEdgeMethod,
}

/// A knife edge's loss and its diffraction parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnifeEdgeLoss {
    /// The diffraction parameter ν.
    pub nu: f64,
    /// J(ν), dB.
    pub loss_db: f64,
}

impl Loss for KnifeEdgeLoss {
    fn loss_db(&self) -> f64 {
        self.loss_db
    }
}

impl PathLoss<KnifeEdgeGeometry> for KnifeEdge {
    type Output = KnifeEdgeLoss;

    fn path_loss(
        &self,
        geometry: &KnifeEdgeGeometry,
        signal: &Signal,
    ) -> Result<KnifeEdgeLoss, PropagationError> {
        let nu = diffraction_parameter(
            geometry.height_m,
            geometry.d1_m,
            geometry.d2_m,
            signal.frequency.wavelength_m(),
        )?;
        let loss_db = match self.method {
            KnifeEdgeMethod::FresnelIntegral => knife_edge_loss_db(nu),
            KnifeEdgeMethod::Approximation => knife_edge_loss_approx_db(nu),
        };
        Ok(KnifeEdgeLoss { nu, loss_db })
    }
}
