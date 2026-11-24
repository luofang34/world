//! Recommendation ITU-R P.526-16, *Propagation by diffraction*.
//!
//! - §2.7 the complex Fresnel integral by Boersma's approximation:
//!   [`fresnel_integrals`].
//! - §4.1 a single knife edge: [`diffraction_parameter`] (eq. 26),
//!   [`knife_edge_loss_db`] (eq. 30), [`knife_edge_loss_approx_db`]
//!   (eq. 31) and the [`KnifeEdge`] model.
//! - §3.1.1 and §3.2 a smooth spherical Earth at any distance, 10 MHz and
//!   above: [`SmoothEarth`].
//! - §4.5 a general terrestrial path over a [`crate::TerrainProfile`]: the
//!   Bullington construction combined with the smooth-Earth loss,
//!   [`DeltaBullington`].

mod bullington;
mod fresnel;
mod knife_edge;
mod smooth_earth;

pub use bullington::{DeltaBullington, DeltaBullingtonLoss, ProfilePath};
pub use fresnel::fresnel_integrals;
pub use knife_edge::{
    APPROXIMATION_CUTOFF, KnifeEdge, KnifeEdgeGeometry, KnifeEdgeLoss, KnifeEdgeMethod,
    diffraction_parameter, knife_edge_loss_approx_db, knife_edge_loss_db,
};
pub use smooth_earth::{
    DEFAULT_EFFECTIVE_EARTH_RADIUS_M, MIN_FREQUENCY_HZ, SmoothEarth, SmoothEarthLoss, TerminalPath,
};

#[cfg(test)]
mod tests;
