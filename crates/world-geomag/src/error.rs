//! Why an evaluation was refused.

use core::fmt;

use world_core::{DecimalYear, ValidityWindow};

/// Why the model refused to evaluate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeomagError {
    /// The date is outside the model's validity window.
    OutsideValidity {
        /// The requested date.
        at: DecimalYear,
        /// The dates the model is valid for.
        validity: ValidityWindow,
    },
    /// The position is not stated in WGS-84, the frame the model is
    /// defined in.
    UnsupportedDatum,
    /// Latitude outside [-90, 90], height outside the model's domain, or a
    /// non-finite coordinate.
    InvalidPosition,
}

impl fmt::Display for GeomagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideValidity { at, validity } => write!(
                f,
                "decimal year {} is outside the model validity {}..={}",
                at.value(),
                validity.from.value(),
                validity.until.value()
            ),
            Self::UnsupportedDatum => f.write_str("the position is not stated in WGS-84"),
            Self::InvalidPosition => f.write_str("the position is outside the model domain"),
        }
    }
}

impl core::error::Error for GeomagError {}
