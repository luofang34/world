//! Which tracks are near ownship.

use alloc::vec::Vec;

use surveillance_core::TrackSnapshot;

use crate::{Ownship, RelativeTraffic, relative_to_ownship};

/// Every track that can be placed at `now_micros`, nearest first.
///
/// Tracks that are refused (see [`relative_to_ownship`]) are left out: an
/// ownship shadow is not traffic, and a track outside its producer's bounds
/// has no position at this instant to rank.
pub fn traffic_by_range<T: AsRef<TrackSnapshot>>(
    ownship: &Ownship,
    tracks: impl IntoIterator<Item = T>,
    now_micros: u64,
) -> Vec<RelativeTraffic> {
    let mut relatives: Vec<RelativeTraffic> = tracks
        .into_iter()
        .filter_map(|track| relative_to_ownship(ownship, track.as_ref(), now_micros).ok())
        .collect();
    relatives.sort_by(|a, b| {
        a.horizontal_range_m
            .total_cmp(&b.horizontal_range_m)
            .then(a.id.cmp(&b.id))
    });
    relatives
}

/// The `count` nearest placeable tracks, nearest first.
pub fn nearest<T: AsRef<TrackSnapshot>>(
    ownship: &Ownship,
    tracks: impl IntoIterator<Item = T>,
    now_micros: u64,
    count: usize,
) -> Vec<RelativeTraffic> {
    let mut relatives = traffic_by_range(ownship, tracks, now_micros);
    relatives.truncate(count);
    relatives
}

/// The placeable tracks within `radius_m` horizontally, nearest first.
pub fn within_radius<T: AsRef<TrackSnapshot>>(
    ownship: &Ownship,
    tracks: impl IntoIterator<Item = T>,
    now_micros: u64,
    radius_m: f64,
) -> Vec<RelativeTraffic> {
    let mut relatives = traffic_by_range(ownship, tracks, now_micros);
    relatives.retain(|relative| relative.horizontal_range_m <= radius_m);
    relatives
}

#[cfg(test)]
mod tests;
