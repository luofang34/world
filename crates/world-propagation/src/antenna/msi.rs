//! MSI Planet (`.msi` / `.pln`) antenna pattern file reader (`no_std` + `alloc`).

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{PatternParseErrorKind, PropagationError};

use super::cut::{
    AntennaMetadata, CutPlane, DBD_TO_DBI, PatternCut, PatternPolarisation, PatternSample,
    normalize_azimuth_cut,
};
use super::tabulated::TabulatedAntennaPattern;

#[derive(Clone, Copy)]
enum Section {
    Header,
    Horizontal { expected: usize, read: usize },
    Vertical { expected: usize, read: usize },
}

struct MsiState {
    meta: AntennaMetadata,
    gain_dbi: Option<f64>,
    horiz: Vec<PatternSample>,
    vert_raw: Vec<PatternSample>,
    has_horiz: bool,
    has_vert: bool,
    section: Section,
}

impl MsiState {
    fn new() -> Self {
        Self {
            meta: AntennaMetadata::default(),
            gain_dbi: None,
            horiz: Vec::new(),
            vert_raw: Vec::new(),
            has_horiz: false,
            has_vert: false,
            section: Section::Header,
        }
    }
}

fn err(line: usize, kind: PatternParseErrorKind) -> PropagationError {
    PropagationError::InvalidPatternFormat { line, kind }
}

/// Parses an MSI Planet ASCII antenna pattern into a [`TabulatedAntennaPattern`].
pub(crate) fn parse_msi(text: &str) -> Result<TabulatedAntennaPattern, PropagationError> {
    let cleaned = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut state = MsiState::new();
    for (idx, raw) in cleaned.lines().enumerate() {
        let line_num = idx.wrapping_add(1);
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        process_msi_line(&mut state, line, line_num)?;
    }
    finish_msi(state)
}

fn process_msi_line(
    state: &mut MsiState,
    line: &str,
    line_num: usize,
) -> Result<(), PropagationError> {
    if let Some((key_upper, value)) = split_keyword_line(line) {
        if key_upper == "HORIZONTAL" || key_upper == "VERTICAL" {
            return start_msi_section(state, &key_upper, value, line_num);
        }
        if matches!(state.section, Section::Header) {
            return parse_header_key(
                &mut state.meta,
                &mut state.gain_dbi,
                &key_upper,
                value,
                line_num,
            );
        }
    }

    match state.section {
        Section::Header => Err(err(
            line_num,
            PatternParseErrorKind::MalformedRecord("expected MSI header keyword and value"),
        )),
        Section::Horizontal {
            expected,
            ref mut read,
        } => push_msi_sample(&mut state.horiz, line, expected, read, line_num),
        Section::Vertical {
            expected,
            ref mut read,
        } => push_msi_sample(&mut state.vert_raw, line, expected, read, line_num),
    }
}

fn start_msi_section(
    state: &mut MsiState,
    key_upper: &str,
    value: &str,
    line_num: usize,
) -> Result<(), PropagationError> {
    close_active_section(state, line_num)?;
    let expected: usize = value
        .trim()
        .parse()
        .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("count")))?;
    if expected < 2 {
        return Err(err(
            line_num,
            PatternParseErrorKind::MalformedRecord("cut point count must be at least 2"),
        ));
    }
    if key_upper == "HORIZONTAL" {
        if state.has_horiz {
            return Err(err(
                line_num,
                PatternParseErrorKind::MalformedRecord("duplicate HORIZONTAL section"),
            ));
        }
        state.has_horiz = true;
        state.horiz.reserve(expected);
        state.section = Section::Horizontal { expected, read: 0 };
    } else {
        if state.has_vert {
            return Err(err(
                line_num,
                PatternParseErrorKind::MalformedRecord("duplicate VERTICAL section"),
            ));
        }
        state.has_vert = true;
        state.vert_raw.reserve(expected);
        state.section = Section::Vertical { expected, read: 0 };
    }
    Ok(())
}

fn push_msi_sample(
    target: &mut Vec<PatternSample>,
    line: &str,
    expected: usize,
    read: &mut usize,
    line_num: usize,
) -> Result<(), PropagationError> {
    let sample = parse_msi_sample(line, target.last().map(|s| s.angle_deg), *read, line_num)?;
    *read = read.wrapping_add(1);
    if *read > expected {
        return Err(err(
            line_num,
            PatternParseErrorKind::InvalidCount {
                expected,
                found: *read,
            },
        ));
    }
    target.push(sample);
    Ok(())
}

