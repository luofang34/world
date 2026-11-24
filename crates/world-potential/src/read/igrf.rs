//! The IGRF multi-epoch coefficient table (`igrfNNcoeffs.txt`).
//!
//! After `#` comment lines, a `c/s deg ord …` line names the model of
//! each column and a `g/h n m 1900.0 … 2025.0 2025-30` line gives each
//! column's epoch, the last column being the secular variation of the
//! final epoch. Each row is `g` or `h`, n, m, then one Schmidt
//! semi-normalised value (nT) per epoch and the secular variation
//! (nT/year).

use alloc::vec;
use alloc::vec::Vec;

use world_core::{DecimalYear, ValidityWindow};

use super::{ParseError, ParseErrorKind, err, index, number};
use crate::{
    HarmonicCoefficients, MagneticModel, MagneticSeries, schmidt_to_geodesy, triangular_index,
    triangular_len,
};

/// IGRF predicts five years past its last epoch.
const FORECAST_YEARS: f64 = 5.0;

struct Row {
    sine: bool,
    n: usize,
    m: usize,
    values: Vec<f64>,
}

/// Reads an IGRF coefficient table into one segment per epoch interval:
/// each segment's secular variation is the difference to the next epoch
/// over the interval (IGRF's linear interpolation), and the last uses the
/// published secular variation for five years.
///
/// # Errors
///
/// [`ParseError`] naming the line: no epoch line, a row whose value count
/// differs from it, a malformed number, or no rows.
pub fn read_igrf_table(text: &str, radius_m: f64) -> Result<MagneticSeries, ParseError> {
    let mut epochs: Option<Vec<f64>> = None;
    let mut rows = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let line = i + 1;
        let fields: Vec<&str> = l.split_whitespace().collect();
        match fields.first().copied() {
            None | Some("c/s") => {}
            Some(f) if f.starts_with('#') => {}
            Some("g/h") => {
                // Every column but the trailing secular-variation one.
                let columns = fields.get(3..fields.len().saturating_sub(1)).unwrap_or(&[]);
                let parsed: Result<Vec<f64>, _> = columns.iter().map(|c| number(c, line)).collect();
                epochs = Some(parsed?);
            }
            Some(kind @ ("g" | "h")) => {
                let expected = epochs
                    .as_ref()
                    .ok_or(err(line, ParseErrorKind::MissingHeader("g/h")))?
                    .len()
                    + 1;
                let values = fields.get(3..).unwrap_or(&[]);
                if values.len() != expected {
                    return Err(err(line, ParseErrorKind::BadRow));
                }
                let f = |i: usize| {
                    fields
                        .get(i)
                        .copied()
                        .ok_or(err(line, ParseErrorKind::BadRow))
                };
                let (n, m) = (index(f(1)?, line)?, index(f(2)?, line)?);
                if m > n {
                    return Err(err(line, ParseErrorKind::BadRow));
                }
                let values: Result<Vec<f64>, _> = values.iter().map(|v| number(v, line)).collect();
                rows.push(Row {
                    sine: kind == "h",
                    n,
                    m,
                    values: values?,
                });
            }
            Some(_) => return Err(err(line, ParseErrorKind::BadRow)),
        }
    }
    let epochs = epochs.ok_or(err(0, ParseErrorKind::MissingHeader("g/h")))?;
    if rows.is_empty() || epochs.is_empty() {
        return Err(err(0, ParseErrorKind::Empty));
    }
    segments(&epochs, &rows, radius_m)
}

/// Column `j` of every row as fully normalised triangles.
fn column(rows: &[Row], degree: usize, j: usize) -> (Vec<f64>, Vec<f64>) {
    let len = triangular_len(degree);
    let (mut c, mut s) = (vec![0.0; len], vec![0.0; len]);
    for r in rows {
        let v = schmidt_to_geodesy(r.n, r.values.get(j).copied().unwrap_or(0.0));
        let target = if r.sine { &mut s } else { &mut c };
        if let Some(slot) = target.get_mut(triangular_index(r.n, r.m)) {
            *slot = v;
        }
    }
    (c, s)
}

fn segments(epochs: &[f64], rows: &[Row], radius_m: f64) -> Result<MagneticSeries, ParseError> {
    let degree = rows.iter().map(|r| r.n).max().unwrap_or(0);
    let model_error = |e| err(0, ParseErrorKind::Model(e));
    let mut segments = Vec::with_capacity(epochs.len());
    for (j, &epoch) in epochs.iter().enumerate() {
        let (c, s) = column(rows, degree, j);
        let next = epochs.get(j + 1).copied();
        let (dc, ds, until) = match next {
            Some(until) => {
                let (c1, s1) = column(rows, degree, j + 1);
                let years = until - epoch;
                let rate = |a: &[f64], b: &[f64]| -> Vec<f64> {
                    a.iter().zip(b).map(|(x0, x1)| (x1 - x0) / years).collect()
                };
                (rate(&c, &c1), rate(&s, &s1), until)
            }
            None => {
                let (dc, ds) = column(rows, degree, j + 1);
                (dc, ds, epoch + FORECAST_YEARS)
            }
        };
        let main = HarmonicCoefficients::new(degree, c, s).map_err(model_error)?;
        let secular = HarmonicCoefficients::new(degree, dc, ds).map_err(model_error)?;
        let validity = ValidityWindow {
            from: DecimalYear(epoch),
            until: DecimalYear(until),
        };
        segments.push(
            MagneticModel::new(radius_m, DecimalYear(epoch), validity, main, secular)
                .map_err(model_error)?,
        );
    }
    Ok(MagneticSeries::new(segments))
}
