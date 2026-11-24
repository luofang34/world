//! Field and sample parsers for NSMA WG16.99.050 antenna patterns.

use crate::error::{PatternParseErrorKind, PropagationError};

use super::cut::{AntennaMetadata, CutPlane, DBD_TO_DBI, PatternSample};

#[derive(Clone, Copy)]
pub(crate) enum GainUnit {
    Dbi,
    Dbd,
}

#[derive(Clone, Copy)]
pub(crate) enum SampleUnit {
    Dbr,
    Dbi,
    Dbd,
    Lin,
}

pub(crate) fn nsma_err(line: usize, kind: PatternParseErrorKind) -> PropagationError {
    PropagationError::InvalidPatternFormat { line, kind }
}

pub(crate) fn parse_patcut(val: &str, line_num: usize) -> Result<CutPlane, PropagationError> {
    match val.trim().to_ascii_uppercase().as_str() {
        "AZ" | "H" | "AZIMUT" | "AZIMUTH" => Ok(CutPlane::Azimuth),
        "EL" | "V" | "ELEVAT" | "ELEVATION" => Ok(CutPlane::Elevation),
        _ => Err(nsma_err(
            line_num,
            PatternParseErrorKind::MalformedRecord("unsupported PATCUT"),
        )),
    }
}

pub(crate) fn parse_gunits(
    val: &str,
    line_num: usize,
) -> Result<(GainUnit, SampleUnit), PropagationError> {
    let up = val.trim().to_ascii_uppercase();
    let (gain_str, pat_str) = match up.split_once('/') {
        Some((a, b)) => (a.trim(), b.trim()),
        None => (up.as_str(), up.as_str()),
    };
    let gain_u = match gain_str {
        "DBI" | "DBR" => GainUnit::Dbi,
        "DBD" => GainUnit::Dbd,
        _ => return Err(nsma_err(line_num, PatternParseErrorKind::UnsupportedUnits)),
    };
    let pat_u = match pat_str {
        "DBR" => SampleUnit::Dbr,
        "DBI" => SampleUnit::Dbi,
        "DBD" => SampleUnit::Dbd,
        "LIN" => SampleUnit::Lin,
        _ => return Err(nsma_err(line_num, PatternParseErrorKind::UnsupportedUnits)),
    };
    Ok((gain_u, pat_u))
}

pub(crate) fn parse_nsma_sample(
    line: &str,
    prev_angle: Option<f64>,
    sample_idx: usize,
    unit: SampleUnit,
    gain_max_dbi: f64,
    line_num: usize,
) -> Result<PatternSample, PropagationError> {
    let (a_tok, v_tok) = split_sample_pair(line, line_num)?;
    let angle_deg: f64 = a_tok
        .parse()
        .map_err(|_| nsma_err(line_num, PatternParseErrorKind::BadNumber("angle")))?;
    let raw_val: f64 = v_tok
        .parse()
        .map_err(|_| nsma_err(line_num, PatternParseErrorKind::BadNumber("gain")))?;
    if !angle_deg.is_finite() || !(-180.0..=360.0).contains(&angle_deg) {
        return Err(nsma_err(
            line_num,
            PatternParseErrorKind::InvalidAngle { angle_deg },
        ));
    }
    if prev_angle.is_some_and(|p| angle_deg <= p) {
        return Err(nsma_err(
            line_num,
            PatternParseErrorKind::NonMonotonicAngle { index: sample_idx },
        ));
    }
    if !raw_val.is_finite() {
        return Err(nsma_err(
            line_num,
            PatternParseErrorKind::NonFiniteValue { index: sample_idx },
        ));
    }
    let attenuation_db = match unit {
        SampleUnit::Dbr => -raw_val,
        SampleUnit::Dbi => gain_max_dbi - raw_val,
        SampleUnit::Dbd => gain_max_dbi - (raw_val + DBD_TO_DBI),
        SampleUnit::Lin => {
            if raw_val <= 0.0 {
                return Err(nsma_err(
                    line_num,
                    PatternParseErrorKind::NonFiniteValue { index: sample_idx },
                ));
            }
            -20.0 * libm::log10(raw_val)
        }
    };
    Ok(PatternSample {
        angle_deg,
        attenuation_db,
    })
}

fn split_sample_pair(line: &str, line_num: usize) -> Result<(&str, &str), PropagationError> {
    if let Some((a, v)) = line.split_once(',') {
        return Ok((a.trim(), v.trim()));
    }
    let mut toks = line.split_whitespace();
    let a = toks.next().ok_or_else(|| {
        nsma_err(
            line_num,
            PatternParseErrorKind::MalformedRecord("missing angle"),
        )
    })?;
    let v = toks.next().ok_or_else(|| {
        nsma_err(
            line_num,
            PatternParseErrorKind::MalformedRecord("missing value"),
        )
    })?;
    if toks.next().is_some() {
        return Err(nsma_err(
            line_num,
            PatternParseErrorKind::MalformedRecord("trailing tokens in pattern sample"),
        ));
    }
    Ok((a, v))
}

pub(crate) fn update_mid_freq(meta: &mut AntennaMetadata) {
    meta.frequency_mhz = match (meta.low_frequency_mhz, meta.high_frequency_mhz) {
        (Some(lo), Some(hi)) => Some(0.5 * (lo + hi)),
        (Some(lo), None) => Some(lo),
        (None, Some(hi)) => Some(hi),
        (None, None) => None,
    };
}

pub(crate) fn parse_finite(
    val: &str,
    field: &'static str,
    line_num: usize,
) -> Result<f64, PropagationError> {
    let v: f64 = val
        .trim()
        .parse()
        .map_err(|_| nsma_err(line_num, PatternParseErrorKind::BadNumber(field)))?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(nsma_err(line_num, PatternParseErrorKind::BadNumber(field)))
    }
}

pub(crate) fn parse_positive(
    val: &str,
    field: &'static str,
    line_num: usize,
) -> Result<f64, PropagationError> {
    let v = parse_finite(val, field, line_num)?;
    if v > 0.0 {
        Ok(v)
    } else {
        Err(nsma_err(line_num, PatternParseErrorKind::BadNumber(field)))
    }
}
