//! Why a loss or a gain was not evaluated.

use core::fmt;

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
