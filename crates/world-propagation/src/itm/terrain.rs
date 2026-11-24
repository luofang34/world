//! Path geometry from the terrain profile: horizons, effective heights
//! and the terrain irregularity Δh (NTIA `FindHorizons`, `QuickPfl`,
//! `ComputeDeltaH`, `LinearLeastSquaresFit`).

use crate::error::PropagationError;
use crate::profile::TerrainProfile;

/// Relative tolerance on the spacing of an equally spaced profile.
const SPACING_TOLERANCE: f64 = 1.0e-6;

/// An equally spaced profile seen as ITM's `pfl` array: `np` intervals
/// of `xi` metres, heights 0..=np. Heights are read through the caller's
/// [`TerrainProfile`]; an index past the end reads NaN, which the final
/// finiteness check reports instead of a panic.
pub(crate) struct Pfl<'a, P: ?Sized> {
    profile: &'a P,
    /// Number of intervals.
    pub np: usize,
    /// Spacing, m.
    pub xi: f64,
}

impl<'a, P: TerrainProfile + ?Sized> Pfl<'a, P> {
    /// Checks the profile has at least two intervals at equal spacing.
    pub fn new(profile: &'a P) -> Result<Self, PropagationError> {
        let ends = crate::profile::validate(profile, 3)?;
        let np = profile.len() - 1;
        let xi = ends.length_km * 1.0e3 / np as f64;
        let origin = profile.point(0).map_or(0.0, |p| p.distance_m);
        for index in 0..=np {
            let expected = origin + index as f64 * xi;
            let point = profile
                .point(index)
                .ok_or(PropagationError::InvalidProfileSample { index })?;
            if libm::fabs(point.distance_m - expected) > SPACING_TOLERANCE * xi {
                return Err(PropagationError::InvalidProfileSample { index });
            }
        }
        Ok(Self { profile, np, xi })
    }
}

impl<P: TerrainProfile + ?Sized> Pfl<'_, P> {
    /// Height of sample `i`, m above sea level.
    pub fn h(&self, i: usize) -> f64 {
        self.profile.point(i).map_or(f64::NAN, |p| p.height_m)
    }

    /// Path length, m.
    pub fn distance_m(&self) -> f64 {
        self.np as f64 * self.xi
    }
}

/// Positive difference, C's `fdim`.
pub(crate) fn fdim(a: f64, b: f64) -> f64 {
    if a > b { a - b } else { 0.0 }
}

/// Mean height of the profile without its first and last 10 %, m.
pub(crate) fn system_height_m<P: TerrainProfile + ?Sized>(pfl: &Pfl<'_, P>) -> f64 {
    let p10 = (0.1 * pfl.np as f64) as usize;
    let last = pfl.np.saturating_sub(p10);
    let sum: f64 = (p10..=last).map(|i| pfl.h(i)).sum();
    sum / (pfl.np as f64 - 2.0 * p10 as f64 + 1.0)
}

/// What the profile analysis yields for each terminal (tx, rx) and the
/// path.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PathParameters {
    /// Horizon elevation angles, rad.
    pub theta_hzn: [f64; 2],
    /// Horizon distances, m.
    pub d_hzn_m: [f64; 2],
    /// Effective heights, m.
    pub h_e_m: [f64; 2],
    /// Terrain irregularity Δh, m.
    pub delta_h_m: f64,
    /// Path length, m.
    pub d_m: f64,
}

