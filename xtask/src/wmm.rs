//! The WMM2025 coefficient table and the file it is generated from.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::XtaskError;

const COF: &str = "crates/world-geomag/data/WMM.COF";
const TABLE: &str = "crates/world-geomag/src/coefficients.rs";
const PROVENANCE: &str = "crates/world-geomag/src/field.rs";
const CHECKSUM_FIELD: &str = "source_sha256: \"";

/// The coefficient table module a `WMM.COF` text generates.
///
/// The first line is the model header; each following six-field line is
/// one `n m g h g_dot h_dot` row, up to the first line that is not (the
/// file ends with a line of nines).
pub(crate) fn generate(cof: &str) -> String {
    let rows: Vec<Vec<&str>> = cof
        .lines()
        .skip(1)
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .take_while(|fields| fields.len() == 6)
        .collect();
    let max_degree = rows
        .last()
        .and_then(|row| row.first())
        .copied()
        .unwrap_or("0");
    let mut out = String::from(concat!(
        "//! WMM2025 Gauss coefficients, as published.\n",
        "//!\n",
        "//! Transcribed from the coefficient file [`crate::WMM2025`]'s provenance\n",
        "//! names; that provenance carries the file's checksum.\n",
        "\n",
        "/// Maximum spherical-harmonic degree.\n",
    ));
    // Writing to a String cannot fail.
    writeln!(out, "pub(crate) const MAX_DEGREE: usize = {max_degree};").ok();
    out.push_str(concat!(
        "\n",
        "/// `(n, m, g, h, g_dot, h_dot)`: Schmidt semi-normalized coefficients in\n",
        "/// nT and their secular variation in nT/year.\n",
        "#[rustfmt::skip]\n",
    ));
    writeln!(
        out,
        "pub(crate) const COEFFICIENTS: [(usize, usize, f64, f64, f64, f64); {}] = [",
        rows.len()
    )
    .ok();
    for row in &rows {
        writeln!(out, "    ({}),", row.join(", ")).ok();
    }
    out.push_str("];\n");
    out
}

/// Fails unless `cof` is the file `provenance` names by checksum and
/// `table` is exactly what it generates.
pub(crate) fn verify(cof: &[u8], table: &str, provenance: &str) -> Result<(), XtaskError> {
    let recorded = provenance
        .split_once(CHECKSUM_FIELD)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(sha, _)| sha.to_owned())
        .ok_or_else(|| XtaskError::NoRecordedChecksum {
            path: PathBuf::from(PROVENANCE),
        })?;
    let actual = hex(&Sha256::digest(cof));
    if recorded != actual {
        return Err(XtaskError::ChecksumMismatch { recorded, actual });
    }
    let text = std::str::from_utf8(cof).map_err(|_| XtaskError::NotText {
        path: PathBuf::from(COF),
    })?;
    if generate(text) != table {
        return Err(XtaskError::TableDiffers {
            path: PathBuf::from(TABLE),
        });
    }
    Ok(())
}

/// `check-wmm` over the committed files.
pub(crate) fn check(root: &Path) -> Result<(), XtaskError> {
    verify(
        &read(root, COF)?,
        &read_text(root, TABLE)?,
        &read_text(root, PROVENANCE)?,
    )
}

/// `gen-wmm`: rewrite the table from the committed file.
pub(crate) fn write(root: &Path) -> Result<(), XtaskError> {
    let cof = read(root, COF)?;
    let text = std::str::from_utf8(&cof).map_err(|_| XtaskError::NotText {
        path: PathBuf::from(COF),
    })?;
    let path = root.join(TABLE);
    std::fs::write(&path, generate(text)).map_err(|source| XtaskError::Io {
        action: "writing",
        path,
        source,
    })
}

fn read(root: &Path, relative: &str) -> Result<Vec<u8>, XtaskError> {
    let path = root.join(relative);
    std::fs::read(&path).map_err(|source| XtaskError::Io {
        action: "reading",
        path,
        source,
    })
}

fn read_text(root: &Path, relative: &str) -> Result<String, XtaskError> {
    let bytes = read(root, relative)?;
    String::from_utf8(bytes).map_err(|_| XtaskError::NotText {
        path: root.join(relative),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").ok();
        out
    })
}

#[cfg(test)]
mod tests;
