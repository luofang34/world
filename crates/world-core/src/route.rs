//! A route through space and time, and evenly spaced instants along it.
//!
//! A cockpit asks about the leg it is flying and a planner about a flight
//! that has not started; both state it as positions with the instant each is
//! reached. [`sample_route`] walks such a route at a fixed interval so a
//! traffic, weather or wind query is asked at stated places and times. The
//! number of samples is bounded up front: a route that needs more is refused,
//! never silently thinned.

use core::fmt;

use crate::{GeodeticPosition, WorldInstant};

/// A place on a route and the instant it is reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoutePoint {
    /// Where the vehicle is.
    pub position: GeodeticPosition,
    /// When it is there.
    pub at: WorldInstant,
}

/// How densely a route is sampled, and the most samples that may result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteSampling {
    /// Monotonic time between consecutive samples, microseconds.
    pub interval_micros: u64,
    /// Most samples one route may produce.
    pub max_samples: usize,
}

/// Why a route gave no samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteError {
    /// The route has no points.
    Empty,
    /// The sampling interval is zero.
    ZeroInterval,
    /// A point is malformed or on another datum than the first.
    PointNotUsable {
        /// Index of the offending point.
        index: usize,
    },
    /// A point is not later than the one before it on the monotonic clock.
    TimeNotIncreasing {
        /// Index of the offending point.
        index: usize,
    },
    /// The route needs more samples than the stated bound.
    TooManySamples {
        /// Samples the route needs at this interval.
        needed: u64,
        /// The stated bound.
        limit: usize,
    },
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("route has no points"),
            Self::ZeroInterval => f.write_str("route sampling interval is zero"),
            Self::PointNotUsable { index } => write!(
                f,
                "route point {index} is malformed or on another datum than the first"
            ),
            Self::TimeNotIncreasing { index } => {
                write!(f, "route point {index} is not later than the one before it")
            }
            Self::TooManySamples { needed, limit } => {
                write!(f, "route needs {needed} samples; the bound is {limit}")
            }
        }
    }
}

impl core::error::Error for RouteError {}

/// Evenly spaced points along a route, ending on its last point.
#[derive(Clone, Debug)]
pub struct RouteSamples<'a> {
    points: &'a [RoutePoint],
    interval_micros: u64,
    leg: usize,
    next_micros: Option<u64>,
}

/// Walk `points` every `sampling.interval_micros`, from the first point to
/// the last inclusive.
///
/// Between two points the position moves linearly in latitude, longitude
/// (the short way round) and height, and UTC moves linearly with monotonic
/// time. That is a straight line on a plate carrée chart, not a great
/// circle: a caller who needs the great circle states more points.
///
/// # Errors
///
/// [`RouteError`] for an empty or malformed route, a route whose monotonic
/// time does not increase, a zero interval, or a route that needs more than
/// `sampling.max_samples` samples.
pub fn sample_route(
    points: &[RoutePoint],
    sampling: RouteSampling,
) -> Result<RouteSamples<'_>, RouteError> {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return Err(RouteError::Empty);
    };
    if sampling.interval_micros == 0 {
        return Err(RouteError::ZeroInterval);
    }
    for (index, point) in points.iter().enumerate() {
        if !point.position.is_well_formed() || point.position.datum != first.position.datum {
            return Err(RouteError::PointNotUsable { index });
        }
    }
    for (index, pair) in points.windows(2).enumerate() {
        let in_order = matches!(
            pair,
            [earlier, later] if later.at.monotonic_micros > earlier.at.monotonic_micros
        );
        if !in_order {
            return Err(RouteError::TimeNotIncreasing { index: index + 1 });
        }
    }
    let span = last.at.monotonic_micros - first.at.monotonic_micros;
    let on_grid = span / sampling.interval_micros + 1;
    let needed = on_grid + u64::from(span % sampling.interval_micros != 0);
    if usize::try_from(needed).map_or(true, |needed| needed > sampling.max_samples) {
        return Err(RouteError::TooManySamples {
            needed,
            limit: sampling.max_samples,
        });
    }
    Ok(RouteSamples {
        points,
        interval_micros: sampling.interval_micros,
        leg: 0,
        next_micros: Some(first.at.monotonic_micros),
    })
}

impl Iterator for RouteSamples<'_> {
    type Item = RoutePoint;

    fn next(&mut self) -> Option<RoutePoint> {
        let at_micros = self.next_micros?;
        let last = self.points.last()?;
        while self
            .points
            .get(self.leg + 1)
            .is_some_and(|end| end.at.monotonic_micros < at_micros)
        {
            self.leg += 1;
        }
        let start = self.points.get(self.leg)?;
        let sample = match self.points.get(self.leg + 1) {
            Some(end) => interpolate(start, end, at_micros),
            None => *start,
        };
        let end_micros = last.at.monotonic_micros;
        self.next_micros = if at_micros >= end_micros {
            None
        } else {
            // The last point is always sampled, on the grid or not.
            Some(
                at_micros
                    .saturating_add(self.interval_micros)
                    .min(end_micros),
            )
        };
        Some(sample)
    }
}

fn interpolate(start: &RoutePoint, end: &RoutePoint, at_micros: u64) -> RoutePoint {
    let span = end.at.monotonic_micros - start.at.monotonic_micros;
    let into = at_micros
        .saturating_sub(start.at.monotonic_micros)
        .min(span);
    let fraction = if span == 0 {
        0.0
    } else {
        into as f64 / span as f64
    };
    let lerp = |from: f64, to: f64| from + (to - from) * fraction;
    let mut delta_lon = end.position.longitude_deg - start.position.longitude_deg;
    if delta_lon > 180.0 {
        delta_lon -= 360.0;
    } else if delta_lon < -180.0 {
        delta_lon += 360.0;
    }
    let mut longitude_deg = start.position.longitude_deg + delta_lon * fraction;
    if longitude_deg > 180.0 {
        longitude_deg -= 360.0;
    } else if longitude_deg < -180.0 {
        longitude_deg += 360.0;
    }
    let utc_span = i128::from(end.at.utc_unix_micros) - i128::from(start.at.utc_unix_micros);
    let utc_into = if span == 0 {
        0
    } else {
        utc_span * i128::from(into) / i128::from(span)
    };
    let utc_unix_micros = i64::try_from(i128::from(start.at.utc_unix_micros) + utc_into)
        .unwrap_or(end.at.utc_unix_micros);
    RoutePoint {
        position: GeodeticPosition {
            latitude_deg: lerp(start.position.latitude_deg, end.position.latitude_deg),
            longitude_deg,
            ellipsoidal_height_m: lerp(
                start.position.ellipsoidal_height_m,
                end.position.ellipsoidal_height_m,
            ),
            datum: start.position.datum,
        },
        at: WorldInstant::new(at_micros, utc_unix_micros),
    }
}

#[cfg(test)]
mod tests;
