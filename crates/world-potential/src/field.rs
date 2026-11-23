//! A vector field of body-fixed position and date, and sums of them.

use core::ops::Add;

use world_core::DecimalYear;

use crate::{Derivatives, FieldError, SymmetricTensor, Workspace};

/// A field vector and, on request, its gradient, in body-fixed axes. The
/// unit is the field's: m/s² and 1/s² for gravity, nT and nT/m for a
/// magnetic field.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FieldSample {
    /// The field vector.
    pub vector: [f64; 3],
    /// ∂vector_i/∂x_j, symmetric for a potential field. `None` when
    /// [`Derivatives::Gradient`] was asked for, or when a part of a sum
    /// cannot give one.
    pub gradient: Option<SymmetricTensor>,
}

impl Add for FieldSample {
    type Output = Self;

    /// The sum. Its gradient is present only when both parts give one, so
    /// a partial tensor is never mistaken for the whole.
    fn add(self, other: Self) -> Self {
        let ([ax, ay, az], [bx, by, bz]) = (self.vector, other.vector);
        Self {
            vector: [ax + bx, ay + by, az + bz],
            gradient: match (self.gradient, other.gradient) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            },
        }
    }
}

/// A vector field of body-fixed Cartesian position (metres; z along the
/// body's rotation pole, x through its prime meridian) and date.
///
/// Object safe, so a simulator holds `Box<dyn VectorField>` per body and
/// per physical quantity. Evaluation borrows a caller-owned [`Workspace`]
/// of at least [`VectorField::workspace_degree`]; implementations never
/// allocate.
pub trait VectorField {
    /// The degree a workspace must hold to evaluate this field.
    fn workspace_degree(&self) -> usize;

    /// The field at `position_m` on `at`.
    ///
    /// # Errors
    ///
    /// The implementation's: a date outside its validity, a position
    /// outside its domain, or a workspace too small.
    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError>;
}

impl<T: VectorField + ?Sized> VectorField for &T {
    fn workspace_degree(&self) -> usize {
        (**self).workspace_degree()
    }

    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        (**self).sample(workspace, position_m, at, derivatives)
    }
}

#[cfg(feature = "alloc")]
impl<T: VectorField + ?Sized> VectorField for alloc::boxed::Box<T> {
    fn workspace_degree(&self) -> usize {
        (**self).workspace_degree()
    }

    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        (**self).sample(workspace, position_m, at, derivatives)
    }
}

/// A body with no field of this kind — the Moon and Mars have no global
/// magnetic dipole. Zero everywhere, at every date, with a zero gradient.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoField;

impl VectorField for NoField {
    fn workspace_degree(&self) -> usize {
        0
    }

    fn sample(
        &self,
        _workspace: &mut Workspace<'_>,
        _position_m: [f64; 3],
        _at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        Ok(FieldSample {
            vector: [0.0; 3],
            gradient: match derivatives {
                Derivatives::Gradient => None,
                Derivatives::Hessian => Some(SymmetricTensor::default()),
            },
        })
    }
}

/// Two fields of the same quantity added: typically a global model and a
/// local anomaly overlay. Either part's error is the sum's.
#[derive(Clone, Debug, PartialEq)]
pub struct Sum<A, B>(pub A, pub B);

impl<A: VectorField, B: VectorField> VectorField for Sum<A, B> {
    fn workspace_degree(&self) -> usize {
        self.0.workspace_degree().max(self.1.workspace_degree())
    }

    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        let a = self.0.sample(workspace, position_m, at, derivatives)?;
        let b = self.1.sample(workspace, position_m, at, derivatives)?;
        Ok(a + b)
    }
}

/// Any number of fields of the same quantity added, chosen at run time.
#[cfg(feature = "alloc")]
#[derive(Default)]
pub struct FieldStack {
    /// The fields summed, in order.
    pub layers: alloc::vec::Vec<alloc::boxed::Box<dyn VectorField>>,
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for FieldStack {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FieldStack")
            .field("layers", &self.layers.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl VectorField for FieldStack {
    fn workspace_degree(&self) -> usize {
        self.layers
            .iter()
            .map(|l| l.workspace_degree())
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
        let empty = NoField.sample(workspace, position_m, at, derivatives)?;
        self.layers.iter().try_fold(empty, |sum, layer| {
            Ok(sum + layer.sample(workspace, position_m, at, derivatives)?)
        })
    }
}

#[cfg(test)]
mod tests;
