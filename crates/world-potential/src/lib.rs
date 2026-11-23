//! Spherical-harmonic gravity and magnetic potential fields of any body.
//!
//! One synthesis core, [`synthesize`], sums a fully normalised harmonic
//! series at a body-fixed Cartesian position and returns the potential,
//! its gradient and optionally its Hessian. It runs in x, y, z with no
//! pole singularity, never allocates and takes its scratch memory from the
//! caller as a [`Workspace`], so its degree is bounded only by the buffer.
//!
//! [`GravityModel`] (GM, reference radius, C̄nm, S̄nm, tide system) and
//! [`MagneticModel`] (Schmidt Gauss coefficients with linear secular
//! variation and a validity window, converted at load time) are both
//! scaled syntheses of that core. Both implement [`VectorField`], which a
//! simulator holds as `dyn` (every field is `Send + Sync`) and composes
//! with [`Sum`] and a gridded [`AnomalyGrid`] overlay. Each model states
//! its canonical bytes through [`CanonicalContent`] for hashing by content.
//!
//! With the `alloc` feature (default) the crate also reads ICGEM `.gfc`,
//! PDS SHADR, WMM `.COF` and IGRF coefficient-table files from text, and carries IGRF-14
//! and EGM96 to degree 36 with their provenance. The core is `no_std`
//! either way; its dependencies are `world-core` and `libm`.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod content;
mod error;
mod field;
mod gravity;
mod harmonic;
mod magnetic;
mod overlay;
mod tensor;

#[cfg(feature = "alloc")]
mod embedded;
#[cfg(feature = "alloc")]
mod read;

pub use content::CanonicalContent;
pub use error::FieldError;
pub use field::{FieldSample, NoField, Sum, VectorField};
pub use gravity::{Gravitation, GravityModel, TideSystem};
pub use harmonic::{
    CoefficientSource, Derivatives, HarmonicCoefficients, Synthesis, Truncation, Workspace,
    synthesize, triangular_index, triangular_len, workspace_len,
};
pub use magnetic::{MagneticModel, schmidt_to_geodesy};
pub use overlay::{AnomalyGrid, GridGeometry, OutsideCoverage, ScalarAnomalyGrid};
pub use tensor::SymmetricTensor;

#[cfg(feature = "alloc")]
pub use embedded::{EGM96_DEGREE_36, IGRF14, egm96_degree_36, igrf14};
#[cfg(feature = "alloc")]
pub use field::FieldStack;
#[cfg(feature = "alloc")]
pub use harmonic::WorkspaceBuffer;
#[cfg(feature = "alloc")]
pub use magnetic::MagneticSeries;
#[cfg(feature = "alloc")]
pub use read::{
    CofModel, GaussRow, GfcModel, ParseError, ParseErrorKind, ShadrModel, read_cof, read_gfc,
    read_igrf_table, read_shadr,
};
