//! Errors produced while parsing space-weather files or querying indices.

use core::fmt;

use crate::ThermosphereError;

use super::CalendarDate;

/// What went wrong while reading a space-weather dataset or resolving indices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpaceWeatherErrorKind {
    /// The input contained no data records.
    Empty,
    /// A required header line, section marker, or column is missing.
    MissingHeader(&'static str),
    /// A section was opened without a matching `END` marker or mismatched.
    UnclosedSection(&'static str),
    /// A line or JSON structure did not match the expected format.
    Malformed(&'static str),
    /// A numeric field could not be parsed.
    BadNumber(&'static str),
    /// A calendar date is invalid or outside the supported year range.
    InvalidDate {
        /// Calendar year.
        year: i32,
        /// Month, 1 through 12.
        month: u8,
        /// Day of the month.
        day: u8,
    },
    /// The UTC seconds value is outside `[0.0, 86_400.0)` or non-finite.
    InvalidTime {
        /// Seconds of the UTC day.
        ut_seconds: f64,
    },
    /// A parsed or provided index value is out of range or non-finite.
    InvalidValue {
        /// Name of the field or index.
        field: &'static str,
        /// Offending value.
        value: f64,
    },
    /// A quality or data-type flag is unrecognized.
    InvalidQuality,
    /// A required date is absent from the lookup table.
    MissingDate {
        /// Missing calendar date.
        date: CalendarDate,
    },
    /// A date exists in the table, but the required index is missing.
    MissingIndex {
        /// Date whose record lacks the index.
        date: CalendarDate,
        /// Name of the missing index field.
        index: &'static str,
    },
    /// The synthesized inputs failed [`crate::MsisInput`] validation.
    Thermosphere(ThermosphereError),
}

/// Why parsing a space-weather file or synthesizing activity indices failed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceWeatherError {
    /// 1-based line number of the offending input line, or `0` for table-level
    /// and query-time errors.
    pub line: usize,
    /// Specific failure reason.
    pub kind: SpaceWeatherErrorKind,
}

impl SpaceWeatherError {
    /// Constructs a table-level or query-time error (`line = 0`).
    pub(crate) const fn at_query(kind: SpaceWeatherErrorKind) -> Self {
        Self { line: 0, kind }
    }

    /// Constructs a line-tagged error (`1`-based line number).
    pub(crate) const fn at_line(line: usize, kind: SpaceWeatherErrorKind) -> Self {
        Self { line, kind }
    }
}

impl fmt::Display for SpaceWeatherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: ", self.line)?;
        }
        match self.kind {
            SpaceWeatherErrorKind::Empty => f.write_str("no space-weather records found"),
            SpaceWeatherErrorKind::MissingHeader(name) => {
                write!(f, "missing required header or column `{name}`")
            }
            SpaceWeatherErrorKind::UnclosedSection(section) => {
                write!(f, "unclosed or mismatched section `{section}`")
            }
            SpaceWeatherErrorKind::Malformed(detail) => write!(f, "malformed input: {detail}"),
            SpaceWeatherErrorKind::BadNumber(field) => write!(f, "invalid number in `{field}`"),
            SpaceWeatherErrorKind::InvalidDate { year, month, day } => {
                write!(f, "invalid calendar date {year:04}-{month:02}-{day:02}")
            }
            SpaceWeatherErrorKind::InvalidTime { ut_seconds } => {
                write!(f, "ut_seconds = {ut_seconds} is outside [0, 86400)")
            }
            SpaceWeatherErrorKind::InvalidValue { field, value } => {
                write!(f, "invalid `{field}` value {value}")
            }
            SpaceWeatherErrorKind::InvalidQuality => f.write_str("unrecognized index quality flag"),
            SpaceWeatherErrorKind::MissingDate { date } => {
                write!(f, "no space-weather record for {date}")
            }
            SpaceWeatherErrorKind::MissingIndex { date, index } => {
                write!(f, "missing `{index}` on {date}")
            }
            SpaceWeatherErrorKind::Thermosphere(err) => write!(f, "{err}"),
        }
    }
}

impl core::error::Error for SpaceWeatherError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match &self.kind {
            SpaceWeatherErrorKind::Thermosphere(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ThermosphereError> for SpaceWeatherError {
    fn from(err: ThermosphereError) -> Self {
        Self::at_query(SpaceWeatherErrorKind::Thermosphere(err))
    }
}
