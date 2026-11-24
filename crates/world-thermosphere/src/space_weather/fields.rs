//! Shared scalar and date field parsers for space-weather text formats.

use super::calendar::CalendarDate;
use super::error::{SpaceWeatherError, SpaceWeatherErrorKind};

/// Strips an optional UTF-8 byte-order mark and trailing `\r`.
pub(crate) fn clean_text(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}

/// Extracts an ASCII column range `[start..end)` safely, returning a trimmed
/// slice when the line starts inside `start..` even if trailing columns are
/// truncated.
pub(crate) fn fixed_field(line: &str, start: usize, end: usize) -> Option<&str> {
    if !line.is_ascii() || start >= line.len() {
        return None;
    }
    let clamped = end.min(line.len());
    line.get(start..clamped).map(str::trim)
}

/// Parses an integer token tagged with `line` and `field`.
pub(crate) fn parse_i32(
    token: &str,
    line: usize,
    field: &'static str,
) -> Result<i32, SpaceWeatherError> {
    token
        .trim()
        .parse::<i32>()
        .map_err(|_| SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber(field)))
}

/// Parses an unsigned byte token tagged with `line` and `field`.
pub(crate) fn parse_u8(
    token: &str,
    line: usize,
    field: &'static str,
) -> Result<u8, SpaceWeatherError> {
    token
        .trim()
        .parse::<u8>()
        .map_err(|_| SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber(field)))
}

/// Parses a finite floating-point token tagged with `line` and `field`.
pub(crate) fn parse_f64(
    token: &str,
    line: usize,
    field: &'static str,
) -> Result<f64, SpaceWeatherError> {
    let val = token
        .trim()
        .parse::<f64>()
        .map_err(|_| SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber(field)))?;
    if !val.is_finite() {
        return Err(SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::InvalidValue { field, value: val },
        ));
    }
    Ok(val)
}

/// Parses `(year, month, day)` tokens and attaches `line` to any date error.
pub(crate) fn parse_ymd_tokens(
    y: &str,
    m: &str,
    d: &str,
    line: usize,
) -> Result<CalendarDate, SpaceWeatherError> {
    let year = parse_i32(y, line, "year")?;
    let month = parse_u8(m, line, "month")?;
    let day = parse_u8(d, line, "day")?;
    CalendarDate::new(year, month, day).map_err(|mut err| {
        err.line = line;
        err
    })
}

/// Parses an ISO `YYYY-MM-DD` date string.
pub(crate) fn parse_iso_date(token: &str, line: usize) -> Result<CalendarDate, SpaceWeatherError> {
    let trimmed = token.trim();
    let mut parts = trimmed.split('-');
    let y = parts.next().ok_or_else(|| {
        SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::Malformed("expected YYYY-MM-DD"),
        )
    })?;
    let m = parts.next().ok_or_else(|| {
        SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::Malformed("expected YYYY-MM-DD"),
        )
    })?;
    let d = parts.next().ok_or_else(|| {
        SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::Malformed("expected YYYY-MM-DD"),
        )
    })?;
    if parts.next().is_some() {
        return Err(SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::Malformed("expected YYYY-MM-DD"),
        ));
    }
    parse_ymd_tokens(y, m, d, line)
}

/// Parses an optional non-negative `ap`/`Ap` index (`blank` or `missing_sentinel` -> `None`).
pub(crate) fn parse_optional_ap(
    token: &str,
    line: usize,
    field: &'static str,
    missing_sentinel: Option<f64>,
) -> Result<Option<f64>, SpaceWeatherError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let val = parse_f64(trimmed, line, field)?;
    if missing_sentinel.is_some_and(|s| val == s) {
        return Ok(None);
    }
    if val < 0.0 {
        return Err(SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::InvalidValue { field, value: val },
        ));
    }
    Ok(Some(val))
}

/// Parses an optional positive F10.7 flux (`blank` or sentinel -> `None`).
pub(crate) fn parse_optional_flux(
    token: &str,
    line: usize,
    field: &'static str,
    sentinels: &[f64],
) -> Result<Option<f64>, SpaceWeatherError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let val = parse_f64(trimmed, line, field)?;
    if sentinels.contains(&val) {
        return Ok(None);
    }
    if val <= 0.0 {
        return Err(SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::InvalidValue { field, value: val },
        ));
    }
    Ok(Some(val))
}

/// Parses an ISO `YYYY-MM-DD[THH:MM:SS[Z]]` timestamp into date and whole UTC
/// seconds of day.
pub(crate) fn parse_time_tag(
    raw: &str,
    line: usize,
) -> Result<(CalendarDate, u32), SpaceWeatherError> {
    let trimmed = raw.trim();
    let (date_str, rest) = if let Some((d, t)) = trimmed.split_once(['T', 't', ' ']) {
        (d, Some(t))
    } else {
        (trimmed, None)
    };
    let date = parse_iso_date(date_str, line)?;
    let secs = match rest {
        None => 0,
        Some(time_part) => parse_hms_seconds(time_part, line)?,
    };
    Ok((date, secs))
}

fn parse_hms_seconds(raw: &str, line: usize) -> Result<u32, SpaceWeatherError> {
    let core = raw.strip_suffix('Z').unwrap_or(raw);
    let mut parts = core.split(':');
    let h_str = parts.next().ok_or_else(|| {
        SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::Malformed("time_tag"))
    })?;
    let m_str = parts.next().ok_or_else(|| {
        SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::Malformed("time_tag"))
    })?;
    let s_str = parts.next().ok_or_else(|| {
        SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::Malformed("time_tag"))
    })?;
    let s_whole = s_str.split('.').next().unwrap_or(s_str);
    let h: u32 = h_str
        .parse()
        .map_err(|_| SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber("hour")))?;
    let m: u32 = m_str.parse().map_err(|_| {
        SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber("minute"))
    })?;
    let s: u32 = s_whole.parse().map_err(|_| {
        SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber("second"))
    })?;
    if h >= 24 || m >= 60 || s >= 60 {
        return Err(SpaceWeatherError::at_line(
            line,
            SpaceWeatherErrorKind::Malformed("time_tag clock out of range"),
        ));
    }
    Ok(h * 3_600 + m * 60 + s)
}
