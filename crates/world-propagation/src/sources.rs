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