/// Horizon angles and distances seen from each terminal over an Earth of
/// radius `a_e_m` (`FindHorizons`, [TN101 eq. 6.15]).
fn find_horizons<P: TerrainProfile + ?Sized>(
    pfl: &Pfl<'_, P>,
    a_e_m: f64,
    h_m: [f64; 2],
) -> ([f64; 2], [f64; 2]) {
    let [h_tx, h_rx] = h_m;
    let d = pfl.distance_m();
    let z_tx = pfl.h(0) + h_tx;
    let z_rx = pfl.h(pfl.np) + h_rx;
    let mut theta = [
        (z_rx - z_tx) / d - d / (2.0 * a_e_m),
        -(z_rx - z_tx) / d - d / (2.0 * a_e_m),
    ];
    let mut d_hzn = [d, d];
    let mut d_tx = 0.0;
    let mut d_rx = d;
    for i in 1..pfl.np {
        d_tx += pfl.xi;
        d_rx -= pfl.xi;
        let theta_tx = (pfl.h(i) - z_tx) / d_tx - d_tx / (2.0 * a_e_m);
        let theta_rx = -(z_rx - pfl.h(i)) / d_rx - d_rx / (2.0 * a_e_m);
        if theta_tx > theta[0] {
            theta[0] = theta_tx;
            d_hzn[0] = d_tx;
        }
        if theta_rx > theta[1] {
            theta[1] = theta_rx;
            d_hzn[1] = d_rx;
        }
    }
    (theta, d_hzn)
}

/// The profile analysis of point-to-point mode (`QuickPfl`).
pub(crate) fn quick_pfl<P: TerrainProfile + ?Sized>(
    pfl: &Pfl<'_, P>,
    gamma_e: f64,
    h_m: [f64; 2],
) -> PathParameters {
    let d = pfl.distance_m();
    let a_e = 1.0 / gamma_e;
    let (mut theta, mut d_hzn) = find_horizons(pfl, a_e, h_m);
    // Hufford (1982): terrain counts from about 15 tower heights out.
    let d_start = (15.0 * h_m[0]).min(0.1 * d_hzn[0]);
    let d_end = d - (15.0 * h_m[1]).min(0.1 * d_hzn[1]);
    let delta_h = compute_delta_h(pfl, d_start, d_end);
    let mut h_e = [0.0; 2];
    let heights = |i: usize| pfl.h(i);
    if d_hzn[0] + d_hzn[1] > 1.5 * d {
        // Well inside line of sight: horizons come from the fitted surface.
        let (fit_tx, fit_rx) = least_squares(&heights, pfl.np, pfl.xi, d_start, d_end);
        h_e = [
            h_m[0] + fdim(pfl.h(0), fit_tx),
            h_m[1] + fdim(pfl.h(pfl.np), fit_rx),
        ];
        let horizon = |h_e: f64| {
            libm::sqrt(2.0 * h_e * a_e) * libm::exp(-0.07 * libm::sqrt(delta_h / h_e.max(5.0)))
        };
        d_hzn = [horizon(h_e[0]), horizon(h_e[1])];
        let combined = d_hzn[0] + d_hzn[1];
        if combined <= d {
            let q = (d / combined) * (d / combined);
            h_e = [h_e[0] * q, h_e[1] * q];
            d_hzn = [horizon(h_e[0]), horizon(h_e[1])];
        }
        for (theta_i, (&h_e_i, &d_hzn_i)) in theta.iter_mut().zip(h_e.iter().zip(d_hzn.iter())) {
            let q = libm::sqrt(2.0 * h_e_i * a_e);
            *theta_i = (0.65 * delta_h * (q / d_hzn_i - 1.0) - 2.0 * h_e_i) / q;
        }
    } else {
        let (fit_tx, _) = least_squares(&heights, pfl.np, pfl.xi, d_start, 0.9 * d_hzn[0]);
        h_e[0] = h_m[0] + fdim(pfl.h(0), fit_tx);
        let (_, fit_rx) = least_squares(&heights, pfl.np, pfl.xi, d - 0.9 * d_hzn[1], d_end);
        h_e[1] = h_m[1] + fdim(pfl.h(pfl.np), fit_rx);
    }
    PathParameters {
        theta_hzn: theta,
        d_hzn_m: d_hzn,
        h_e_m: h_e,
        delta_h_m: delta_h,
        d_m: d,
    }
}

/// Most samples the Δh resampling uses: 10 · 25 − 5.
const MAX_DELTA_H_SAMPLES: usize = 245;

