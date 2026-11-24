//! Terrain profiles along a path, supplied by the caller.

use crate::error::PropagationError;

/// One terrain sample along a great-circle path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfilePoint {
    /// Distance from the transmitter, m.
    pub distance_m: f64,
    /// Terrain (plus any clutter the caller adds) above mean sea level, m.
    pub height_m: f64,
}

/// A terrain profile from transmitter to receiver.
///
/// The first and last samples lie under the two antennas. A terrain
/// database, a DEM tile cache or a plain slice implements this; the
/// models read it by index and never copy it.
pub trait TerrainProfile {
    /// Number of samples.
    fn len(&self) -> usize;

    /// The sample at `index`, or `None` past the end.
    fn point(&self, index: usize) -> Option<ProfilePoint>;

    /// Whether the profile has no samples.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl TerrainProfile for [ProfilePoint] {
    fn len(&self) -> usize {
        <[ProfilePoint]>::len(self)
    }

    fn point(&self, index: usize) -> Option<ProfilePoint> {
        self.get(index).copied()
    }
}

impl<const N: usize> TerrainProfile for [ProfilePoint; N] {
    fn len(&self) -> usize {
        N
    }

    fn point(&self, index: usize) -> Option<ProfilePoint> {
        self.get(index).copied()
    }
}

/// The same profile with every height replaced by zero: the smooth path
/// the delta-Bullington method compares the actual path against.
pub(crate) struct Flattened<'a, P: ?Sized>(pub(crate) &'a P);

impl<P: TerrainProfile + ?Sized> TerrainProfile for Flattened<'_, P> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn point(&self, index: usize) -> Option<ProfilePoint> {
        self.0.point(index).map(|p| ProfilePoint {
            distance_m: p.distance_m,
            height_m: 0.0,
        })
    }
}

/// The samples in order, with distances taken from the first sample and
/// converted to km, the unit the ITU-R profile formulae are written in.
pub(crate) fn samples_km<P: TerrainProfile + ?Sized>(
    profile: &P,
) -> impl Iterator<Item = (f64, f64)> + '_ {
    let origin = profile.point(0).map_or(0.0, |p| p.distance_m);
    (0..profile.len())
        .filter_map(move |i| profile.point(i))
        .map(move |p| ((p.distance_m - origin) * 1.0e-3, p.height_m))
}

/// Checks the profile has at least `required` finite samples at strictly
/// increasing distances, and returns the first and last heights and the
/// path length in km.
pub(crate) fn validate<P: TerrainProfile + ?Sized>(
    profile: &P,
    required: usize,
) -> Result<ProfileEnds, PropagationError> {
    let samples = profile.len();
    if samples < required {
        return Err(PropagationError::ProfileTooShort { samples, required });
    }
    let mut previous: Option<f64> = None;
    for index in 0..samples {
        let point = profile
            .point(index)
            .ok_or(PropagationError::InvalidProfileSample { index })?;
        let ordered = previous.is_none_or(|d| point.distance_m > d);
        if !point.distance_m.is_finite() || !point.height_m.is_finite() || !ordered {
            return Err(PropagationError::InvalidProfileSample { index });
        }
        previous = Some(point.distance_m);
    }
    let first = profile
        .point(0)
        .ok_or(PropagationError::InvalidProfileSample { index: 0 })?;
    let last = profile
        .point(samples - 1)
        .ok_or(PropagationError::InvalidProfileSample { index: samples - 1 })?;
    Ok(ProfileEnds {
        first_height_m: first.height_m,
        last_height_m: last.height_m,
        length_km: (last.distance_m - first.distance_m) * 1.0e-3,
    })
}

/// What [`validate`] learns about a profile's ends.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProfileEnds {
    /// Terrain height under the transmitter, m.
    pub first_height_m: f64,
    /// Terrain height under the receiver, m.
    pub last_height_m: f64,
    /// Path length, km.
    pub length_km: f64,
}
