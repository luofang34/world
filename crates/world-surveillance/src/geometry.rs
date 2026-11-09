//! Dead-reckoning projection and ownship-relative geometry / CPA kinematics.

use core::f64::consts::TAU;
use libm::{atan2, hypot, sqrt};
use world_core::{GeodeticPosition, LocalTangentPlane, Ned};

use crate::{TrackId, TrackPhase, TrackSnapshot};

/// Bounds on kinematic dead-reckoning extrapolation from a track's last fix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionBounds {
    /// Maximum extrapolation interval beyond `observed_at_us`, microseconds.
    pub max_extrapolate_us: u64,
    /// Maximum horizontal distance extrapolated from the reported position, meters.
    pub max_distance_m: f64,
}

impl Default for ProjectionBounds {
    fn default() -> Self {
        Self {
            max_extrapolate_us: 15_000_000,
            max_distance_m: 1_000.0,
        }
    }
}

/// Projects `track`'s geodetic position to `now_us` along its `velocity_ned_m_s`
/// in the track's [`LocalTangentPlane`], clamped by `bounds`.
pub fn project_geodetic(
    track: &TrackSnapshot,
    now_us: u64,
    bounds: ProjectionBounds,
) -> Option<GeodeticPosition> {
    if !track.position.is_well_formed() {
        return None;
    }
    let dt_us = now_us.saturating_sub(track.observed_at_us);
    if dt_us == 0 {
        return Some(track.position);
    }
    let clamped_us = dt_us.min(bounds.max_extrapolate_us);
    let dt_s = clamped_us as f64 * 1e-6;
    let mut dn = track.velocity_ned_m_s.north_m * dt_s;
    let mut de = track.velocity_ned_m_s.east_m * dt_s;
    let horiz = hypot(dn, de);
    if horiz > bounds.max_distance_m && horiz > 1e-9 {
        let scale = bounds.max_distance_m / horiz;
        dn *= scale;
        de *= scale;
    }
    let dd = track.velocity_ned_m_s.down_m * dt_s;
    let plane = LocalTangentPlane::new(track.position).ok()?;
    Some(plane.to_geodetic(Ned {
        north_m: dn,
        east_m: de,
        down_m: dd,
    }))
}

/// Ownship-relative geometry and linear closest-point-of-approach (CPA) metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeTarget {
    /// Track identifier.
    pub id: TrackId,
    /// Lifecycle phase at evaluation time.
    pub phase: TrackPhase,
    /// Target offset from ownship in ownship's local north-east-down frame, meters.
    pub offset_ned_m: Ned,
    /// Horizontal range in ownship's tangent plane, meters.
    pub horizontal_range_m: f64,
    /// Three-dimensional slant range, meters.
    pub slant_range_m: f64,
    /// True bearing from ownship in radians `[0, 2π)` clockwise from true north.
    pub true_bearing_rad: f64,
    /// Target altitude minus ownship altitude (`-offset_ned_m.down_m`), meters.
    pub relative_altitude_m: f64,
    /// Range closure rate (positive when closing), m/s.
    pub closure_rate_m_s: f64,
    /// Time until horizontal closest point of approach (`0.0` when diverging), seconds.
    pub time_to_cpa_s: f64,
    /// Horizontal separation at `time_to_cpa_s`, meters.
    pub cpa_horizontal_m: f64,
}

/// Computes [`RelativeTarget`] geometry for `track` relative to `ownship` at `now_us`.
pub fn relative_target(
    ownship: GeodeticPosition,
    ownship_vel_ned_m_s: Ned,
    track: &TrackSnapshot,
    now_us: u64,
    bounds: ProjectionBounds,
) -> Option<RelativeTarget> {
    let target_pos = project_geodetic(track, now_us, bounds)?;
    let plane = LocalTangentPlane::new(ownship).ok()?;
    let offset = plane.to_ned(target_pos).ok()?;
    let horizontal_range_m = hypot(offset.north_m, offset.east_m);
    let slant_range_m = sqrt(
        offset.north_m * offset.north_m
            + offset.east_m * offset.east_m
            + offset.down_m * offset.down_m,
    );
    let raw_bearing = atan2(offset.east_m, offset.north_m);
    let true_bearing_rad = if raw_bearing < 0.0 {
        raw_bearing + TAU
    } else {
        raw_bearing
    };

    let rel_vn = track.velocity_ned_m_s.north_m - ownship_vel_ned_m_s.north_m;
    let rel_ve = track.velocity_ned_m_s.east_m - ownship_vel_ned_m_s.east_m;
    let rel_vd = track.velocity_ned_m_s.down_m - ownship_vel_ned_m_s.down_m;

    let closure_rate_m_s = if slant_range_m > 1e-6 {
        -(offset.north_m * rel_vn + offset.east_m * rel_ve + offset.down_m * rel_vd) / slant_range_m
    } else {
        0.0
    };

    let v_horiz_sq = rel_vn * rel_vn + rel_ve * rel_ve;
    let r_dot_v = offset.north_m * rel_vn + offset.east_m * rel_ve;
    let time_to_cpa_s = if v_horiz_sq > 1e-6 && r_dot_v < 0.0 {
        -r_dot_v / v_horiz_sq
    } else {
        0.0
    };
    let cpa_n = offset.north_m + rel_vn * time_to_cpa_s;
    let cpa_e = offset.east_m + rel_ve * time_to_cpa_s;
    let cpa_horizontal_m = hypot(cpa_n, cpa_e);

    Some(RelativeTarget {
        id: track.id,
        phase: track.phase,
        offset_ned_m: offset,
        horizontal_range_m,
        slant_range_m,
        true_bearing_rad,
        relative_altitude_m: -offset.down_m,
        closure_rate_m_s,
        time_to_cpa_s,
        cpa_horizontal_m,
    })
}
