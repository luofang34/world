//! Why a loss or a gain was not evaluated.

use core::fmt;

/// What went wrong while parsing a tabulated antenna pattern file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternParseErrorKind {
    /// The input contained no header or pattern records.
    Empty,
    /// A required header record or pattern section is missing.
    MissingHeader(&'static str),
    /// A header or section line did not match the format grammar.
    MalformedRecord(&'static str),
    /// A numeric header or table field could not be parsed as a number.
    BadNumber(&'static str),
    /// The declared point count in a cut did not match the parsed rows.
    InvalidCount {
        /// Point count declared in the section header.
        expected: usize,
        /// Actual number of sample rows parsed.
        found: usize,
    },
    /// An angle sample fell outside the valid domain for the cut.
    InvalidAngle {
        /// The offending angle, degrees.
        angle_deg: f64,
    },
    /// A sample angle was not strictly greater than the previous sample.
    NonMonotonicAngle {
        /// Zero-based index of the non-monotonic sample inside the cut.
        index: usize,
    },
    /// A numeric pattern value was non-finite or physically invalid.
    NonFiniteValue {
        /// Zero-based index of the offending sample inside the cut.
        index: usize,
    },
    /// The file declared an unsupported gain or pattern unit.
    UnsupportedUnits,
    /// The file ended without its required end-of-file marker.
    MissingEndMarker,
}

impl fmt::Display for PatternParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty antenna pattern input"),
            Self::MissingHeader(name) => write!(f, "missing required header or cut `{name}`"),
            Self::MalformedRecord(detail) => write!(f, "malformed record: {detail}"),
            Self::BadNumber(field) => write!(f, "invalid numeric value in `{field}`"),
            Self::InvalidCount { expected, found } => {
                write!(f, "expected {expected} pattern samples, found {found}")
            }
            Self::InvalidAngle { angle_deg } => {
                write!(f, "angle {angle_deg} deg is outside the valid cut domain")
            }
            Self::NonMonotonicAngle { index } => {
                write!(
                    f,
                    "sample angle at index {index} is not strictly increasing"
                )
            }
            Self::NonFiniteValue { index } => {
                write!(
                    f,
                    "pattern sample at index {index} is non-finite or invalid"
                )
            }
            Self::UnsupportedUnits => f.write_str("unsupported antenna gain or pattern units"),
            Self::MissingEndMarker => f.write_str("missing required end-of-file marker"),
        }
    }
}

/// Why a propagation model or an antenna pattern refused its input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropagationError {
    /// A parameter is non-finite or outside the values it can physically
    /// take (a negative distance, a non-positive Earth radius).
    InvalidParameter {
        /// The parameter's name.
        name: &'static str,
        /// The value supplied.
        value: f64,
    },
    /// The frequency is outside the range the method is defined for.
    FrequencyOutOfRange {
        /// The frequency supplied, Hz.
        frequency_hz: f64,
        /// Lowest frequency the method covers, Hz.
        min_hz: f64,
        /// Highest frequency the method covers, Hz.
        max_hz: f64,
    },
    /// The terrain profile has fewer samples than the method needs.
    ProfileTooShort {
        /// Samples supplied.
        samples: usize,
        /// Samples the method needs.
        required: usize,
    },
    /// A profile sample is non-finite or not strictly further along the
    /// path than the one before it.
    InvalidProfileSample {
        /// Index of the offending sample.
        index: usize,
    },
    /// The off-axis angle is outside the range the pattern defines; a
    /// reference pattern that leaves the main beam unspecified refuses
    /// angles inside it rather than inventing one.
    AngleOutsidePattern {
        /// The off-axis angle supplied, degrees.
        off_axis_deg: f64,
        /// Smallest angle the pattern defines, degrees.
        min_deg: f64,
        /// Largest angle the pattern defines, degrees.
        max_deg: f64,
    },
    /// A tabulated antenna pattern file failed syntax or physical validation.
    InvalidPatternFormat {
        /// 1-based line number of the offending input, or 0 for whole-file errors.
        line: usize,
        /// Specific pattern parse failure.
        kind: PatternParseErrorKind,
    },
    /// A caller-supplied atmospheric grid provider failed SHA-256 integrity
    /// verification or lacked an expected content digest.
    GridIntegrityMismatch,
    /// A caller-supplied atmospheric grid provider had no data covering the
    /// requested ground station coordinates.
    GridDataUnavailable {
        /// Station geodetic latitude, degrees.
        latitude_deg: f64,
        /// Station geodetic longitude, degrees.
        longitude_deg: f64,
    },
}

impl fmt::Display for PropagationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, value } => {
                write!(f, "parameter {name} = {value} is not a valid value")
            }
            Self::FrequencyOutOfRange {
                frequency_hz,
                min_hz,
                max_hz,
            } => write!(
                f,
                "frequency {frequency_hz} Hz is outside the method's range {min_hz}..={max_hz} Hz"
            ),
            Self::ProfileTooShort { samples, required } => write!(
                f,
                "the terrain profile has {samples} samples but the method needs {required}"
            ),
            Self::InvalidProfileSample { index } => write!(
                f,
                "profile sample {index} is non-finite or not beyond the previous sample"
            ),
            Self::AngleOutsidePattern {
                off_axis_deg,
                min_deg,
                max_deg,
            } => write!(
                f,
                "off-axis angle {off_axis_deg} deg is outside the pattern's range {min_deg}..={max_deg} deg"
            ),
            Self::InvalidPatternFormat { line, kind } => {
                if *line > 0 {
                    write!(f, "antenna pattern line {line}: {kind}")
                } else {
                    write!(f, "antenna pattern: {kind}")
                }
            }
            Self::GridIntegrityMismatch => {
                f.write_str("atmospheric grid provider failed SHA-256 integrity verification")
            }
            Self::GridDataUnavailable {
                latitude_deg,
                longitude_deg,
            } => write!(
                f,
                "no atmospheric grid data covers ({latitude_deg} deg, {longitude_deg} deg)"
            ),
        }
    }
}

impl core::error::Error for PropagationError {}

/// `value` if it is finite, otherwise [`PropagationError::InvalidParameter`].
pub(crate) fn finite(name: &'static str, value: f64) -> Result<f64, PropagationError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(PropagationError::InvalidParameter { name, value })
    }
}

/// `value` if it is finite and strictly positive.
pub(crate) fn positive(name: &'static str, value: f64) -> Result<f64, PropagationError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(PropagationError::InvalidParameter { name, value })
    }
}

/// `value` if it is finite and not negative.
pub(crate) fn non_negative(name: &'static str, value: f64) -> Result<f64, PropagationError> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(PropagationError::InvalidParameter { name, value })
    }
}
