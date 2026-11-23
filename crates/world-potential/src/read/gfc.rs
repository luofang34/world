//! ICGEM `.gfc` static gravity-field files (ICGEM format 1.0/2.0).
//!
//! A header of `key value` lines ends at `end_of_head`; each following
//! `gfc L M C S [sigma_C sigma_S]` row is one fully normalised pair.
//! Time-variable keys are refused rather than silently dropped.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::{ParseError, ParseErrorKind, Triangles, err, index, number};
use crate::{GravityModel, TideSystem};

/// A `.gfc` file's model and the header facts the model does not carry.
#[derive(Clone, Debug, PartialEq)]
pub struct GfcModel {
    /// `modelname`.
    pub name: String,
    /// `errors` (`formal`, `calibrated`, …), empty when absent.
    pub errors: String,
    /// `max_degree` as the header states it, before any cap.
    pub header_degree: usize,
    /// Whether the file had no degree-0 row, so C̄00 was set to 1.
    pub central_term_implied: bool,
    /// The model, to the lower of the cap and the highest degree present.
    pub model: GravityModel<Vec<f64>>,
}

#[derive(Default)]
struct Header<'a> {
    name: Option<&'a str>,
    gm: Option<f64>,
    radius: Option<f64>,
    max_degree: Option<usize>,
    tide: Option<&'a str>,
    norm: Option<&'a str>,
    errors: Option<&'a str>,
}

/// Reads a `.gfc` text, keeping degrees up to `max_degree` when given.
///
/// The tide system is taken from `tide_system` (`tide_free`, `zero_tide`,
/// `mean_tide`; anything else is [`TideSystem::Unstated`]). C̄00 is the
/// file's when it has a degree-0 row and 1 otherwise.
///
/// # Errors
///
/// [`ParseError`] naming the line: a missing `end_of_head`, GM, `radius`
/// or `max_degree`; `norm` other than `fully_normalized`; a time-variable
/// or unknown key; a malformed row; or no rows at all.
pub fn read_gfc(text: &str, max_degree: Option<usize>) -> Result<GfcModel, ParseError> {
    let (header, body_start) = header(text)?;
    let missing = |field| err(body_start, ParseErrorKind::MissingHeader(field));
    let gm = header.gm.ok_or(missing("earth_gravity_constant"))?;
    let radius = header.radius.ok_or(missing("radius"))?;
    let header_degree = header.max_degree.ok_or(missing("max_degree"))?;
    if header.norm.is_some_and(|n| n != "fully_normalized") {
        return Err(err(0, ParseErrorKind::UnsupportedNormalization));
    }

    let rows = || {
        text.lines()
            .enumerate()
            .skip(body_start)
            .map(|(i, l)| (i + 1, l))
    };
    let cap = max_degree.unwrap_or(usize::MAX).min(header_degree);
    let mut present = None;
    for (line, l) in rows() {
        if let Some((n, _)) = row_indices(l, line)? {
            present = Some(present.unwrap_or(0).max(n.min(cap)));
        }
    }
    let degree = present.ok_or(err(0, ParseErrorKind::Empty))?;

    let mut triangles = Triangles::new(degree);
    let mut central = false;
    for (line, l) in rows() {
        let Some((n, m)) = row_indices(l, line)? else {
            continue;
        };
        let mut fields = l.split_whitespace().skip(3);
        let mut next = || fields.next().ok_or(err(line, ParseErrorKind::BadRow));
        let c = number(next()?, line)?;
        let s = number(next()?, line)?;
        triangles.put(n, m, c, s, line)?;
        central |= n == 0;
    }
    if !central {
        triangles.put(0, 0, 1.0, 0.0, 0)?;
    }
    let model = GravityModel::new(gm, radius, triangles.finish()?, tide(header.tide))
        .map_err(|e| err(0, ParseErrorKind::Model(e)))?;
    Ok(GfcModel {
        name: header.name.unwrap_or("").to_string(),
        errors: header.errors.unwrap_or("").to_string(),
        header_degree,
        central_term_implied: !central,
        model,
    })
}

fn tide(value: Option<&str>) -> TideSystem {
    match value {
        Some("tide_free") => TideSystem::TideFree,
        Some("zero_tide") => TideSystem::ZeroTide,
        Some("mean_tide") => TideSystem::MeanTide,
        _ => TideSystem::Unstated,
    }
}

/// The header fields and the index of the first line after `end_of_head`.
fn header(text: &str) -> Result<(Header<'_>, usize), ParseError> {
    let mut h = Header::default();
    for (i, l) in text.lines().enumerate() {
        let line = i + 1;
        let mut fields = l.split_whitespace();
        let (Some(key), value) = (fields.next(), fields.next()) else {
            continue;
        };
        match (key, value) {
            ("end_of_head", _) => return Ok((h, line)),
            ("modelname", v) => h.name = v,
            ("earth_gravity_constant" | "gravity_constant", Some(v)) => {
                h.gm = Some(number(v, line)?);
            }
            ("radius", Some(v)) => h.radius = Some(number(v, line)?),
            ("max_degree", Some(v)) => h.max_degree = Some(index(v, line)?),
            ("tide_system", v) => h.tide = v,
            ("norm", v) => h.norm = v,
            ("errors", v) => h.errors = v,
            _ => {}
        }
    }
    Err(err(0, ParseErrorKind::MissingHeader("end_of_head")))
}

/// `(L, M)` of a `gfc` row, `None` for a blank line.
fn row_indices(l: &str, line: usize) -> Result<Option<(usize, usize)>, ParseError> {
    let mut fields = l.split_whitespace();
    match fields.next() {
        None => Ok(None),
        Some("gfc") => {
            let mut next = || fields.next().ok_or(err(line, ParseErrorKind::BadRow));
            let n = index(next()?, line)?;
            let m = index(next()?, line)?;
            if m > n {
                return Err(err(line, ParseErrorKind::BadRow));
            }
            Ok(Some((n, m)))
        }
        Some(_) => Err(err(line, ParseErrorKind::UnsupportedKey)),
    }
}