fn close_active_section(state: &MsiState, line_num: usize) -> Result<(), PropagationError> {
    match state.section {
        Section::Header => Ok(()),
        Section::Horizontal { expected, read } | Section::Vertical { expected, read } => {
            if read != expected {
                Err(err(
                    line_num,
                    PatternParseErrorKind::InvalidCount {
                        expected,
                        found: read,
                    },
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn finish_msi(state: MsiState) -> Result<TabulatedAntennaPattern, PropagationError> {
    close_active_section(&state, 0)?;
    if state.meta.name.is_empty() && state.gain_dbi.is_none() && !state.has_horiz && !state.has_vert
    {
        return Err(err(0, PatternParseErrorKind::Empty));
    }
    let gain_dbi = state
        .gain_dbi
        .ok_or_else(|| err(0, PatternParseErrorKind::MissingHeader("GAIN")))?;
    if !state.has_horiz {
        return Err(err(0, PatternParseErrorKind::MissingHeader("HORIZONTAL")));
    }
    if !state.has_vert {
        return Err(err(0, PatternParseErrorKind::MissingHeader("VERTICAL")));
    }
    let azimuth_cut = normalize_azimuth_cut(&state.horiz);
    let elevation_cut = msi_vertical_to_elevation(&state.vert_raw);
    let pol = state
        .meta
        .polarisation
        .unwrap_or(PatternPolarisation::Vertical);
    let cuts = alloc::vec![
        PatternCut::new(CutPlane::Azimuth, pol, azimuth_cut.clone())?,
        PatternCut::new(CutPlane::Elevation, pol, elevation_cut.clone())?,
    ];
    TabulatedAntennaPattern::from_parts(state.meta, gain_dbi, azimuth_cut, elevation_cut, cuts)
}

/// Maps MSI `VERTICAL 360` angles (`0°` = horizon, `1..90°` = downtilt, `270..359°`
/// = uptilt) into monotonically sorted `[-90, +90]` elevation cuts.
fn msi_vertical_to_elevation(vert_raw: &[PatternSample]) -> Vec<PatternSample> {
    let mut elev: Vec<PatternSample> = vert_raw
        .iter()
        .filter_map(|s| {
            let a = s.angle_deg;
            let el = if (0.0..=90.0).contains(&a) {
                Some(-a)
            } else if (270.0..360.0).contains(&a) {
                Some(360.0 - a)
            } else {
                None
            };
            el.map(|angle_deg| PatternSample {
                angle_deg,
                attenuation_db: s.attenuation_db,
            })
        })
        .collect();
    elev.sort_by(|a, b| a.angle_deg.total_cmp(&b.angle_deg));
    elev
}

fn split_keyword_line(line: &str) -> Option<(String, &str)> {
    let mut parts = line.splitn(2, char::is_whitespace);
    let key = parts.next()?.trim();
    let val = parts.next().unwrap_or("").trim();
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    // Data rows start with a number (e.g. `0 0.00`).
    if key
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_digit() || b == b'-' || b == b'+')
    {
        return None;
    }
    Some((key.to_ascii_uppercase(), val))
}

fn parse_header_key(
    meta: &mut AntennaMetadata,
    gain_dbi: &mut Option<f64>,
    key: &str,
    value: &str,
    line_num: usize,
) -> Result<(), PropagationError> {
    match key {
        "NAME" => {
            if value.is_empty() {
                return Err(err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("empty NAME"),
                ));
            }
            meta.name = value.to_string();
        }
        "MAKE" => meta.make = value.to_string(),
        "FREQUENCY" => meta.frequency_mhz = Some(parse_freq_mhz(value, line_num)?),
        "H_WIDTH" => meta.h_width_deg = Some(parse_non_neg(value, "H_WIDTH", line_num)?),
        "V_WIDTH" => meta.v_width_deg = Some(parse_non_neg(value, "V_WIDTH", line_num)?),
        "FRONT_TO_BACK" => {
            meta.front_to_back_db = Some(parse_non_neg(value, "FRONT_TO_BACK", line_num)?);
        }
        "GAIN" => *gain_dbi = Some(parse_msi_gain(value, line_num)?),
        "TILT" => meta.tilt_deg = parse_msi_tilt(value, line_num)?,
        "POLARIZATION" | "POLARISATION" => {
            let pol = PatternPolarisation::parse(value).ok_or_else(|| {
                err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("unrecognized POLARIZATION"),
                )
            })?;
            meta.polarisation = Some(pol);
        }
        "COMMENT" => {
            if !value.is_empty() {
                meta.comment = Some(value.to_string());
            }
        }
        _ => {
            return Err(err(
                line_num,
                PatternParseErrorKind::MalformedRecord("unrecognized MSI header keyword"),
            ));
        }
    }
    Ok(())
}

fn parse_msi_gain(value: &str, line_num: usize) -> Result<f64, PropagationError> {
    let mut toks = value.split_whitespace();
    let num_tok = toks
        .next()
        .ok_or_else(|| err(line_num, PatternParseErrorKind::BadNumber("GAIN")))?;
    let val: f64 = num_tok
        .parse()
        .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("GAIN")))?;
    if !val.is_finite() {
        return Err(err(line_num, PatternParseErrorKind::BadNumber("GAIN")));
    }
    let unit = toks.next();
    if toks.next().is_some() {
        return Err(err(
            line_num,
            PatternParseErrorKind::MalformedRecord("unexpected tokens in GAIN"),
        ));
    }
    match unit.map(str::to_ascii_uppercase).as_deref() {
        Some("DBI") => Ok(val),
        // Planet MSI defaults bare GAIN values to dBd.
        Some("DBD") | None => Ok(val + DBD_TO_DBI),
        Some(_) => Err(err(line_num, PatternParseErrorKind::UnsupportedUnits)),
    }
}

fn parse_freq_mhz(value: &str, line_num: usize) -> Result<f64, PropagationError> {
    let mut toks = value.split_whitespace();
    let num_tok = toks
        .next()
        .ok_or_else(|| err(line_num, PatternParseErrorKind::BadNumber("FREQUENCY")))?;
    let val: f64 = num_tok
        .parse()
        .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("FREQUENCY")))?;
    if !val.is_finite() || val <= 0.0 {
        return Err(err(line_num, PatternParseErrorKind::BadNumber("FREQUENCY")));
    }
    let scale = match toks.next().map(str::to_ascii_uppercase).as_deref() {
        None | Some("MHZ") => 1.0,
        Some("GHZ") => 1_000.0,
        Some("KHZ") => 1.0e-3,
        Some(_) => return Err(err(line_num, PatternParseErrorKind::UnsupportedUnits)),
    };
    if toks.next().is_some() {
        return Err(err(
            line_num,
            PatternParseErrorKind::MalformedRecord("unexpected tokens in FREQUENCY"),
        ));
    }
    Ok(val * scale)
}

