//! CelesTrak legacy CSSI fixed-width (`SW-All.txt`) reader.

use alloc::vec::Vec;

use crate::space_weather::error::{SpaceWeatherError, SpaceWeatherErrorKind};
use crate::space_weather::fields::{
    clean_text, fixed_field, parse_optional_ap, parse_optional_flux, parse_ymd_tokens,
};
use crate::space_weather::record::{IndexQuality, SpaceWeatherDay, SpaceWeatherSource};
use crate::space_weather::table::SpaceWeatherTable;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LegacySection {
    Observed,
    DailyPredicted,
    MonthlyPredicted,
}

impl LegacySection {
    const fn name(self) -> &'static str {
        match self {
            Self::Observed => "OBSERVED",
            Self::DailyPredicted => "DAILY_PREDICTED",
            Self::MonthlyPredicted => "MONTHLY_PREDICTED",
        }
    }
}

/// Parses a CelesTrak legacy CSSI space-weather file (`SW-All.txt` or
/// `SW-Last5Years.txt`) into a [`SpaceWeatherTable`].
///
/// Parses both `BEGIN OBSERVED`..`END OBSERVED` and
/// `BEGIN DAILY_PREDICTED`..`END DAILY_PREDICTED` sections, skipping
/// `MONTHLY_PREDICTED` (which carries only 1st-of-month solar flux predictions
/// and no `ap` indices; see [`parse_celestrak_legacy_with_monthly`] to include
/// those rows). Handles both fixed Fortran `F6.1`/`I4` column layout
/// (including blank or truncated trailing fields) and variable whitespace,
/// while skipping `ISN`.
///
/// # Errors
///
/// [`SpaceWeatherError`] on unclosed or mismatched section markers, malformed
/// lines, invalid numbers/dates, or empty input.
pub fn parse_celestrak_legacy(text: &str) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    parse_legacy_inner(text, false)
}

/// Parses a CelesTrak legacy CSSI file, optionally including
/// `BEGIN MONTHLY_PREDICTED`..`END MONTHLY_PREDICTED` records.
///
/// Monthly-predicted records have `ap = [None; 8]`, `daily_ap = None`, and
/// [`IndexQuality::MonthlyPredicted`].
///
/// # Errors
///
/// As [`parse_celestrak_legacy`].
pub fn parse_celestrak_legacy_with_monthly(
    text: &str,
    include_monthly: bool,
) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    parse_legacy_inner(text, include_monthly)
}

fn parse_legacy_inner(
    text: &str,
    include_monthly: bool,
) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    let cleaned = clean_text(text);
    let mut section: Option<LegacySection> = None;
    let mut days = Vec::new();
    for (idx, raw_line) in cleaned.lines().enumerate() {
        let line_num = idx.wrapping_add(1);
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if handle_section_marker(trimmed, line_num, &mut section)? {
            continue;
        }
        let Some(active) = section else {
            continue;
        };
        if active == LegacySection::MonthlyPredicted && !include_monthly {
            continue;
        }
        days.push(parse_legacy_row(line, line_num, active)?);
    }
    if let Some(open) = section {
        return Err(SpaceWeatherError::at_query(
            SpaceWeatherErrorKind::UnclosedSection(open.name()),
        ));
    }
    if days.is_empty() {
        return Err(SpaceWeatherError::at_query(SpaceWeatherErrorKind::Empty));
    }
    SpaceWeatherTable::from_days(days)
}

fn handle_section_marker(
    trimmed: &str,
    line_num: usize,
    section: &mut Option<LegacySection>,
) -> Result<bool, SpaceWeatherError> {
    let begin = match trimmed {
        "BEGIN OBSERVED" => Some(LegacySection::Observed),
        "BEGIN DAILY_PREDICTED" => Some(LegacySection::DailyPredicted),
        "BEGIN MONTHLY_PREDICTED" => Some(LegacySection::MonthlyPredicted),
        _ => None,
    };
    if let Some(next_sec) = begin {
        if let Some(open) = section {
            return Err(SpaceWeatherError::at_line(
                line_num,
                SpaceWeatherErrorKind::UnclosedSection(open.name()),
            ));
        }
        *section = Some(next_sec);
        return Ok(true);
    }
    let end = match trimmed {
        "END OBSERVED" => Some(LegacySection::Observed),
        "END DAILY_PREDICTED" => Some(LegacySection::DailyPredicted),
        "END MONTHLY_PREDICTED" => Some(LegacySection::MonthlyPredicted),
        _ => None,
    };
    if let Some(closing) = end {
        match section {
            Some(open) if *open == closing => {
                *section = None;
                return Ok(true);
            }
            _ => {
                return Err(SpaceWeatherError::at_line(
                    line_num,
                    SpaceWeatherErrorKind::UnclosedSection(closing.name()),
                ));
            }
        }
    }
    Ok(false)
}

fn is_fixed_cssi(line: &str) -> bool {
    if !line.is_ascii() || line.len() < 83 {
        return false;
    }
    let bytes = line.as_bytes();
    let sep_indices = [4usize, 7, 10, 15, 18, 42, 46, 78, 82, 88, 92, 98];
    sep_indices
        .iter()
        .all(|&idx| bytes.get(idx).is_none_or(|&b| b == b' '))
}

fn parse_legacy_row(
    line: &str,
    line_num: usize,
    section: LegacySection,
) -> Result<SpaceWeatherDay, SpaceWeatherError> {
    if is_fixed_cssi(line) {
        parse_legacy_fixed(line, line_num, section)
    } else if section != LegacySection::MonthlyPredicted {
        parse_legacy_whitespace(line, line_num, section)
    } else {
        Err(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("MONTHLY_PREDICTED row requires fixed CSSI columns"),
        ))
    }
}

