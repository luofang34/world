//! Ownship-relative geometry and linear closest approach for one track.

use core::f64::consts::TAU;

use libm::{atan2, cos, hypot, sin};
use surveillance_core::{TrackId, TrackKey, TrackSnapshot, VerticalRateSource};
use world_core::{GeodeticPosition, HorizontalDatum, LocalTangentPlane, Ned};

use crate::{TrackFix, TrafficRefusal, track_fix};

const METRES_PER_FOOT: f64 = 0.3048;
const METRES_PER_SECOND_PER_KNOT: f64 = 1_852.0 / 3_600.0;
const FEET_PER_MINUTE_TO_METRES_PER_SECOND: f64 = METRES_PER_FOOT / 60.0;

/// The vehicle traffic is asked about, or the point a planner stands at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ownship {
    /// WGS 84 position; the height is above the ellipsoid.
    pub position: GeodeticPosition,
    /// Velocity over the ground in local north-east-down, m/s. Zero for a
    /// fixed point.
    pub velocity_ned_m_s: Ned,
    /// Barometric pressure altitude in feet, when ownship has one. Traffic
    /// reports pressure altitude, so this is what makes a relative altitude
    /// free of the local pressure setting.
    pub pressure_altitude_ft: Option<f64>,
}

/// Which pair of altitudes a relative altitude compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeReference {
    /// Both altitudes are barometric pressure altitudes.
    Pressure,
    /// Both altitudes are geometric heights above the WGS 84 ellipsoid.
    Geometric,
}

/// A track's altitude relative to ownship, on one stated reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeAltitude {
    /// Track altitude minus ownship altitude, meters.
    pub above_ownship_m: f64,
    /// The reference both altitudes are on.
    pub reference: AltitudeReference,
}

/// Closest approach of ownship and a track if both hold their present
/// velocity.
///
/// This is the geometry of two straight lines. Neither vehicle is predicted
/// to fly one: the figures are a present-rate indication and lose meaning as
/// either turns or changes speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearCpa {
    /// Rate at which horizontal range shrinks, m/s; negative when it grows.
    pub closure_rate_m_s: f64,
    /// Time until the closest horizontal approach, seconds; zero when the
    /// range is already growing.
    pub time_to_cpa_s: f64,
    /// Horizontal separation at that time, meters.
    pub horizontal_miss_m: f64,
    /// Track velocity relative to ownship in ownship's local
    /// north-east-down, m/s. The vertical part is absent when the track
    /// reports no vertical rate.
    pub relative_velocity_ned_m_s: Ned,
}

/// One track as seen from ownship at one instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeTraffic {
    /// Stable engine-assigned track identity.
    pub id: TrackId,
    /// The track's primary over-the-air identity.
    pub key: TrackKey,
    /// The track's position at the instant, and how it was established.
    pub fix: TrackFix,
    /// North offset from ownship in ownship's tangent plane, meters.
    pub north_m: f64,
    /// East offset from ownship in ownship's tangent plane, meters.
    pub east_m: f64,
    /// Horizontal range, meters.
    pub horizontal_range_m: f64,
    /// True bearing from ownship in radians `[0, 2π)` clockwise from north.
    pub true_bearing_rad: f64,
    /// Altitude relative to ownship, absent when the two share no altitude
    /// reference.
    pub relative_altitude: Option<RelativeAltitude>,
    /// Linear closest approach, absent when the track has no current ground
    /// velocity.
    pub cpa: Option<LinearCpa>,
}

