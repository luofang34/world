//! Why an evaluation or a model construction was refused.

use core::fmt;

use world_core::{DecimalYear, ValidityWindow};

/// Why a field was not evaluated or a model not built.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldError {
    /// The scratch buffer is shorter than [`crate::workspace_len`] of the
    /// requested degree.
    WorkspaceTooSmall {
        /// Buffer length the degree needs, in `f64`s.
        required: usize,
        /// Buffer length supplied.
        provided: usize,
    },
    /// The truncation degree is above what the workspace was built for.
    DegreeExceedsWorkspace {
        /// The requested degree.
        degree: usize,
        /// The workspace's maximum degree.
        capacity: usize,
    },
    /// The truncation degree is above the model's coefficients.
    DegreeExceedsModel {
        /// The requested degree.
        degree: usize,
        /// The coefficients' maximum degree.
        available: usize,
    },
    /// The truncation order is above its degree, or its minimum degree is.
    InvalidTruncation,
    /// Coefficient storage does not hold one triangle of the stated degree.
    CoefficientStorage {
        /// The stated maximum degree.
        degree: usize,
        /// Entries one triangle of that degree holds.
        expected: usize,
        /// Entries supplied for the cosine terms.
        cosine: usize,
        /// Entries supplied for the sine terms.
        sine: usize,
    },
    /// A non-finite coordinate, the body's centre, or a point so deep
    /// inside the reference sphere that the series overflows.
    InvalidPosition,
    /// A non-finite or non-positive reference radius or GM, or a
    /// malformed grid.
    InvalidModel,
    /// The date is outside the model's validity window.
    OutsideValidity {
        /// The requested date.
        at: DecimalYear,
        /// The dates the model is valid for.
        validity: ValidityWindow,
    },
    /// The position is outside an overlay that refuses to extrapolate.
    OutsideCoverage,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceTooSmall { required, provided } => write!(
                f,
                "the workspace holds {provided} values but the degree needs {required}"
            ),
            Self::DegreeExceedsWorkspace { degree, capacity } => write!(
                f,
                "degree {degree} exceeds the workspace's maximum degree {capacity}"
            ),
            Self::DegreeExceedsModel { degree, available } => write!(
                f,
                "degree {degree} exceeds the model's maximum degree {available}"
            ),
            Self::InvalidTruncation => {
                f.write_str("the truncation order or minimum degree exceeds its degree")
            }
            Self::CoefficientStorage {
                degree,
                expected,
                cosine,
                sine,
            } => write!(
                f,
                "degree {degree} needs {expected} coefficients per kind; got {cosine} cosine and {sine} sine"
            ),
            Self::InvalidPosition => f.write_str("the position is non-finite or at the centre"),
            Self::InvalidModel => f.write_str("the model constants or grid are malformed"),
            Self::OutsideValidity { at, validity } => write!(
                f,
                "decimal year {} is outside the model validity {}..={}",
                at.value(),
                validity.from.value(),
                validity.until.value()
            ),
            Self::OutsideCoverage => f.write_str("the position is outside the overlay's grid"),
        }
    }
}

impl core::error::Error for FieldError {}
