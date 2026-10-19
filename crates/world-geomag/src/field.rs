//! The model and what an evaluation gives.

use libm::sqrt;
use world_core::{DatasetId, DecimalYear, Provenance, Uncertainty, ValidityWindow};

/// A geomagnetic main-field model: which one, its epoch, the dates it
/// holds for and the published file it was generated from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeomagModel {
    /// The model's name and edition.
    pub dataset: DatasetId,
    /// The model epoch.
    pub epoch: DecimalYear,
    /// The dates the model is valid for.
    pub validity: ValidityWindow,
    /// The coefficient file the model was generated from.
    pub provenance: Provenance,
}

const WMM2025_DATASET: DatasetId = DatasetId {
    name: "WMM",
    edition: "WMM2025",
};

/// The World Magnetic Model 2025.
///
/// A product of the US National Geospatial-Intelligence Agency and the UK
/// Defence Geographic Centre, produced by NOAA NCEI and the British
/// Geological Survey.
pub const WMM2025: GeomagModel = GeomagModel {
    dataset: WMM2025_DATASET,
    epoch: DecimalYear(2025.0),
    validity: ValidityWindow {
        from: DecimalYear(2025.0),
        until: DecimalYear(2030.0),
    },
    provenance: Provenance {
        dataset: WMM2025_DATASET,
        publisher: "NOAA NCEI / British Geological Survey",
        source_url: "https://www.ncei.noaa.gov/products/world-magnetic-model",
        source_file: "WMM2025COF.zip: WMM.COF (header 2025.0 WMM-2025 11/13/2024)",
        source_sha256: "dfa8597825af4e0b87ff4198a5b4fb661b3c49f4cd090cd0164e0259b075582f",
        terms: "public domain",
    },
};

/// The model's one-sigma uncertainty against a measurement, from the
/// WMM2025 error model (NOAA NCEI, "Accuracy, limitations and error
/// model"). It covers the model's omission of crustal and disturbance
/// fields, so it is the uncertainty of a field measured on the ground,
/// not of the synthesis arithmetic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldUncertainty {
    /// North component (X), nT.
    pub north_nt: Uncertainty,
    /// East component (Y), nT.
    pub east_nt: Uncertainty,
    /// Down component (Z), nT.
    pub down_nt: Uncertainty,
    /// Horizontal intensity (H), nT.
    pub horizontal_nt: Uncertainty,
    /// Total intensity (F), nT.
    pub total_nt: Uncertainty,
    /// Inclination (I), degrees.
    pub inclination_deg: Uncertainty,
    /// Declination (D), degrees. It grows where the horizontal field is
    /// weak, near the magnetic poles.
    pub declination_deg: Uncertainty,
}

impl FieldUncertainty {
    /// The WMM2025 error model where the horizontal intensity is
    /// `horizontal_nt`.
    pub(crate) fn wmm2025(horizontal_nt: f64) -> Self {
        let sigma = |one_sigma| Uncertainty { one_sigma };
        let weak_horizontal = 5_417.0 / horizontal_nt;
        Self {
            north_nt: sigma(137.0),
            east_nt: sigma(89.0),
            down_nt: sigma(141.0),
            horizontal_nt: sigma(133.0),
            total_nt: sigma(138.0),
            inclination_deg: sigma(0.20),
            declination_deg: sigma(sqrt(0.26 * 0.26 + weak_horizontal * weak_horizontal)),
        }
    }
}

/// The main field at one position and date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MagneticField {
    /// North component (X), nT.
    pub north_nt: f64,
    /// East component (Y), nT.
    pub east_nt: f64,
    /// Down component (Z), nT.
    pub down_nt: f64,
    /// Horizontal intensity (H), nT.
    pub horizontal_nt: f64,
    /// Total intensity (F), nT.
    pub total_nt: f64,
    /// Declination (D): angle from true north to the horizontal field,
    /// positive east, degrees.
    pub declination_deg: f64,
    /// Inclination (I): angle from the horizontal to the field, positive
    /// down, degrees.
    pub inclination_deg: f64,
    /// The model's one-sigma uncertainty at this position.
    pub uncertainty: FieldUncertainty,
    /// The model that produced this field.
    pub model: GeomagModel,
}

impl MagneticField {
    /// Declination in radians, positive east.
    pub fn declination_rad(&self) -> f64 {
        self.declination_deg.to_radians()
    }

    /// Inclination in radians, positive down.
    pub fn inclination_rad(&self) -> f64 {
        self.inclination_deg.to_radians()
    }
}