fn parse_non_neg(
    value: &str,
    field: &'static str,
    line_num: usize,
) -> Result<f64, PropagationError> {
    let mut toks = value.split_whitespace();
    let tok = toks
        .next()
        .ok_or_else(|| err(line_num, PatternParseErrorKind::BadNumber(field)))?;
    let v: f64 = tok
        .parse()
        .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber(field)))?;
    if !v.is_finite() || v < 0.0 {
        return Err(err(line_num, PatternParseErrorKind::BadNumber(field)));
    }
    Ok(v)
}

fn parse_msi_tilt(value: &str, line_num: usize) -> Result<Option<f64>, PropagationError> {
    if value.is_empty() {
        return Err(err(
            line_num,
            PatternParseErrorKind::MalformedRecord("empty TILT"),
        ));
    }
    if let Ok(v) = value.trim().parse::<f64>() {
        if !v.is_finite() {
            return Err(err(line_num, PatternParseErrorKind::BadNumber("TILT")));
        }
        return Ok(Some(v));
    }
    let upper = value.trim().to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "ELECTRICAL" | "MECHANICAL" | "FIXED" | "NONE"
    ) {
        Ok(None)
    } else {
        Err(err(line_num, PatternParseErrorKind::BadNumber("TILT")))
    }
}

fn parse_msi_sample(
    line: &str,
    prev_angle: Option<f64>,
    sample_idx: usize,
    line_num: usize,
) -> Result<PatternSample, PropagationError> {
    let mut toks = line.split_whitespace();
    let angle_tok = toks.next().ok_or_else(|| {
        err(
            line_num,
            PatternParseErrorKind::MalformedRecord("missing angle"),
        )
    })?;
    let att_tok = toks.next().ok_or_else(|| {
        err(
            line_num,
            PatternParseErrorKind::MalformedRecord("missing attenuation"),
        )
    })?;
    if toks.next().is_some() {
        return Err(err(
            line_num,
            PatternParseErrorKind::MalformedRecord("trailing tokens in pattern sample"),
        ));
    }
    let angle_deg: f64 = angle_tok
        .parse()
        .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("angle")))?;
    let attenuation_db: f64 = att_tok
        .parse()
        .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("attenuation")))?;
    if !angle_deg.is_finite() || !(0.0..360.0).contains(&angle_deg) {
        return Err(err(
            line_num,
            PatternParseErrorKind::InvalidAngle { angle_deg },
        ));
    }
    if !attenuation_db.is_finite() || attenuation_db < 0.0 {
        return Err(err(
            line_num,
            PatternParseErrorKind::NonFiniteValue { index: sample_idx },
        ));
    }
    if prev_angle.is_some_and(|p| angle_deg <= p) {
        return Err(err(
            line_num,
            PatternParseErrorKind::NonMonotonicAngle { index: sample_idx },
        ));
    }
    Ok(PatternSample {
        angle_deg,
        attenuation_db,
    })
}
