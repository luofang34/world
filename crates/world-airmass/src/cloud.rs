//! Cloud layers and ceiling/visibility flight categories.

use world_core::{GeodeticPosition, ValidityWindow};

/// Broad cloud genus used by weather products and volumetric cloud renderers.
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
    /// Identifier assigned by the producer or weather product.
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

/// Aviation ceiling-and-visibility flight category (FAA / ICAO thresholds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlightCategory {
    /// Visual Flight Rules: ceiling > 3,000 ft AGL and visibility > 5 SM.
    Vfr,
    /// Marginal VFR: ceiling 1,000..=3,000 ft AGL or visibility 3..=5 SM.
    Mvfr,
    /// Instrument Flight Rules: ceiling 500..<1,000 ft AGL or visibility 1..<3 SM.
    Ifr,
    /// Low IFR: ceiling < 500 ft AGL or visibility < 1 SM.
    Lifr,
}

const FEET_TO_METERS: f64 = 0.3048;
const STATUTE_MILE_TO_METERS: f64 = 1_609.344;

impl FlightCategory {
    /// Classifies conditions from an optional ceiling above ground level (meters)
    /// and prevailing horizontal visibility (meters).
    pub fn from_ceiling_and_visibility(ceiling_agl_m: Option<f64>, visibility_m: f64) -> Self {
        let lifr_ceil_m = 500.0 * FEET_TO_METERS;
        let ifr_ceil_m = 1_000.0 * FEET_TO_METERS;
        let mvfr_ceil_m = 3_000.0 * FEET_TO_METERS;

        let lifr_vis_m = 1.0 * STATUTE_MILE_TO_METERS;
        let ifr_vis_m = 3.0 * STATUTE_MILE_TO_METERS;
        let mvfr_vis_m = 5.0 * STATUTE_MILE_TO_METERS;

        let ceil_cat = match ceiling_agl_m {
            Some(c) if c < lifr_ceil_m => Self::Lifr,
            Some(c) if c < ifr_ceil_m => Self::Ifr,
            Some(c) if c <= mvfr_ceil_m => Self::Mvfr,
            _ => Self::Vfr,
        };
        let vis_cat = if visibility_m < lifr_vis_m {
            Self::Lifr
        } else if visibility_m < ifr_vis_m {
            Self::Ifr
        } else if visibility_m <= mvfr_vis_m {
            Self::Mvfr
        } else {
            Self::Vfr
        };
        ceil_cat.max(vis_cat)
    }
}
