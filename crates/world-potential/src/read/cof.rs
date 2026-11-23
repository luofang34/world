//! WMM `.COF` coefficient files.
//!
//! This is the WMM release layout only; the multi-model geomag70 `.COF`
//! layout differs, and IGRF is read from its coefficient table instead
//! ([`crate::read_igrf_table`]).
//!
//! The first line is `epoch model-name release-date`; each following
//! `n m g h g_dot h_dot` line is one Schmidt semi-normalised pair in nT
//! with its secular variation in nT/year, up to a line of nines or the
//! end of the text.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use world_core::{DecimalYear, ValidityWindow};

use super::{ParseError, ParseErrorKind, Triangles, err, index, number};
use crate::{FieldError, MagneticModel, schmidt_to_geodesy};

/// One Schmidt semi-normalised Gauss coefficient pair and its secular
/// variation, as main-field files publish them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaussRow {
    /// Degree.
    pub n: usize,
    /// Order.
    pub m: usize,
    /// gnm, nT.
    pub g: f64,
    /// hnm, nT.
    pub h: f64,
    /// dgnm/dt, nT/year.
    pub g_dot: f64,
    /// dhnm/dt, nT/year.
    pub h_dot: f64,
}

/// A `.COF` file's model and header.
#[derive(Clone, Debug, PartialEq)]
pub struct CofModel {
    /// The model name, such as `WMM-2025`.
    pub name: String,
    /// The release date as written.
    pub release: String,
    /// The model, valid for five years from its epoch (the WMM and IGRF
    /// convention; [`MagneticModel::with_validity`] restates it).
    pub model: MagneticModel<Vec<f64>>,
}

impl MagneticModel<Vec<f64>> {
    /// A model from Schmidt semi-normalised `rows`, converted with
    /// [`schmidt_to_geodesy`]; its degree is the highest row's.
    ///
    /// # Errors
    ///
    /// As [`MagneticModel::new`], and [`FieldError::CoefficientStorage`]
    /// for a row with m > n.
    pub fn from_schmidt(
        radius_m: f64,
        epoch: DecimalYear,
        validity: ValidityWindow,
        rows: &[GaussRow],
    ) -> Result<Self, FieldError> {
        let degree = rows.iter().map(|r| r.n).max().unwrap_or(0);
        let mut main = Triangles::new(degree);
        let mut secular = Triangles::new(degree);
        for r in rows {
            let k = |v| schmidt_to_geodesy(r.n, v);
            let bad = |_| FieldError::CoefficientStorage {
                degree,
                expected: crate::triangular_len(degree),
                cosine: 0,
                sine: 0,
            };
            main.put(r.n, r.m, k(r.g), k(r.h), 0).map_err(bad)?;
            secular
                .put(r.n, r.m, k(r.g_dot), k(r.h_dot), 0)
                .map_err(bad)?;
        }
        let model_error = |e: ParseError| match e.kind {
            ParseErrorKind::Model(e) => e,
            _ => FieldError::InvalidModel,
        };
        let main = main.finish().map_err(model_error)?;
        let secular = secular.finish().map_err(model_error)?;
        Self::new(radius_m, epoch, validity, main, secular)
    }
}

/// Reads a `.COF` text for a model of reference radius `radius_m` (the
/// file does not state it; WMM and IGRF use 6371.2 km).
///
/// # Errors
///
/// [`ParseError`] naming the line: no header epoch, a malformed row, no
/// rows, or constants the model refuses.
pub fn read_cof(text: &str, radius_m: f64) -> Result<CofModel, ParseError> {
    let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
    let (line, head) = lines
        .by_ref()
        .find(|(_, l)| !l.trim().is_empty())
        .ok_or(err(0, ParseErrorKind::Empty))?;
    let mut head_fields = head.split_whitespace();
    let epoch = head_fields
        .next()
        .ok_or(err(line, ParseErrorKind::MissingHeader("epoch")))
        .and_then(|e| number(e, line))?;
    let name = head_fields.next().unwrap_or("").to_string();
    let release = head_fields.next().unwrap_or("").to_string();

    let mut rows = Vec::new();
    for (line, l) in lines {
        let fields: Vec<&str> = l.split_whitespace().collect();
        if fields.len() != 6 {
            break;
        }
        let f = |i: usize| {
            fields
                .get(i)
                .copied()
                .ok_or(err(line, ParseErrorKind::BadRow))
        };
        let (n, m) = (index(f(0)?, line)?, index(f(1)?, line)?);
        if m > n {
            return Err(err(line, ParseErrorKind::BadRow));
        }
        rows.push(GaussRow {
            n,
            m,
            g: number(f(2)?, line)?,
            h: number(f(3)?, line)?,
            g_dot: number(f(4)?, line)?,
            h_dot: number(f(5)?, line)?,
        });
    }
    if rows.is_empty() {
        return Err(err(0, ParseErrorKind::Empty));
    }
    let validity = ValidityWindow {
        from: DecimalYear(epoch),
        until: DecimalYear(epoch + 5.0),
    };
    let model = MagneticModel::from_schmidt(radius_m, DecimalYear(epoch), validity, &rows)
        .map_err(|e| err(0, ParseErrorKind::Model(e)))?;
    Ok(CofModel {
        name,
        release,
        model,
    })
}
