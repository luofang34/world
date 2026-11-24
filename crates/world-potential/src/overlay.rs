//! A gridded local anomaly to add on top of a global harmonic model.
//!
//! Anomaly maps (aeromagnetic surveys, gravity-anomaly grids) resolve
//! wavelengths far shorter than any global expansion. The overlay holds
//! north-east-down anomaly vectors on a regular latitude-longitude grid on
//! the body's reference ellipsoid and interpolates them bilinearly. It is
//! applied at every height: upward continuation is not modelled, so it
//! belongs near the altitude the map was compiled for.

use libm::{cos, sin};
use world_core::{DecimalYear, Ecef, Ellipsoid};

use crate::content::{feed_ellipsoid, feed_f64, feed_f64s, feed_u8, feed_usize, kind};
use crate::field::{FieldSample, VectorField};
use crate::{CanonicalContent, Derivatives, FieldError, Workspace};

mod lattice;
mod scalar;

use lattice::Lattice;
pub use scalar::ScalarAnomalyGrid;

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
    lattice: Lattice<B>,
    shape: Ellipsoid,
    outside: OutsideCoverage,
}

impl GridGeometry {
    /// The first node: (latitude of the first row, longitude of the first
    /// column), degrees.
    pub fn origin_deg(&self) -> (f64, f64) {
        (self.south_deg, self.west_deg)
    }

    /// Node spacing: (row, column), degrees.
    pub fn spacing_deg(&self) -> (f64, f64) {
        (self.latitude_step_deg, self.longitude_step_deg)
    }

    /// Node counts: (rows, columns).
    pub fn dimensions(&self) -> (usize, usize) {
        (self.rows, self.columns)
    }
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
        Ok(Self {
            lattice: Lattice::new(geometry, values, 3)?,
            shape,
            outside,
        })
    }

    /// The grid.
    pub fn geometry(&self) -> GridGeometry {
        self.lattice.geometry
    }

    /// The node values, three per node, in the order the type documents.
    pub fn values(&self) -> &[f64] {
        self.lattice.values.as_ref()
    }

    /// The ellipsoid the latitudes are geodetic on.
    pub fn shape(&self) -> Ellipsoid {
        self.shape
    }

    /// What a position off the grid gets.
    pub fn outside(&self) -> OutsideCoverage {
        self.outside
    }

    /// Whether the columns span a full 360° of longitude, so that the
    /// grid interpolates across the ±180° seam.
    pub fn is_global_longitude(&self) -> bool {
        self.lattice.is_global()
    }

    /// The interpolated north-east-down anomaly at a geodetic latitude and
    /// longitude, or `None` off the grid.
    pub fn anomaly_ned(&self, latitude_deg: f64, longitude_deg: f64) -> Option<[f64; 3]> {
        self.lattice.interpolate(latitude_deg, longitude_deg)
    }
}

impl<B: AsRef<[f64]>> CanonicalContent for AnomalyGrid<B> {
    fn visit_content(&self, f: &mut dyn FnMut(&[u8])) {
        feed_u8(f, kind::VECTOR_GRID);
        feed_grid(f, self.lattice.geometry, self.values());
        feed_ellipsoid(f, self.shape);
        feed_outside(f, self.outside);
    }
}

pub(super) fn feed_grid(f: &mut dyn FnMut(&[u8]), g: GridGeometry, values: &[f64]) {
    for v in [
        g.south_deg,
        g.west_deg,
        g.latitude_step_deg,
        g.longitude_step_deg,
    ] {
        feed_f64(f, v);
    }
    feed_usize(f, g.rows);
    feed_usize(f, g.columns);
    feed_f64s(f, values);
}

pub(super) fn feed_outside(f: &mut dyn FnMut(&[u8]), outside: OutsideCoverage) {
    feed_u8(
        f,
        match outside {
            OutsideCoverage::Zero => 0,
            OutsideCoverage::Refuse => 1,
        },
    );
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

impl<B: AsRef<[f64]> + Send + Sync> VectorField for AnomalyGrid<B> {
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
