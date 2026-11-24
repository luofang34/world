//! Cloud layers of the physical air model.

use world_core::{GeodeticPosition, ValidityWindow};

/// Broad cloud genus used by volumetric cloud renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CloudGenus {
    /// Layered cloud with gentle vertical motion (stratus, altostratus).
    Stratiform,
    /// Heaped convective cloud (cumulus, towering cumulus).
    Cumuliform,
    /// Deep thunderstorm convection (cumulonimbus).
    Convective,
    /// High ice-crystal cloud (cirrus, cirrostratus).
    Cirriform,
}

/// A regional or global cloud layer with explicit validity bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudLayer {
    /// Identifier assigned by the producer.
    pub id: u64,
    /// Center of the cloud region, or `None` for a global layer.
    pub center: Option<GeodeticPosition>,
    /// Horizontal radius around `center` in meters; ignored when `center` is `None`.
    pub radius_m: f64,
    /// Fractional sky coverage in `0.0..=1.0`.
    pub coverage: f64,
    /// Cloud base above mean sea level, meters.
    pub base_msl_m: f64,
    /// Cloud top above mean sea level, meters.
    pub top_msl_m: f64,
    /// Cloud genus.
    pub genus: CloudGenus,
    /// Dates for which this layer is valid.
    pub valid: ValidityWindow,
}
