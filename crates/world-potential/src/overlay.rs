//! A gridded local anomaly to add on top of a global harmonic model.
//!
//! Anomaly maps (aeromagnetic surveys, gravity-anomaly grids) resolve
//! wavelengths far shorter than any global expansion. The overlay holds
//! north-east-down anomaly vectors on a regular latitude-longitude grid on
//! the body's reference ellipsoid and interpolates them bilinearly. It is
//! applied at every height: upward continuation is not modelled, so it
//! belongs near the altitude the map was compiled for.

use libm::{cos, fmod, sin};
use world_core::{DecimalYear, Ecef, Ellipsoid};

use crate::field::{FieldSample, VectorField};
use crate::{Derivatives, FieldError, Workspace};

/// Where the grid nodes are: `rows` latitudes from `south_deg` north in
/// steps of `latitude_step_deg`, `columns` longitudes from `west_deg` east
/// in steps of `longitude_step_deg`, geodetic on the overlay's ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGeometry {
    /// Latitude of the first row, degrees.
    pub south_deg: f64,
    /// Longitude of the first column, degrees.
    pub west_deg: f64,
    /// Row spacing, degrees.
    pub latitude_step_deg: f64,
    /// Column spacing, degrees.
    pub longitude_step_deg: f64,
    /// Number of latitudes, at least 2.
    pub rows: usize,
    /// Number of longitudes, at least 2.
    pub columns: usize,
}

/// What a position off the grid gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutsideCoverage {
    /// No anomaly: the global model alone, as where nothing was surveyed.
    Zero,
    /// [`FieldError::OutsideCoverage`], for a consumer that must know.
    Refuse,
}

/// North-east-down anomaly vectors on a [`GridGeometry`], in the unit of
/// the field they are added to. `values` holds three per node, row by row
/// from the south, west to east within a row. A scalar gravity anomaly is
/// a down-only grid.
#[derive(Clone, Debug, PartialEq)]
pub struct AnomalyGrid<B> {
    geometry: GridGeometry,
    values: B,
    shape: Ellipsoid,
    outside: OutsideCoverage,
}

impl<B: AsRef<[f64]>> AnomalyGrid<B> {
    /// The overlay of `values` on `geometry`, latitudes on `shape` (a
    /// sphere is an ellipsoid of zero flattening).
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidModel`] for fewer than two rows or columns,
    /// non-positive or non-finite steps or corner, or a value count other
    /// than 3 × rows × columns.
    pub fn new(
        geometry: GridGeometry,
        values: B,
        shape: Ellipsoid,
        outside: OutsideCoverage,
    ) -> Result<Self, FieldError> {
        let g = geometry;
        let finite = [g.south_deg, g.west_deg].iter().all(|v| v.is_finite());
        let steps = [g.latitude_step_deg, g.longitude_step_deg]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0);
        let count = g.rows.checked_mul(g.columns).and_then(|c| c.checked_mul(3));
        if !finite || !steps || g.rows < 2 || g.columns < 2 {
            return Err(FieldError::InvalidModel);
        }
        if count != Some(values.as_ref().len()) {
            return Err(FieldError::InvalidModel);
        }
        Ok(Self {
            geometry,
            values,
            shape,
            outside,
        })
    }

    /// The grid.
    pub fn geometry(&self) -> GridGeometry {
        self.geometry
    }

    fn node(&self, row: usize, column: usize) -> [f64; 3] {
        let i = 3 * (row * self.geometry.columns + column);
        let v = self.values.as_ref();
        let at = |k: usize| v.get(i + k).copied().unwrap_or(0.0);
        [at(0), at(1), at(2)]
    }

    /// The interpolated north-east-down anomaly at a geodetic latitude and
    /// longitude, or `None` off the grid.
    pub fn anomaly_ned(&self, latitude_deg: f64, longitude_deg: f64) -> Option<[f64; 3]> {
        let g = self.geometry;
        let fr = (latitude_deg - g.south_deg) / g.latitude_step_deg;
        let east = fmod(fmod(longitude_deg - g.west_deg, 360.0) + 360.0, 360.0);
        let fc = east / g.longitude_step_deg;
        let (last_row, last_col) = ((g.rows - 1) as f64, (g.columns - 1) as f64);
        if !(0.0..=last_row).contains(&fr) || !(0.0..=last_col).contains(&fc) {
            return None;
        }
        // Clamped so a point on the last row or column interpolates within
        // the final cell instead of past it.
        let r0 = (fr as usize).min(g.rows - 2);
        let c0 = (fc as usize).min(g.columns - 2);
        let (tr, tc) = (fr - r0 as f64, fc - c0 as f64);
        let (a, b) = (self.node(r0, c0), self.node(r0, c0 + 1));
        let (c, d) = (self.node(r0 + 1, c0), self.node(r0 + 1, c0 + 1));
        Some([0, 1, 2].map(|k| {
            let pick = |v: [f64; 3]| v.get(k).copied().unwrap_or(0.0);
            let south = pick(a) + tc * (pick(b) - pick(a));
            let north = pick(c) + tc * (pick(d) - pick(c));
            south + tr * (north - south)
        }))
    }
}

/// A north-east-down vector at geodetic (latitude, longitude) in
/// body-fixed Cartesian axes.
fn ned_to_body([n, e, d]: [f64; 3], latitude_deg: f64, longitude_deg: f64) -> [f64; 3] {
    let (lat, lon) = (latitude_deg.to_radians(), longitude_deg.to_radians());
    let (sl, cl, so, co) = (sin(lat), cos(lat), sin(lon), cos(lon));
    [
        -sl * co * n - so * e - cl * co * d,
        -sl * so * n + co * e - cl * so * d,
        cl * n - sl * d,
    ]
}

impl<B: AsRef<[f64]>> VectorField for AnomalyGrid<B> {
    fn workspace_degree(&self) -> usize {
        0
    }

    /// The anomaly vector. A bilinear grid has no meaningful gradient, so
    /// the gradient is always `None` and a [`crate::Sum`] containing it
    /// reports none either.
    fn sample(
        &self,
        _workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        _at: DecimalYear,
        _derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        let [x_m, y_m, z_m] = position_m;
        if !position_m.iter().all(|v| v.is_finite()) {
            return Err(FieldError::InvalidPosition);
        }
        let (lat, lon, _) = self.shape.from_ecef(Ecef { x_m, y_m, z_m });
        let vector = match (self.anomaly_ned(lat, lon), self.outside) {
            (Some(ned), _) => ned_to_body(ned, lat, lon),
            (None, OutsideCoverage::Zero) => [0.0; 3],
            (None, OutsideCoverage::Refuse) => return Err(FieldError::OutsideCoverage),
        };
        Ok(FieldSample {
            vector,
            gradient: None,
        })
    }
}

#[cfg(test)]
mod tests;
