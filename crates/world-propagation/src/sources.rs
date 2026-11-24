//! The Recommendations the models implement, identified by the published
//! PDF's checksum so a reader can fetch the exact text the code follows.

use world_core::{DatasetId, Provenance};

/// The terms printed on every ITU-R Recommendation; the formulae are
/// implemented here, the text is not redistributed.
const ITU_TERMS: &str =
    "(c) ITU, all rights reserved; free download from the ITU-R Recommendation page";

/// Recommendation ITU-R P.526-16 (11/2025), *Propagation by diffraction*.
pub const P526_16: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.526",
        edition: "P.526-16 (11/2025)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.526-16-202511-I/en",
    source_file: "R-REC-P.526-16-202511-I!!PDF-E.pdf",
    source_sha256: "7647c3cb3873dc0389a7007f487728359551ed82468a5677b7cd09430875f23d",
    terms: ITU_TERMS,
};

/// Recommendation ITU-R S.465-6 (01/2010), earth-station reference
/// radiation pattern, 2 to 31 GHz.
pub const S465_6: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R S.465",
        edition: "S.465-6 (01/2010)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-S.465-6-201001-I/en",
    source_file: "R-REC-S.465-6-201001-I!!PDF-E.pdf",
    source_sha256: "a813d82235c24ad52681634d5d8a1275da621317ddb1a6754cd983643024f98a",
    terms: ITU_TERMS,
};

/// Recommendation ITU-R S.580-6 (01/2004), design-objective radiation
/// diagrams for earth stations operating with geostationary satellites.
pub const S580_6: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R S.580",
        edition: "S.580-6 (01/2004)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-S.580-6-200401-I/en",
    source_file: "R-REC-S.580-6-200401-I!!PDF-E.pdf",
    source_sha256: "529186072dad7c8fad2e5df3f5d242832d84a9e2297ac82985f2230f086f7bec",
    terms: ITU_TERMS,
};

/// Recommendation ITU-R F.1245-3 (01/2019), average radiation pattern of
/// point-to-point fixed-wireless antennas, 1 to 86 GHz.
pub const F1245_3: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R F.1245",
        edition: "F.1245-3 (01/2019)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-F.1245-3-201901-I/en",
    source_file: "R-REC-F.1245-3-201901-I!!PDF-E.pdf",
    source_sha256: "01ff882892a1eb6d2dc65075bca2814e67b315a94d45cb0f94bd46a503030730",
    terms: ITU_TERMS,
};

/// The NTIA/ITS reference implementation of the Irregular Terrain Model,
/// version 1.4, that [`crate::itm`] ports. The commit pins the sources;
/// the checksum is of its `LICENSE.md`, which states the terms.
pub const NTIA_ITM: Provenance = Provenance {
    dataset: DatasetId {
        name: "NTIA ITM",
        edition: "v1.4, commit 62168e686e8af4a234cf3f8067d80b543a5d52ef",
    },
    publisher: "U.S. Department of Commerce, NTIA Institute for Telecommunication Sciences",
    source_url: "https://github.com/NTIA/itm/tree/62168e686e8af4a234cf3f8067d80b543a5d52ef",
    source_file: "LICENSE.md",
    source_sha256: "2e66fd45776db7cb35d0f087fbadaf193234b383ac89f6eac45fe3f1f7447147",
    terms: "US-government work not subject to copyright in the US (17 U.S.C. 105); worldwide royalty-free grant to prepare and distribute derivative works; provided AS IS; modified works should note their changes and acknowledge NTIA",
};

/// Notice for ITU-R Recommendations whose published PDFs are not bundled
/// locally; callers should fetch and verify the official PDF from ITU.
const UNBUNDLED_SHA256: &str = "not verified locally (official ITU-R PDF not bundled)";

/// Recommendation ITU-R P.531-15 (08/2023), *Ionospheric propagation data and
/// prediction methods required for the design of satellite networks and
/// systems*.
pub const P531_15: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.531",
        edition: "P.531-15 (08/2023)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.531-15-202308-I/en",
    source_file: "R-REC-P.531-15-202308-I!!PDF-E.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: ITU_TERMS,
};

/// Recommendation ITU-R P.618-14 (08/2023), *Propagation data and prediction
/// methods required for the design of Earth-space telecommunication systems*.
pub const P618_14: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.618",
        edition: "P.618-14 (08/2023)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.618-14-202308-I/en",
    source_file: "R-REC-P.618-14-202308-I!!PDF-E.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: ITU_TERMS,
};

/// Recommendation ITU-R P.676-10 (09/2013), *Attenuation by atmospheric gases*
/// (Annex 2 dry-air and equivalent-height formulae; water-vapour curve fit
/// follows P.676-5 Annex 2).
pub const P676_10: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.676",
        edition: "P.676-10 (09/2013) / P.676-5 (02/2001) Annex 2",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.676-10-201309-S/en",
    source_file: "R-REC-P.676-10-201309-S!!PDF-E.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: ITU_TERMS,
};

/// Recommendation ITU-R P.838-3 (03/2005), *Specific attenuation model for rain
/// for use in prediction methods*.
pub const P838_3: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.838",
        edition: "P.838-3 (03/2005)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.838-3-200503-I/en",
    source_file: "R-REC-P.838-3-200503-I!!PDF-E.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: ITU_TERMS,
};

/// Recommendation ITU-R P.839-4 (09/2013), *Rain height model for prediction
/// methods*.
pub const P839_4: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.839",
        edition: "P.839-4 (09/2013)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.839-4-201309-I/en",
    source_file: "R-REC-P.839-4-201309-I!!PDF-E.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: ITU_TERMS,
};

/// Recommendation ITU-R P.840-9 (08/2023), *Attenuation due to clouds and fog*
/// (Rayleigh double-Debye liquid-water specific attenuation).
pub const P840_9: Provenance = Provenance {
    dataset: DatasetId {
        name: "ITU-R P.840",
        edition: "P.840-9 (08/2023)",
    },
    publisher: "ITU Radiocommunication Sector",
    source_url: "https://www.itu.int/rec/R-REC-P.840-9-202308-I/en",
    source_file: "R-REC-P.840-9-202308-I!!PDF-E.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: ITU_TERMS,
};

/// NSMA Recommendation WG16.99.050 (05/1999), *Antenna Systems — Standard
/// Format for Digitized Antenna Patterns* (ANSI/TIA/EIA-804).
pub const NSMA_WG16_99: Provenance = Provenance {
    dataset: DatasetId {
        name: "NSMA WG16.99.050",
        edition: "WG16.99.050 (1999-05-20)",
    },
    publisher: "National Spectrum Management Association",
    source_url: "https://www.nsma.org/wp-content/uploads/2016/05/wg16_99_050.pdf",
    source_file: "wg16_99_050.pdf",
    source_sha256: UNBUNDLED_SHA256,
    terms: "NSMA published recommendation; implemented from the ASCII record specification",
};
