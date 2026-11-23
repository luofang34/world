//! Models defined piecewise in time, such as IGRF's five-year epochs.

use alloc::vec::Vec;

use world_core::{DecimalYear, Provenance, ValidityWindow};

use crate::field::{FieldSample, VectorField};
use crate::{Derivatives, FieldError, MagneticModel, Workspace};

/// Consecutive [`MagneticModel`]s, each valid over its own window. A date
/// on a shared boundary is evaluated with the earlier segment; the series
/// is continuous there when each segment's secular variation is the
/// difference to the next epoch, as in IGRF.
#[derive(Clone, Debug, PartialEq)]
pub struct MagneticSeries {
    segments: Vec<MagneticModel<Vec<f64>>>,
    provenance: Option<Provenance>,
}

impl MagneticSeries {
    /// The series of `segments`, in date order.
    pub fn new(segments: Vec<MagneticModel<Vec<f64>>>) -> Self {
        Self {
            segments,
            provenance: None,
        }
    }

    /// The series, recording the published file it came from.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// The segments.
    pub fn segments(&self) -> &[MagneticModel<Vec<f64>>] {
        &self.segments
    }

    /// The published file, when known.
    pub fn provenance(&self) -> Option<Provenance> {
        self.provenance
    }

    /// From the first segment's start to the last one's end; `None` for an
    /// empty series.
    pub fn validity(&self) -> Option<ValidityWindow> {
        Some(ValidityWindow {
            from: self.segments.first()?.validity().from,
            until: self.segments.last()?.validity().until,
        })
    }

    /// The segment that holds `at`.
    pub fn segment_at(&self, at: DecimalYear) -> Option<&MagneticModel<Vec<f64>>> {
        self.segments.iter().find(|s| s.validity().contains(at))
    }
}

impl VectorField for MagneticSeries {
    fn workspace_degree(&self) -> usize {
        self.segments
            .iter()
            .map(VectorField::workspace_degree)
            .max()
            .unwrap_or(0)
    }

    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        let Some(segment) = self.segment_at(at) else {
            let validity = self.validity().unwrap_or(ValidityWindow {
                from: DecimalYear(f64::NAN),
                until: DecimalYear(f64::NAN),
            });
            return Err(FieldError::OutsideValidity { at, validity });
        };
        segment.field(workspace, position_m, at, derivatives)
    }
}
