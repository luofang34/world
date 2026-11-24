//! One instant on the two clocks a moving world is driven by.
//!
//! Perishable state ages on a monotonic clock, which never steps, and is
//! valid on civil UTC, which can. A [`WorldInstant`] names one instant on
//! both, so a consumer never derives one clock from the other on its own. A
//! [`ClockCorrespondence`] is the host's statement of how the two relate; it
//! places a planned UTC time on the monotonic clock and back.

use core::fmt;

use crate::DecimalYear;

/// One instant, on the host's monotonic clock and on civil UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldInstant {
    /// Host monotonic clock, microseconds from an origin the host chooses.
    pub monotonic_micros: u64,
    /// Civil UTC, microseconds since the Unix epoch.
    pub utc_unix_micros: i64,
}

impl WorldInstant {
    /// The instant at `monotonic_micros` and `utc_unix_micros`.
    pub const fn new(monotonic_micros: u64, utc_unix_micros: i64) -> Self {
        Self {
            monotonic_micros,
            utc_unix_micros,
        }
    }

    /// The instant's UTC as the decimal year geophysical models take.
    pub fn decimal_year(self) -> DecimalYear {
        DecimalYear::from_unix_micros(self.utc_unix_micros)
    }
}

/// The host's statement that one monotonic reading and one UTC reading are
/// the same instant.
///
/// The mapping is exact only while UTC does not step; a host that corrects
/// its UTC publishes a new correspondence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockCorrespondence {
    /// Monotonic clock value at the correspondence instant, microseconds.
    pub monotonic_micros: u64,
    /// Microseconds since the Unix epoch (UTC) at the same instant.
    pub utc_unix_micros: i64,
}

impl ClockCorrespondence {
    /// The correspondence pairing `monotonic_micros` with `utc_unix_micros`.
    pub const fn new(monotonic_micros: u64, utc_unix_micros: i64) -> Self {
        Self {
            monotonic_micros,
            utc_unix_micros,
        }
    }

    /// The instant at a monotonic reading.
    ///
    /// # Errors
    ///
    /// [`ClockError::UtcOutOfRange`] when the UTC value does not fit.
    pub fn at_monotonic(self, monotonic_micros: u64) -> Result<WorldInstant, ClockError> {
        let offset = i128::from(monotonic_micros) - i128::from(self.monotonic_micros);
        let utc = i128::from(self.utc_unix_micros) + offset;
        i64::try_from(utc)
            .map(|utc_unix_micros| WorldInstant::new(monotonic_micros, utc_unix_micros))
            .map_err(|_| ClockError::UtcOutOfRange { monotonic_micros })
    }

    /// The instant at a UTC time, as a planner states a route time.
    ///
    /// # Errors
    ///
    /// [`ClockError::OutsideMonotonicClock`] for a UTC time before the
    /// monotonic origin or past the end of the monotonic clock.
    pub fn at_utc(self, utc_unix_micros: i64) -> Result<WorldInstant, ClockError> {
        let offset = i128::from(utc_unix_micros) - i128::from(self.utc_unix_micros);
        let monotonic = i128::from(self.monotonic_micros) + offset;
        u64::try_from(monotonic)
            .map(|monotonic_micros| WorldInstant::new(monotonic_micros, utc_unix_micros))
            .map_err(|_| ClockError::OutsideMonotonicClock { utc_unix_micros })
    }
}

/// Why a correspondence gave no instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    /// The UTC value at a monotonic reading does not fit its range.
    UtcOutOfRange {
        /// The monotonic reading asked about, microseconds.
        monotonic_micros: u64,
    },
    /// A UTC time falls outside what the monotonic clock can express.
    OutsideMonotonicClock {
        /// The UTC time asked about, microseconds since the Unix epoch.
        utc_unix_micros: i64,
    },
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UtcOutOfRange { monotonic_micros } => write!(
                f,
                "UTC at monotonic {monotonic_micros} us is outside the representable range"
            ),
            Self::OutsideMonotonicClock { utc_unix_micros } => write!(
                f,
                "UTC {utc_unix_micros} us is outside the monotonic clock's range"
            ),
        }
    }
}

impl core::error::Error for ClockError {}

#[cfg(test)]
mod tests;
