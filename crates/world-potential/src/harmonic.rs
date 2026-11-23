//! Spherical-harmonic coefficient sets and how far a sum is taken.
//!
//! Coefficients are fully normalised in the geodesy (4π) convention,
//! without the Condon–Shortley phase, so that the mean square of each
//! normalised surface harmonic over the sphere is one. Gravity models
//! publish them this way; magnetic Gauss coefficients are converted at
//! load time (see [`crate::schmidt_to_geodesy`]).

mod synthesis;
mod workspace;

pub use synthesis::{Derivatives, Synthesis, synthesize};
pub use workspace::{Workspace, workspace_len};

#[cfg(feature = "alloc")]
pub use workspace::WorkspaceBuffer;

use crate::FieldError;

/// Entries in one triangle of coefficients up to `degree`: every
/// `(n, m)` with `0 <= m <= n <= degree`.
pub const fn triangular_len(degree: usize) -> usize {
    (degree + 1) * (degree + 2) / 2
}

/// Where `(n, m)` sits in a triangle stored degree by degree, order by
/// order: `n (n + 1) / 2 + m`.
pub const fn triangular_index(n: usize, m: usize) -> usize {
    n * (n + 1) / 2 + m
}

/// Anything the synthesis can ask for the coefficient pair of `(n, m)`.
///
/// Implemented by [`HarmonicCoefficients`]; a time-dependent model
/// implements it to form dated coefficients term by term, so a dated
/// evaluation never needs a coefficient buffer of its own.
pub trait CoefficientSource {
    /// The highest degree held.
    fn max_degree(&self) -> usize;

    /// `(C̄nm, S̄nm)`, fully normalised; `(0, 0)` outside the stored set.
    fn coefficient(&self, n: usize, m: usize) -> (f64, f64);
}

/// One fully normalised coefficient set, cosine and sine triangles in
/// caller-chosen storage: fixed arrays, borrowed slices or (with `alloc`)
/// vectors. S̄n0 is never read: the sine terms of order zero vanish.
#[derive(Clone, Debug, PartialEq)]
pub struct HarmonicCoefficients<B> {
    degree: usize,
    cosine: B,
    sine: B,
}

impl<B: AsRef<[f64]>> HarmonicCoefficients<B> {
    /// Coefficients up to `degree`, each triangle laid out by
    /// [`triangular_index`].
    ///
    /// # Errors
    ///
    /// [`FieldError::CoefficientStorage`] unless both triangles hold
    /// exactly [`triangular_len`]`(degree)` entries.
    pub fn new(degree: usize, cosine: B, sine: B) -> Result<Self, FieldError> {
        let expected = triangular_len(degree);
        let (c, s) = (cosine.as_ref().len(), sine.as_ref().len());
        if c != expected || s != expected {
            return Err(FieldError::CoefficientStorage {
                degree,
                expected,
                cosine: c,
                sine: s,
            });
        }
        Ok(Self {
            degree,
            cosine,
            sine,
        })
    }

    /// The highest degree held.
    pub fn degree(&self) -> usize {
        self.degree
    }

    /// The cosine triangle.
    pub fn cosine(&self) -> &[f64] {
        self.cosine.as_ref()
    }

    /// The sine triangle.
    pub fn sine(&self) -> &[f64] {
        self.sine.as_ref()
    }
}

impl<B: AsRef<[f64]>> CoefficientSource for HarmonicCoefficients<B> {
    fn max_degree(&self) -> usize {
        self.degree
    }

    fn coefficient(&self, n: usize, m: usize) -> (f64, f64) {
        if m > n {
            return (0.0, 0.0);
        }
        let i = triangular_index(n, m);
        let c = self.cosine.as_ref().get(i).copied().unwrap_or(0.0);
        let s = self.sine.as_ref().get(i).copied().unwrap_or(0.0);
        (c, s)
    }
}

/// Which terms a synthesis sums: degrees `min_degree..=degree`, orders
/// `0..=order`.
///
/// `min_degree = 2` drops the central and degree-one terms, the usual way
/// to get the perturbing acceleration an orbit propagator adds to its own
/// two-body term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Truncation {
    /// Lowest degree summed.
    pub min_degree: usize,
    /// Highest degree summed.
    pub degree: usize,
    /// Highest order summed.
    pub order: usize,
}

impl Truncation {
    /// Every degree and order up to `degree`.
    pub const fn full(degree: usize) -> Self {
        Self {
            min_degree: 0,
            degree,
            order: degree,
        }
    }

    /// Whether the order and minimum degree are within the degree.
    pub fn is_well_formed(&self) -> bool {
        self.order <= self.degree && self.min_degree <= self.degree
    }
}

#[cfg(test)]
mod tests;
