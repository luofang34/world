//! A body's internal magnetic field from Gauss coefficients.
//!
//! Main-field models publish Schmidt semi-normalised Gauss coefficients
//! gnm, hnm (nT) for V = a Σ (a/r)^(n+1) Pnm^S (gnm cos mλ + hnm sin mλ).
//! The fully normalised functions the synthesis uses are P̄nm = √(2n + 1)
//! Pnm^S (neither carries the Condon–Shortley phase), so the stored
//! coefficients are C̄nm = gnm/√(2n + 1) and S̄nm = hnm/√(2n + 1), and
//! V = a Φ. The field is B = −∇V.

use world_core::{DecimalYear, Provenance, ValidityWindow};

use crate::field::{FieldSample, VectorField};
use crate::harmonic::Synthesis;
use crate::{
    CoefficientSource, Derivatives, FieldError, HarmonicCoefficients, Truncation, Workspace,
    synthesize,
};

#[cfg(feature = "alloc")]
mod series;
#[cfg(feature = "alloc")]
pub use series::MagneticSeries;

/// A Schmidt semi-normalised Gauss coefficient of degree `n` as the fully
/// normalised coefficient the synthesis takes.
pub fn schmidt_to_geodesy(n: usize, schmidt: f64) -> f64 {
    schmidt / libm::sqrt((2 * n + 1) as f64)
}

/// A main-field model: coefficients at an epoch, their linear secular
/// variation per year, and the dates that extrapolation is valid for.
#[derive(Clone, Debug, PartialEq)]
pub struct MagneticModel<B> {
    radius_m: f64,
    epoch: DecimalYear,
    validity: ValidityWindow,
    main: HarmonicCoefficients<B>,
    secular: HarmonicCoefficients<B>,
    truncation: Truncation,
    provenance: Option<Provenance>,
}

/// The coefficients on a date, formed term by term.
struct Dated<'a, B> {
    main: &'a HarmonicCoefficients<B>,
    secular: &'a HarmonicCoefficients<B>,
    years: f64,
}

impl<B: AsRef<[f64]>> CoefficientSource for Dated<'_, B> {
    fn max_degree(&self) -> usize {
        self.main.degree()
    }

    fn coefficient(&self, n: usize, m: usize) -> (f64, f64) {
        let (c, s) = self.main.coefficient(n, m);
        let (dc, ds) = self.secular.coefficient(n, m);
        (c + self.years * dc, s + self.years * ds)
    }
}

impl<B: AsRef<[f64]>> MagneticModel<B> {
    /// A model with reference radius `radius_m`, fully normalised
    /// coefficients `main` (nT) at `epoch` and `secular` (nT/year).
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidModel`] for a non-finite or non-positive
    /// radius or a non-finite epoch, and
    /// [`FieldError::CoefficientStorage`] if the two sets differ in degree.
    pub fn new(
        radius_m: f64,
        epoch: DecimalYear,
        validity: ValidityWindow,
        main: HarmonicCoefficients<B>,
        secular: HarmonicCoefficients<B>,
    ) -> Result<Self, FieldError> {
        if !radius_m.is_finite() || radius_m <= 0.0 || !epoch.value().is_finite() {
            return Err(FieldError::InvalidModel);
        }
        if main.degree() != secular.degree() {
            return Err(FieldError::CoefficientStorage {
                degree: main.degree(),
                expected: crate::triangular_len(main.degree()),
                cosine: secular.cosine().len(),
                sine: secular.sine().len(),
            });
        }
        let truncation = Truncation::full(main.degree());
        Ok(Self {
            radius_m,
            epoch,
            validity,
            main,
            secular,
            truncation,
            provenance: None,
        })
    }

    /// The model summed over `truncation` instead.
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidTruncation`] or
    /// [`FieldError::DegreeExceedsModel`].
    pub fn with_truncation(mut self, truncation: Truncation) -> Result<Self, FieldError> {
        if !truncation.is_well_formed() {
            return Err(FieldError::InvalidTruncation);
        }
        if truncation.degree > self.main.degree() {
            return Err(FieldError::DegreeExceedsModel {
                degree: truncation.degree,
                available: self.main.degree(),
            });
        }
        self.truncation = truncation;
        Ok(self)
    }

    /// The model, valid over `validity` instead.
    pub fn with_validity(mut self, validity: ValidityWindow) -> Self {
        self.validity = validity;
        self
    }

    /// The model, recording the published file it came from.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Reference radius, m.
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// The coefficients' epoch.
    pub fn epoch(&self) -> DecimalYear {
        self.epoch
    }

    /// The dates the model is valid for.
    pub fn validity(&self) -> ValidityWindow {
        self.validity
    }

    /// The fully normalised coefficients at the epoch, nT.
    pub fn main(&self) -> &HarmonicCoefficients<B> {
        &self.main
    }

    /// Their secular variation, nT/year.
    pub fn secular(&self) -> &HarmonicCoefficients<B> {
        &self.secular
    }

    /// The published file, when known.
    pub fn provenance(&self) -> Option<Provenance> {
        self.provenance
    }

    /// B at `position_m` (body-fixed Cartesian metres) on `at`, nT, and on
    /// request its gradient, nT/m.
    ///
    /// # Errors
    ///
    /// [`FieldError::OutsideValidity`] for a date outside the validity
    /// window (never extrapolated), otherwise as [`synthesize`].
    pub fn field(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        if !self.validity.contains(at) {
            return Err(FieldError::OutsideValidity {
                at,
                validity: self.validity,
            });
        }
        let dated = Dated {
            main: &self.main,
            secular: &self.secular,
            years: at.value() - self.epoch.value(),
        };
        let Synthesis {
            gradient, hessian, ..
        } = synthesize(
            workspace,
            &dated,
            position_m,
            self.radius_m,
            self.truncation,
            derivatives,
        )?;
        let k = -self.radius_m;
        Ok(FieldSample {
            vector: gradient.map(|g| k * g),
            gradient: hessian.map(|h| h * k),
        })
    }
}