/// The interdecile range of terrain heights about a fitted line between
/// `d_start` and `d_end`, scaled to the asymptotic Δh
/// (`ComputeDeltaH`, [ERL 79-ITS 67 eq. 3] inverted).
fn compute_delta_h<P: TerrainProfile + ?Sized>(
    pfl: &Pfl<'_, P>,
    d_start_m: f64,
    d_end_m: f64,
) -> f64 {
    let mut x_start = d_start_m / pfl.xi;
    let x_span = d_end_m / pfl.xi - x_start;
    if x_span < 2.0 {
        return 0.0;
    }
    let p10 = ((0.1 * (x_span + 8.0)) as i64).clamp(4, 25) as usize;
    let n = 10 * p10 - 5;
    let p90 = n - p10;
    let np_s = (n - 1) as f64;
    let step = x_span / np_s;
    let np = pfl.np as i64;
    let mut i = x_start as i64;
    x_start -= (i + 1) as f64;
    let mut samples = [0.0_f64; MAX_DELTA_H_SAMPLES];
    for slot in samples.iter_mut().take(n) {
        while x_start > 0.0 && i + 1 < np {
            x_start -= 1.0;
            i = i.wrapping_add(1);
        }
        let at = |k: i64| usize::try_from(k).map_or(f64::NAN, |k| pfl.h(k));
        *slot = at(i + 1) + (at(i + 1) - at(i)) * x_start;
        x_start += step;
    }
    let resampled = samples.get(..n).unwrap_or(&[]);
    let height = |k: usize| resampled.get(k).copied().unwrap_or(f64::NAN);
    let (mut fit, fit_end) = least_squares(&height, n - 1, 1.0, 0.0, np_s);
    let slope = (fit_end - fit) / np_s;
    let mut diffs = [0.0_f64; MAX_DELTA_H_SAMPLES];
    for (diff, &s) in diffs.iter_mut().zip(resampled) {
        *diff = s - fit;
        fit += slope;
    }
    let diffs = diffs.get_mut(..n).unwrap_or(&mut []);
    diffs.sort_unstable_by(|a, b| b.total_cmp(a));
    let q10 = diffs.get(p10 - 1).copied().unwrap_or(f64::NAN);
    let q90 = diffs.get(p90).copied().unwrap_or(f64::NAN);
    (q10 - q90) / (1.0 - 0.8 * libm::exp(-(d_end_m - d_start_m) / 50.0e3))
}

/// Heights at the two ends of a least-squares line through samples
/// `d_start..=d_end` of a profile with `np` intervals of `xi`
/// (`LinearLeastSquaresFit`); `h` reads sample heights.
pub(crate) fn least_squares(
    h: &dyn Fn(usize) -> f64,
    np: usize,
    xi: f64,
    d_start: f64,
    d_end: f64,
) -> (f64, f64) {
    let np_i = np as i64;
    let mut i_start = fdim(d_start / xi, 0.0) as i64;
    let mut i_end = np_i - fdim(np as f64, d_end / xi) as i64;
    if i_end <= i_start {
        i_start = fdim(i_start as f64, 1.0) as i64;
        i_end = np_i - fdim(np as f64, (i_end + 1) as f64) as i64;
    }
    let at = |k: i64| usize::try_from(k).map_or(f64::NAN, h);
    let x_length = (i_end - i_start) as f64;
    let mid = -0.5 * x_length;
    let mid_end = i_end as f64 + mid;
    let mut sum_y = 0.5 * (at(i_start) + at(i_end));
    let mut scaled_sum_y = 0.5 * (at(i_start) - at(i_end)) * mid;
    for offset in 1..(i_end - i_start) {
        let y = at(i_start + offset);
        sum_y += y;
        scaled_sum_y += y * (mid + offset as f64);
    }
    sum_y /= x_length;
    scaled_sum_y = scaled_sum_y * 12.0 / ((x_length * x_length + 2.0) * x_length);
    (
        sum_y - scaled_sum_y * mid_end,
        sum_y + scaled_sum_y * (np as f64 - mid_end),
    )
}
