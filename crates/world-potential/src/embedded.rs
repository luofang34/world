//! Coefficient files carried in the crate, with their provenance.
//!
//! Only small, low-degree sets are embedded; high-degree files are read
//! at run time with [`crate::read_gfc`] and [`crate::read_shadr`].

use alloc::vec::Vec;

use world_core::{DatasetId, Provenance};

use crate::{GravityModel, MagneticSeries, ParseError, read_gfc, read_igrf_table};

/// Geomagnetic reference radius of IGRF and WMM, m.
const GEOMAGNETIC_RADIUS_M: f64 = 6_371_200.0;

const IGRF14_TEXT: &str = include_str!("../data/igrf14coeffs.txt");
const EGM96_TEXT: &str = include_str!("../data/EGM96_degree36.gfc");

/// The degree of the embedded EGM96 excerpt.
const EGM96_EXCERPT_DEGREE: usize = 36;

const IGRF14_DATASET: DatasetId = DatasetId {
    name: "IGRF",
    edition: "IGRF-14",
};

/// The 14th-generation International Geomagnetic Reference Field: the
/// committed `data/igrf14coeffs.txt` is this file, unmodified.
pub const IGRF14: Provenance = Provenance {
    dataset: IGRF14_DATASET,
    publisher: "IAGA Division V Working Group V-MOD",
    source_url: "https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf14coeffs.txt",
    source_file: "igrf14coeffs.txt",
    source_sha256: "8f8d88403028fc4ee92c4f38d97b46e0a87e2cfc496045b43c9e26c1d6b0903c",
    terms: "not stated in the file; see the IAGA V-MOD IGRF page",
};

const EGM96_DATASET: DatasetId = DatasetId {
    name: "EGM96",
    edition: "EGM96",
};

/// The NASA GSFC / NIMA Earth Gravitational Model 1996 as ICGEM serves
/// it. The checksum is of the full degree-360 file; the committed
/// `data/EGM96_degree36.gfc` is its header and its `gfc` rows of degree
/// 36 and below, each line verbatim.
pub const EGM96_DEGREE_36: Provenance = Provenance {
    dataset: EGM96_DATASET,
    publisher: "NASA GSFC / NIMA, distributed by ICGEM (GFZ Potsdam)",
    source_url: "https://icgem.gfz-potsdam.de/getmodel/gfc/971b0a3b49a497910aad23cd85e066d4cd9af0aeafe7ce6301a696bed8570be3/EGM96.gfc",
    source_file: "EGM96.gfc (degree 360, tide_free), excerpt to degree 36",
    source_sha256: "5247a9e9c316dd2c8f8fd491d53be0e163cb5cb3676b021754240cc9e44cb43b",
    terms: "not stated in the file; see the ICGEM model page",
};

/// IGRF-14 from 1900.0 to 2030.0, one segment per five-year epoch.
///
/// # Errors
///
/// None for the committed file; the `Result` is the reader's.
pub fn igrf14() -> Result<MagneticSeries, ParseError> {
    Ok(read_igrf_table(IGRF14_TEXT, GEOMAGNETIC_RADIUS_M)?.with_provenance(IGRF14))
}

/// EGM96 to degree and order 36, tide-free, C̄00 = 1 (gravitation of
/// the whole Earth; drop the central term with
/// [`crate::Truncation::min_degree`]).
///
/// # Errors
///
/// None for the committed file; the `Result` is the reader's.
pub fn egm96_degree_36() -> Result<GravityModel<Vec<f64>>, ParseError> {
    let gfc = read_gfc(EGM96_TEXT, Some(EGM96_EXCERPT_DEGREE))?;
    Ok(gfc.model.with_provenance(EGM96_DEGREE_36))
}

#[cfg(test)]
mod tests;
