//! One [`WorldInstant`] stated in each domain library's own time types.
//!
//! Surveillance ages tracks on monotonic microseconds alone and relates them
//! to UTC through one [`TimeCorrespondence`] per exported stream. Airmass
//! decides validity on civil UTC milliseconds and age on monotonic
//! microseconds, and takes both as one [`EvaluationTime`]. Neither is
//! derived from the other here: a [`WorldInstant`] already carries both
//! readings, and these functions only restate them.
//!
//! [`TimeCorrespondence`]: surveillance_core::TimeCorrespondence
//! [`EvaluationTime`]: airmass_core::EvaluationTime
//! [`WorldInstant`]: world_core::WorldInstant

#[cfg(feature = "airmass")]
use world_core::WorldInstant;

/// The time Airmass evaluates validity and age at for `instant`.
///
/// Airmass resolves UTC to the millisecond; the instant's UTC is rounded
/// toward the past, so a product is never called valid early.
#[cfg(feature = "airmass")]
pub fn evaluation_time(instant: WorldInstant) -> airmass_core::EvaluationTime {
    airmass_core::EvaluationTime::new(
        utc_time(instant),
        airmass_core::MonotonicTime::from_micros(instant.monotonic_micros),
    )
}

/// The UTC of `instant` at Airmass's millisecond resolution, rounded toward
/// the past.
#[cfg(feature = "airmass")]
pub fn utc_time(instant: WorldInstant) -> airmass_core::UtcTime {
    airmass_core::UtcTime::from_unix_millis(instant.utc_unix_micros.div_euclid(1_000))
}

/// The correspondence a host attaches to an exported Surveillance stream,
/// from the one it drives this workspace with.
#[cfg(feature = "surveillance")]
pub fn time_correspondence(
    clocks: world_core::ClockCorrespondence,
) -> surveillance_core::TimeCorrespondence {
    surveillance_core::TimeCorrespondence {
        monotonic_micros: clocks.monotonic_micros,
        utc_unix_micros: clocks.utc_unix_micros,
    }
}

/// The correspondence of a received Surveillance stream, for placing a
/// planned UTC time on that stream's monotonic clock.
#[cfg(feature = "surveillance")]
pub fn clock_correspondence(
    stream: surveillance_core::TimeCorrespondence,
) -> world_core::ClockCorrespondence {
    world_core::ClockCorrespondence::new(stream.monotonic_micros, stream.utc_unix_micros)
}

#[cfg(all(test, feature = "moving"))]
mod tests;