/// See `snapshot` from `ownship` at `now_micros` on the producer's monotonic
/// clock.
///
/// # Errors
///
/// [`TrafficRefusal`] when the track cannot be placed at `now_micros` inside
/// its producer's bounds (see [`track_fix`]), is an ownship shadow, or
/// `ownship` is malformed or not on WGS 84.
pub fn relative_to_ownship(
    ownship: &Ownship,
    snapshot: &TrackSnapshot,
    now_micros: u64,
) -> Result<RelativeTraffic, TrafficRefusal> {
    if snapshot.ownship_shadow {
        return Err(TrafficRefusal::OwnshipShadow { id: snapshot.id });
    }
    if ownship.position.datum != HorizontalDatum::Wgs84 {
        return Err(TrafficRefusal::ReferenceNotUsable);
    }
    let plane =
        LocalTangentPlane::new(ownship.position).map_err(|_| TrafficRefusal::ReferenceNotUsable)?;
    let fix = track_fix(snapshot, now_micros)?;
    // The track is placed at ownship's height so the offset is horizontal;
    // the vertical relation is stated separately, on a named reference.
    let level_with_ownship = GeodeticPosition::wgs84(
        fix.position.latitude_deg,
        fix.position.longitude_deg,
        ownship.position.ellipsoidal_height_m,
    );
    let offset = plane
        .to_ned(level_with_ownship)
        .map_err(|_| TrafficRefusal::ReferenceNotUsable)?;
    let bearing = atan2(offset.east_m, offset.north_m);
    Ok(RelativeTraffic {
        id: snapshot.id,
        key: snapshot.key,
        fix,
        north_m: offset.north_m,
        east_m: offset.east_m,
        horizontal_range_m: hypot(offset.north_m, offset.east_m),
        true_bearing_rad: if bearing < 0.0 {
            bearing + TAU
        } else {
            bearing
        },
        relative_altitude: relative_altitude(ownship, snapshot, &fix, now_micros),
        cpa: linear_cpa(ownship, snapshot, offset, now_micros),
    })
}

/// Pressure against pressure when both exist, else geometric against
/// geometric: altitudes on different references are never subtracted.
fn relative_altitude(
    ownship: &Ownship,
    snapshot: &TrackSnapshot,
    fix: &TrackFix,
    now_micros: u64,
) -> Option<RelativeAltitude> {
    if let (Some(track_ft), Some(own_ft)) = (fix.pressure_altitude_ft, ownship.pressure_altitude_ft)
    {
        return Some(RelativeAltitude {
            above_ownship_m: (f64::from(track_ft) - own_ft) * METRES_PER_FOOT,
            reference: AltitudeReference::Pressure,
        });
    }
    let bounds = snapshot.projection_bounds?;
    let geometric = snapshot.geometric_altitude_ft.as_ref()?;
    geometric
        .is_fresh(now_micros, bounds.coast_after_micros)
        .then(|| RelativeAltitude {
            above_ownship_m: f64::from(geometric.value) * METRES_PER_FOOT
                - ownship.position.ellipsoidal_height_m,
            reference: AltitudeReference::Geometric,
        })
}

fn linear_cpa(
    ownship: &Ownship,
    snapshot: &TrackSnapshot,
    offset: Ned,
    now_micros: u64,
) -> Option<LinearCpa> {
    let bounds = snapshot.projection_bounds?;
    let velocity = snapshot.velocity.as_ref()?;
    if !velocity.is_fresh(now_micros, bounds.velocity_fresh_micros) {
        return None;
    }
    let speed_m_s = velocity.value.ground_speed_kt? * METRES_PER_SECOND_PER_KNOT;
    let track_rad = velocity.value.track_angle_deg_true?.to_radians();
    // The track's own north and east are taken as ownship's: the frames
    // differ by the meridian convergence across the range, which is small
    // wherever a closest approach is worth stating.
    let north = speed_m_s * cos(track_rad) - ownship.velocity_ned_m_s.north_m;
    let east = speed_m_s * sin(track_rad) - ownship.velocity_ned_m_s.east_m;
    let down = velocity
        .value
        .vertical_rate_fpm
        .filter(|_| velocity.value.vertical_rate_source != Some(VerticalRateSource::Geometric))
        .map_or(0.0, |fpm| {
            -f64::from(fpm) * FEET_PER_MINUTE_TO_METRES_PER_SECOND - ownship.velocity_ned_m_s.down_m
        });

    let range_m = hypot(offset.north_m, offset.east_m);
    let range_rate = offset.north_m * north + offset.east_m * east;
    let speed_sq = north * north + east * east;
    let time_to_cpa_s = if speed_sq > 1e-6 && range_rate < 0.0 {
        -range_rate / speed_sq
    } else {
        0.0
    };
    Some(LinearCpa {
        closure_rate_m_s: if range_m > 1e-6 {
            -range_rate / range_m
        } else {
            0.0
        },
        time_to_cpa_s,
        horizontal_miss_m: hypot(
            offset.north_m + north * time_to_cpa_s,
            offset.east_m + east * time_to_cpa_s,
        ),
        relative_velocity_ned_m_s: Ned {
            north_m: north,
            east_m: east,
            down_m: down,
        },
    })
}

#[cfg(test)]
mod tests;