fn parse_legacy_fixed(
    line: &str,
    line_num: usize,
    section: LegacySection,
) -> Result<SpaceWeatherDay, SpaceWeatherError> {
    let req = |start: usize, end: usize| {
        fixed_field(line, start, end).ok_or(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("truncated CSSI fixed-width record"),
        ))
    };
    let date = parse_ymd_tokens(req(0, 4)?, req(5, 7)?, req(8, 10)?, line_num)?;
    let mut ap = [None; 8];
    let ap_ranges = [
        (46usize, 50usize),
        (50, 54),
        (54, 58),
        (58, 62),
        (62, 66),
        (66, 70),
        (70, 74),
        (74, 78),
    ];
    for (slot, (start, end)) in ap.iter_mut().zip(ap_ranges) {
        if let Some(raw) = fixed_field(line, start, end) {
            *slot = parse_optional_ap(raw, line_num, "ap", None)?;
        }
    }
    let daily_ap = match fixed_field(line, 78, 82) {
        Some(raw) => parse_optional_ap(raw, line_num, "daily_ap", None)?,
        None => None,
    };
    // Columns 88..92 (`ISN`) are intentionally skipped.
    let f107_adjusted = match fixed_field(line, 92, 98) {
        Some(raw) => parse_optional_flux(raw, line_num, "f107_adjusted", &[])?,
        None => None,
    };
    let q_str = fixed_field(line, 98, 100).unwrap_or("");
    let quality = legacy_quality(section, q_str, line_num)?;
    let f107_adjusted_center81 = match fixed_field(line, 100, 106) {
        Some(raw) => parse_optional_flux(raw, line_num, "f107_adjusted_center81", &[])?,
        None => None,
    };
    let f107_observed = match fixed_field(line, 112, 118) {
        Some(raw) => parse_optional_flux(raw, line_num, "f107_observed", &[])?,
        None => None,
    };
    let f107_observed_center81 = match fixed_field(line, 118, 124) {
        Some(raw) => parse_optional_flux(raw, line_num, "f107_observed_center81", &[])?,
        None => None,
    };
    Ok(SpaceWeatherDay {
        date,
        ap,
        daily_ap,
        f107_observed,
        f107_adjusted,
        f107_observed_center81,
        f107_adjusted_center81,
        quality,
        source: SpaceWeatherSource::CelestrakLegacy,
    })
}

fn parse_legacy_whitespace(
    line: &str,
    line_num: usize,
    section: LegacySection,
) -> Result<SpaceWeatherDay, SpaceWeatherError> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let tok = |idx: usize| {
        tokens.get(idx).copied().ok_or(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("too few whitespace tokens in CSSI record"),
        ))
    };
    let date = parse_ymd_tokens(tok(0)?, tok(1)?, tok(2)?, line_num)?;
    let mut ap = [None; 8];
    for (idx, slot) in ap.iter_mut().enumerate() {
        *slot = parse_optional_ap(tok(14usize.wrapping_add(idx))?, line_num, "ap", None)?;
    }
    let daily_ap = parse_optional_ap(tok(22)?, line_num, "daily_ap", None)?;
    // Token 25 (`ISN`) is intentionally skipped.
    let f107_adjusted = parse_optional_flux(tok(26)?, line_num, "f107_adjusted", &[])?;
    let (quality, flux_start) = match section {
        LegacySection::Observed => (legacy_quality(section, tok(27)?, line_num)?, 28usize),
        LegacySection::DailyPredicted => (IndexQuality::Predicted, 27usize),
        LegacySection::MonthlyPredicted => (IndexQuality::MonthlyPredicted, 27usize),
    };
    let opt_flux = |idx: usize, field: &'static str| -> Result<Option<f64>, SpaceWeatherError> {
        match tokens.get(idx).copied() {
            Some(raw) => parse_optional_flux(raw, line_num, field, &[]),
            None => Ok(None),
        }
    };
    let f107_adjusted_center81 = opt_flux(flux_start, "f107_adjusted_center81")?;
    let f107_observed = opt_flux(flux_start.wrapping_add(2), "f107_observed")?;
    let f107_observed_center81 = opt_flux(flux_start.wrapping_add(3), "f107_observed_center81")?;
    if tokens.len() > flux_start.wrapping_add(5) {
        return Err(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("unexpected trailing tokens in CSSI record"),
        ));
    }
    Ok(SpaceWeatherDay {
        date,
        ap,
        daily_ap,
        f107_observed,
        f107_adjusted,
        f107_observed_center81,
        f107_adjusted_center81,
        quality,
        source: SpaceWeatherSource::CelestrakLegacy,
    })
}

fn legacy_quality(
    section: LegacySection,
    q: &str,
    line_num: usize,
) -> Result<IndexQuality, SpaceWeatherError> {
    match section {
        LegacySection::MonthlyPredicted => Ok(IndexQuality::MonthlyPredicted),
        LegacySection::DailyPredicted => Ok(IndexQuality::Predicted),
        LegacySection::Observed => match q {
            "0" | "1" => Ok(IndexQuality::Definitive),
            "2" | "4" => Ok(IndexQuality::Interpolated),
            "3" | "" => Ok(IndexQuality::Provisional),
            _ => Err(SpaceWeatherError::at_line(
                line_num,
                SpaceWeatherErrorKind::InvalidQuality,
            )),
        },
    }
}
