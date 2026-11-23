//! A body's gravitational potential from fully normalised geodesy
//! coefficients.

use world_core::{DecimalYear, Provenance};

use crate::field::{FieldSample, VectorField};
use crate::harmonic::Synthesis;
use crate::{Derivatives, FieldError, HarmonicCoefficients, Truncation, Workspace, synthesize};

/// Which permanent-tide convention a model's C̄20 is stated in. The
/// convention changes C̄20 by about 4e-9 for the Earth; the model never
/// converts between them, so a consumer that mixes models checks this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TideSystem {
    /// The permanent tide's direct and indirect effects both removed.
    TideFree,
    /// The direct effect removed, the indirect (deformation) kept.
    ZeroTide,
    /// Both kept.
    MeanTide,
    /// The source does not say, or the body has no such convention.
    Unstated,
}

/// A gravitational potential U = (GM/R) Σ (R/r)^(n+1) P̄nm (C̄nm cos mλ +
/// S̄nm sin mλ), its acceleration ∇U and, on request, its gradient tensor
/// ∇∇U, in the body-fixed frame the coefficients are stated in.
///
/// This is gravitation only: a rotating body's centrifugal term belongs to
/// whoever chose the rotating frame. C̄00 is the central term and is
/// normally one; C̄1m are kept as given (zero when the origin is the centre
/// of mass). Use [`Truncation::min_degree`] to drop the central term.
#[derive(Clone, Debug, PartialEq)]
pub struct GravityModel<B> {
    gm_m3ps2: f64,
    radius_m: f64,
    coefficients: HarmonicCoefficients<B>,
    tide_system: TideSystem,
    truncation: Truncation,
    provenance: Option<Provenance>,
}

/// Gravitation at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gravitation {
    /// U, m²/s², positive (the geodesy sign).
    pub potential_m2ps2: f64,
    /// ∇U, m/s².
    pub acceleration_mps2: [f64; 3],
    /// ∇∇U, 1/s², when [`Derivatives::Hessian`] was asked for.
    pub gradient_per_s2: Option<crate::SymmetricTensor>,
}

impl<B: AsRef<[f64]>> GravityModel<B> {
    /// A model of `coefficients` for gravitational parameter `gm_m3ps2`
    /// and reference radius `radius_m`, evaluated to its full degree.
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidModel`] for a non-finite or non-positive GM or
    /// radius.
    pub fn new(
        gm_m3ps2: f64,
        radius_m: f64,
        coefficients: HarmonicCoefficients<B>,
        tide_system: TideSystem,
    ) -> Result<Self, FieldError> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(gm_m3ps2) || !positive(radius_m) {
            return Err(FieldError::InvalidModel);
        }
        let truncation = Truncation::full(coefficients.degree());
        Ok(Self {
            gm_m3ps2,
            radius_m,
            coefficients,
            tide_system,
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
        if truncation.degree > self.coefficients.degree() {
            return Err(FieldError::DegreeExceedsModel {
                degree: truncation.degree,
                available: self.coefficients.degree(),
            });
        }
        self.truncation = truncation;
        Ok(self)
    }

    /// The model, recording the published file it came from.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// GM, m³/s².
    pub fn gm_m3ps2(&self) -> f64 {
        self.gm_m3ps2
    }

    /// Reference radius, m.
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// The coefficients.
    pub fn coefficients(&self) -> &HarmonicCoefficients<B> {
        &self.coefficients
    }

    /// The permanent-tide convention of C̄20.
    pub fn tide_system(&self) -> TideSystem {
        self.tide_system
    }

    /// The terms summed.
    pub fn truncation(&self) -> Truncation {
        self.truncation
    }

    /// The published file, when known.
    pub fn provenance(&self) -> Option<Provenance> {
        self.provenance
    }

    /// Gravitation at `position_m`, body-fixed Cartesian metres.
    ///
    /// # Errors
    ///
    /// As [`synthesize`].
    pub fn gravitation(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        derivatives: Derivatives,
    ) -> Result<Gravitation, FieldError> {
        let Synthesis {
            potential,
            gradient,
            hessian,
        } = synthesize(
            workspace,
            &self.coefficients,
            position_m,
            self.radius_m,
            self.truncation,
            derivatives,
        )?;
        let k = self.gm_m3ps2 / self.radius_m;
        Ok(Gravitation {
            potential_m2ps2: k * potential,
            acceleration_mps2: gradient.map(|g| k * g),
            gradient_per_s2: hessian.map(|h| h * k),
        })
    }
}

impl GravityModel<[f64; 1]> {
    /// A point mass (or any spherically symmetric body).
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidModel`] as [`GravityModel::new`].
    pub fn point_mass(gm_m3ps2: f64, radius_m: f64) -> Result<Self, FieldError> {
        let coefficients = HarmonicCoefficients::new(0, [1.0], [0.0])?;
        Self::new(gm_m3ps2, radius_m, coefficients, TideSystem::Unstated)
    }
}

impl GravityModel<[f64; 6]> {
    /// The central term plus the oblateness J2 = −C20 (unnormalised), so
    /// C̄20 = −J2/√5.
    ///
    /// # Errors
    ///
    /// [`FieldError::InvalidModel`] as [`GravityModel::new`], or for a
    /// non-finite J2.
    pub fn oblate(
        gm_m3ps2: f64,
        radius_m: f64,
        j2: f64,
        tide_system: TideSystem,
    ) -> Result<Self, FieldError> {
        if !j2.is_finite() {
            return Err(FieldError::InvalidModel);
        }
        let c20 = -j2 / libm::sqrt(5.0);
        let coefficients = HarmonicCoefficients::new(2, [1.0, 0.0, 0.0, c20, 0.0, 0.0], [0.0; 6])?;
        Self::new(gm_m3ps2, radius_m, coefficients, tide_system)
    }
}

impl<B: AsRef<[f64]>> VectorField for GravityModel<B> {
    fn workspace_degree(&self) -> usize {
        self.truncation.degree
    }

    /// The gravitational acceleration, m/s², and its gradient, 1/s². A
    /// static field: the date is ignored.
    fn sample(
        &self,
        workspace: &mut Workspace<'_>,
        position_m: [f64; 3],
        _at: DecimalYear,
        derivatives: Derivatives,
    ) -> Result<FieldSample, FieldError> {
        let g = self.gravitation(workspace, position_m, derivatives)?;
        Ok(FieldSample {
            vector: g.acceleration_mps2,
            gradient: g.gradient_per_s2,
        })
    }
}

#[cfg(test)]
mod tests;