impl MagneticModel<[f64; 3]> {
    /// A centred dipole from its Schmidt Gauss coefficients g10, g11, h11
    /// (nT), constant over `validity`.
    ///
    /// # Errors
    ///
    /// As [`MagneticModel::new`], or [`FieldError::InvalidModel`] for a
    /// non-finite coefficient.
    pub fn dipole(
        radius_m: f64,
        [g10, g11, h11]: [f64; 3],
        epoch: DecimalYear,
        validity: ValidityWindow,
    ) -> Result<Self, FieldError> {
        if ![g10, g11, h11].iter().all(|g| g.is_finite()) {
            return Err(FieldError::InvalidModel);
        }
        let f = |g| schmidt_to_geodesy(1, g);
        let main = HarmonicCoefficients::new(1, [0.0, f(g10), f(g11)], [0.0, 0.0, f(h11)])?;
        let secular = HarmonicCoefficients::new(1, [0.0; 3], [0.0; 3])?;
        Self::new(radius_m, epoch, validity, main, secular)
    }

    /// A centred dipole whose field at the reference radius on its
    /// magnetic equator is `equatorial_nt`, with its axis through the
    /// point (`pole_latitude_deg`, `pole_longitude_deg`) where the field
    /// points into the body (for the Earth, near 80.7° N, 72.7° W).
    ///
    /// # Errors
    ///
    /// As [`MagneticModel::dipole`].
    pub fn tilted_dipole(
        radius_m: f64,
        equatorial_nt: f64,
        pole_latitude_deg: f64,
        pole_longitude_deg: f64,
        epoch: DecimalYear,
        validity: ValidityWindow,
    ) -> Result<Self, FieldError> {
        let colatitude = (90.0 - pole_latitude_deg).to_radians();
        let longitude = pole_longitude_deg.to_radians();
        let (sc, cc) = (libm::sin(colatitude), libm::cos(colatitude));
        let (sl, cl) = (libm::sin(longitude), libm::cos(longitude));
        let gauss = [
            -equatorial_nt * cc,
            -equatorial_nt * sc * cl,
            -equatorial_nt * sc * sl,
        ];
        Self::dipole(radius_m, gauss, epoch, validity)
    }
}

impl<B: AsRef<[f64]>> VectorField for MagneticModel<B> {
    fn workspace_degree(&self) -> usize {
        self.truncation.degree
    }

    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        self.field(workspace, position_m, at, derivatives)
    }
}

#[cfg(test)]
mod tests;
