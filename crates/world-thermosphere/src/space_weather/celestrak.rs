//! Readers for CelesTrak space-weather CSV (`SW-Last5Years.csv` /
//! `SW-All.csv`) and legacy CSSI fixed-width (`SW-All.txt`) files.
//!
//! No CelesTrak files or excerpts are bundled in this crate; callers supply
//! text obtained in accordance with CelesTrak's usage policy. Sunspot number
//! (`ISN`) columns are skipped and never retained.

mod csv;
mod legacy;

use world_core::{DatasetId, Provenance};

pub use csv::parse_celestrak_csv;
pub use legacy::{parse_celestrak_legacy, parse_celestrak_legacy_with_monthly};

const CELESTRAK_TERMS: &str = "redistributed by CelesTrak from GFZ, DRAO/NRCan, NOAA SWPC, SILSO, and NASA; subject to CelesTrak usage policy (no automated high-frequency bulk redistribution); ISN column excluded";

/// Provenance metadata for CelesTrak space-weather CSV feeds.
pub const CELESTRAK_CSV_PROVENANCE: Provenance = Provenance {
    dataset: DatasetId {
        name: "CelesTrak Space Weather CSV",
        edition: "SW-Last5Years.csv / SW-All.csv",
    },
    publisher: "CelesTrak (Dr. T.S. Kelso)",
    source_url: "https://celestrak.org/SpaceData/SpaceWx-format.php",
    source_file: "SW-Last5Years.csv",
    source_sha256: "dynamic (live feed; caller-supplied text)",
    terms: CELESTRAK_TERMS,
};

/// Provenance metadata for CelesTrak legacy CSSI space-weather files.
pub const CELESTRAK_LEGACY_PROVENANCE: Provenance = Provenance {
    dataset: DatasetId {
        name: "CelesTrak CSSI Space Weather",
        edition: "CssiSpaceWeather v1.2",
    },
    publisher: "CelesTrak (Dr. T.S. Kelso)",
    source_url: "https://celestrak.org/SpaceData/SpaceWx-format.php",
    source_file: "SW-All.txt",
    source_sha256: "dynamic (live feed; caller-supplied text)",
    terms: CELESTRAK_TERMS,
};
