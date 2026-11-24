//! Readers for published coefficient files, from text already in memory.
//!
//! Fetching and caching files is a data-plane concern; these take `&str`
//! so a model never touches a filesystem. Every reader can stop at a
//! lower degree than the file holds, so a degree-2190 file can feed a
//! degree-360 model without the rest ever being stored.

mod cof;
mod gfc;
mod igrf;
mod shadr;

use core::fmt;

pub use cof::{CofModel, GaussRow, read_cof};
pub use gfc::{GfcModel, read_gfc};
pub use igrf::read_igrf_table;
pub use shadr::{ShadrModel, read_shadr};

use alloc::vec;
use alloc::vec::Vec;

use crate::{FieldError, HarmonicCoefficients, triangular_index, triangular_len};

/// What was wrong with a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParseErrorKind {
    /// A required header field is absent.
    MissingHeader(&'static str),
    /// A field that should be a number is not.
    BadNumber,
    /// A row has too few fields, or an order above its degree.
    BadRow,
    /// A `.gfc` key this reader does not evaluate, such as the
    /// time-variable `gfct`, `trnd`, `acos` and `asin`.
    UnsupportedKey,
    /// Coefficients that are not fully normalised.
    UnsupportedNormalization,
    /// An expansion about a rotated pole (non-zero reference latitude or
    /// longitude).
    UnsupportedReference,
    /// No coefficient rows.
    Empty,
    /// The header's constants or the coefficient layout were refused by
    /// the model.
    Model(FieldError),
}

/// Why a file was not read, and on which line (1-based; 0 when the file
/// as a whole is at fault).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParseError {
    /// The offending line.
    pub line: usize,
    /// What was wrong with it.
    pub kind: ParseErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ParseErrorKind::MissingHeader(field) => {
                return write!(f, "line {}: header field `{field}` missing", self.line);
            }
            ParseErrorKind::Model(e) => return write!(f, "line {}: {e}", self.line),
            ParseErrorKind::BadNumber => "a field is not a number",
            ParseErrorKind::BadRow => "a row is malformed or has m > n",
            ParseErrorKind::UnsupportedKey => "time-variable or unknown key",
            ParseErrorKind::UnsupportedNormalization => "coefficients are not fully normalised",
            ParseErrorKind::UnsupportedReference => "the expansion is about a rotated pole",
            ParseErrorKind::Empty => "no coefficient rows",
        };
        write!(f, "line {}: {what}", self.line)
    }
}

impl core::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match &self.kind {
            ParseErrorKind::Model(e) => Some(e),
            _ => None,
        }
    }
}

fn err(line: usize, kind: ParseErrorKind) -> ParseError {
    ParseError { line, kind }
}

/// A number in Fortran or Rust notation (`1.0D-03` and `1.0e-03`).
fn number(token: &str, line: usize) -> Result<f64, ParseError> {
    let bad = err(line, ParseErrorKind::BadNumber);
    if !token.contains(['D', 'd']) {
        return token.parse().map_err(|_| bad);
    }
    let mut buf = [0u8; 64];
    let bytes = token.as_bytes();
    let out = buf.get_mut(..bytes.len()).ok_or(bad)?;
    for (o, b) in out.iter_mut().zip(bytes) {
        *o = if matches!(b, b'D' | b'd') { b'e' } else { *b };
    }
    core::str::from_utf8(out)
        .map_err(|_| bad)?
        .parse()
        .map_err(|_| bad)
}

fn index(token: &str, line: usize) -> Result<usize, ParseError> {
    token
        .parse()
        .map_err(|_| err(line, ParseErrorKind::BadNumber))
}

/// Cosine and sine triangles filled row by row, dropping rows above the
/// degree cap.
struct Triangles {
    degree: usize,
    cosine: Vec<f64>,
    sine: Vec<f64>,
}

impl Triangles {
    fn new(degree: usize) -> Self {
        let len = triangular_len(degree);
        Self {
            degree,
            cosine: vec![0.0; len],
            sine: vec![0.0; len],
        }
    }

    /// Stores `(c, s)` at `(n, m)` unless `n` is above the cap.
    fn put(&mut self, n: usize, m: usize, c: f64, s: f64, line: usize) -> Result<(), ParseError> {
        if m > n {
            return Err(err(line, ParseErrorKind::BadRow));
        }
        if n > self.degree {
            return Ok(());
        }
        let i = triangular_index(n, m);
        if let (Some(cs), Some(ss)) = (self.cosine.get_mut(i), self.sine.get_mut(i)) {
            *cs = c;
            *ss = s;
        }
        Ok(())
    }

    fn finish(self) -> Result<HarmonicCoefficients<Vec<f64>>, ParseError> {
        HarmonicCoefficients::new(self.degree, self.cosine, self.sine)
            .map_err(|e| err(0, ParseErrorKind::Model(e)))
    }
}
