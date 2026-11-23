//! PDS SHADR (Spherical Harmonic ASCII Data Record) tables, the archive
//! format of the GRAIL lunar (GRGM1200A) and MRO Mars (MRO120D) fields.
//!
//! The first record is a comma-separated header: reference radius (km),
//! GM (km³/s²), its uncertainty, degree, order, normalisation state (1 =
//! fully normalised), reference longitude and latitude (degrees). Each
//! following record is `n, m, C, S, sigma_C, sigma_S`. Records are padded
//! with spaces and end in CR LF. The tables start at degree 1 with no
//! degree-0 row: the central term is GM, so C̄00 is set to 1.

use alloc::vec::Vec;

use super::{ParseError, ParseErrorKind, Triangles, err, index, number};
use crate::{GravityModel, TideSystem};

/// A SHADR table's model and its header facts.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadrModel {
    /// The uncertainty of GM, m³/s².
    pub gm_uncertainty_m3ps2: f64,
    /// The degree the header states, before any cap.
    pub header_degree: usize,
    /// The model, to the lower of the cap and the header degree. SHADR
    /// carries no tide-system flag: [`TideSystem::Unstated`].
    pub model: GravityModel<Vec<f64>>,
}

const KM: f64 = 1e3;
const KM3: f64 = 1e9;

/// Reads a SHADR table, keeping degrees up to `max_degree` when given.
///
/// # Errors
///
/// [`ParseError`] naming the line: a header with fewer than eight fields,
/// a normalisation state other than 1, a non-zero reference longitude or
/// latitude (a rotated expansion), a malformed row, or no rows at all.
pub fn read_shadr(text: &str, max_degree: Option<usize>) -> Result<ShadrModel, ParseError> {
    let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
    let (line, head) = lines
        .by_ref()
        .find(|(_, l)| !l.trim().is_empty())
        .ok_or(err(0, ParseErrorKind::Empty))?;
    let h: Vec<&str> = head.split(',').map(str::trim).collect();
    let field = |i: usize| h.get(i).copied().ok_or(err(line, ParseErrorKind::BadRow));
    let radius_m = number(field(0)?, line)? * KM;
    let gm = number(field(1)?, line)? * KM3;
    let gm_sigma = number(field(2)?, line)? * KM3;
    let header_degree = index(field(3)?, line)?;
    if index(field(5)?, line)? != 1 {
        return Err(err(line, ParseErrorKind::UnsupportedNormalization));
    }
    if number(field(6)?, line)? != 0.0 || number(field(7)?, line)? != 0.0 {
        return Err(err(line, ParseErrorKind::UnsupportedReference));
    }

    let degree = max_degree.unwrap_or(header_degree).min(header_degree);
    let mut triangles = Triangles::new(degree);
    triangles.put(0, 0, 1.0, 0.0, line)?;
    let mut any_row = false;
    for (line, l) in lines {
        if l.trim().is_empty() {
            continue;
        }
        let mut fields = l.split(',').map(str::trim);
        let mut next = || fields.next().ok_or(err(line, ParseErrorKind::BadRow));
        let n = index(next()?, line)?;
        let m = index(next()?, line)?;
        if n > degree {
            continue;
        }
        let c = number(next()?, line)?;
        let s = number(next()?, line)?;
        triangles.put(n, m, c, s, line)?;
        any_row = true;
    }
    if !any_row && degree > 0 {
        return Err(err(0, ParseErrorKind::Empty));
    }
    let model = GravityModel::new(gm, radius_m, triangles.finish()?, TideSystem::Unstated)
        .map_err(|e| err(0, ParseErrorKind::Model(e)))?;
    Ok(ShadrModel {
        gm_uncertainty_m3ps2: gm_sigma,
        header_degree,
        model,
    })
}
