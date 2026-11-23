//! A gridded scalar anomaly: total-field magnetic or gravity anomaly maps.

use world_core::Ellipsoid;

use super::lattice::Lattice;
use super::{GridGeometry, OutsideCoverage, feed_grid, feed_outside};
use crate::content::{feed_ellipsoid, feed_u8, kind};
use crate::{CanonicalContent, FieldError};

/// One scalar per node of a [`GridGeometry`], in the unit of the quantity
/// mapped (nT for a total-field anomaly, m/s² for a gravity anomaly), row
/// by row from the south, west to east within a row. It interpolates and
/// wraps across the ±180° seam exactly as [`super::AnomalyGrid`] does.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarAnomalyGrid<B> {
    lattice: Lattice<B>,
    shape: Ellipsoid,
    outside: OutsideCoverage,
}

impl<B: AsRef<[f64]>> ScalarAnomalyGrid<B> {
    /// The overlay of `values` on `geometry`, latitudes on `shape`.
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidModel`] as for [`super::AnomalyGrid::new`],
    /// with a value count of rows × columns.
    pub fn new(
        geometry: GridGeometry,
        values: B,
        shape: Ellipsoid,
        outside: OutsideCoverage,
    ) -> Result<Self, FieldError> {
        Ok(Self {
            lattice: Lattice::new(geometry, values, 1)?,
            shape,
            outside,
        })
    }

    /// The grid.
    pub fn geometry(&self) -> GridGeometry {
        self.lattice.geometry
    }

    /// The node values.
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

    /// The interpolated value at a geodetic latitude and longitude, or
    /// `None` off the grid.
    pub fn anomaly(&self, latitude_deg: f64, longitude_deg: f64) -> Option<f64> {
        let [v] = self.lattice.interpolate::<1>(latitude_deg, longitude_deg)?;
        Some(v)
    }

    /// The anomaly at a body-fixed Cartesian position, metres. Off the
    /// grid it is zero or an error according to [`Self::outside`].
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidPosition`] for a non-finite coordinate, and
    /// [`FieldError::OutsideCoverage`] off a grid that refuses.
    pub fn sample(&self, position_m: [f64; 3]) -> Result<f64, FieldError> {
        let [x_m, y_m, z_m] = position_m;
        if !position_m.iter().all(|v| v.is_finite()) {
            return Err(FieldError::InvalidPosition);
        }
        let (lat, lon, _) = self.shape.from_ecef(world_core::Ecef { x_m, y_m, z_m });
        match (self.anomaly(lat, lon), self.outside) {
            (Some(v), _) => Ok(v),
            (None, OutsideCoverage::Zero) => Ok(0.0),
            (None, OutsideCoverage::Refuse) => Err(FieldError::OutsideCoverage),
        }
    }
}

impl<B: AsRef<[f64]>> CanonicalContent for ScalarAnomalyGrid<B> {
    fn visit_content(&self, f: &mut dyn FnMut(&[u8])) {
        feed_u8(f, kind::SCALAR_GRID);
        feed_grid(f, self.lattice.geometry, self.values());
        feed_ellipsoid(f, self.shape);
        feed_outside(f, self.outside);
    }
}
