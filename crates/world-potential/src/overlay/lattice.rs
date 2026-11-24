//! Bilinear interpolation on a regular latitude-longitude lattice, shared
//! by the vector and scalar overlays.

use libm::fmod;

use super::GridGeometry;
use crate::FieldError;

/// Longitude span, degrees, at which a grid closes on itself.
const FULL_CIRCLE_DEG: f64 = 360.0;

/// A lattice of `N`-component nodes. A grid whose columns span a full
/// 360° (`columns × longitude_step_deg`, with no repeated seam column)
/// is global in longitude and interpolates across the ±180° seam.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Lattice<B> {
    pub(super) geometry: GridGeometry,
    pub(super) values: B,
    global: bool,
}

impl<B: AsRef<[f64]>> Lattice<B> {
    pub(super) fn new(
        geometry: GridGeometry,
        values: B,
        components: usize,
    ) -> Result<Self, FieldError> {
        let g = geometry;
        let finite = [g.south_deg, g.west_deg].iter().all(|v| v.is_finite());
        let steps = [g.latitude_step_deg, g.longitude_step_deg]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0);
        let count = g
            .rows
            .checked_mul(g.columns)
            .and_then(|c| c.checked_mul(components));
        if !finite || !steps || g.rows < 2 || g.columns < 2 {
            return Err(FieldError::InvalidModel);
        }
        if count != Some(values.as_ref().len()) {
            return Err(FieldError::InvalidModel);
        }
        // Tolerance covers a step such as 0.1° that does not sum exactly.
        let span = g.columns as f64 * g.longitude_step_deg;
        let global = (span - FULL_CIRCLE_DEG).abs() < 1e-9;
        Ok(Self {
            geometry,
            values,
            global,
        })
    }

    pub(super) fn is_global(&self) -> bool {
        self.global
    }

    fn node<const N: usize>(&self, row: usize, column: usize) -> [f64; N] {
        let i = N * (row * self.geometry.columns + column);
        let v = self.values.as_ref();
        core::array::from_fn(|k| v.get(i + k).copied().unwrap_or(0.0))
    }

    /// The interpolated node value, or `None` off the grid.
    pub(super) fn interpolate<const N: usize>(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Option<[f64; N]> {
        let g = self.geometry;
        let fr = (latitude_deg - g.south_deg) / g.latitude_step_deg;
        let east = fmod(fmod(longitude_deg - g.west_deg, 360.0) + 360.0, 360.0);
        let fc = east / g.longitude_step_deg;
        let last_row = (g.rows - 1) as f64;
        // A global grid has a cell between its last column and its first.
        let cells = if self.global {
            g.columns
        } else {
            g.columns - 1
        };
        if !(0.0..=last_row).contains(&fr) || !(0.0..=cells as f64).contains(&fc) {
            return None;
        }
        // Clamped so a point on the last row or column interpolates within
        // the final cell instead of past it.
        let r0 = (fr as usize).min(g.rows - 2);
        let c0 = (fc as usize).min(cells - 1);
        let c1 = (c0 + 1) % g.columns;
        let (tr, tc) = (fr - r0 as f64, fc - c0 as f64);
        let (a, b) = (self.node::<N>(r0, c0), self.node::<N>(r0, c1));
        let (c, d) = (self.node::<N>(r0 + 1, c0), self.node::<N>(r0 + 1, c1));
        Some(core::array::from_fn(|k| {
            let pick = |v: [f64; N]| v.get(k).copied().unwrap_or(0.0);
            let south = pick(a) + tc * (pick(b) - pick(a));
            let north = pick(c) + tc * (pick(d) - pick(c));
            south + tr * (north - south)
        }))
    }
}
