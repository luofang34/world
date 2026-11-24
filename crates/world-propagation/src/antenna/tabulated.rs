//! Tabulated 2D antenna radiation pattern evaluator (`no_std` + `alloc`).

use alloc::vec::Vec;

use crate::antenna::{AntennaPattern, off_axis};
use crate::error::{PatternParseErrorKind, PropagationError, finite};

use super::cut::{
    AntennaMetadata, CutPlane, PatternCut, PatternPolarisation, PatternSample,
    interpolate_circular_360, interpolate_linear_clamped,
};

/// A 2D tabulated antenna gain pattern parsed from MSI Planet or NSMA WG16.99
/// text, with peak gain normalized to dBi.
#[derive(Clone, Debug, PartialEq)]
pub struct TabulatedAntennaPattern {
    metadata: AntennaMetadata,
    gain_max_dbi: f64,
    azimuth_cut: Vec<PatternSample>,
    elevation_cut: Vec<PatternSample>,
    cuts: Vec<PatternCut>,
}

impl TabulatedAntennaPattern {
    /// Parses an MSI Planet (`.msi` / `.pln`) antenna pattern file from UTF-8 text.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidPatternFormat`] on missing headers, bad
    /// counts, non-monotonic or out-of-range angles, or non-finite values.
    pub fn from_msi(text: &str) -> Result<Self, PropagationError> {
        super::msi::parse_msi(text)
    }

    /// Parses an NSMA WG16.99.050 (`.adf`) antenna pattern file from UTF-8 text.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidPatternFormat`] on missing headers or `ENDFIL`,
    /// bad counts, non-monotonic or out-of-range angles, or non-finite values.
    pub fn from_nsma(text: &str) -> Result<Self, PropagationError> {
        super::nsma::parse_nsma(text)
    }

    /// Constructs a tabulated pattern from pre-validated nominal cuts.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] if `gain_max_dbi` is non-finite, or
    /// [`PropagationError::InvalidPatternFormat`] if cuts are invalid or empty.
    pub(crate) fn from_parts(
        metadata: AntennaMetadata,
        gain_max_dbi: f64,
        azimuth_cut: Vec<PatternSample>,
        elevation_cut: Vec<PatternSample>,
        cuts: Vec<PatternCut>,
    ) -> Result<Self, PropagationError> {
        let gain_max_dbi = finite("gain_max_dbi", gain_max_dbi)?;
        if azimuth_cut.is_empty() && elevation_cut.is_empty() {
            return Err(PropagationError::InvalidPatternFormat {
                line: 0,
                kind: PatternParseErrorKind::Empty,
            });
        }
        Ok(Self {
            metadata,
            gain_max_dbi,
            azimuth_cut,
            elevation_cut,
            cuts,
        })
    }

    /// Pattern metadata (`NAME`, `MAKE`, frequency, polarization, tilt, …).
    pub fn metadata(&self) -> &AntennaMetadata {
        &self.metadata
    }

    /// Model name.
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Manufacturer name.
    pub fn make(&self) -> &str {
        &self.metadata.make
    }

    /// Nominal frequency in MHz, if specified.
    pub const fn frequency_mhz(&self) -> Option<f64> {
        self.metadata.frequency_mhz
    }

    /// Nominal polarization, if specified.
    pub const fn polarisation(&self) -> Option<PatternPolarisation> {
        self.metadata.polarisation
    }

    /// Tilt angle in degrees, if numeric tilt was supplied.
    pub const fn tilt_deg(&self) -> Option<f64> {
        self.metadata.tilt_deg
    }

    /// Front-to-back ratio in dB, if supplied.
    pub const fn front_to_back_db(&self) -> Option<f64> {
        self.metadata.front_to_back_db
    }

    /// Peak isotropic gain, dBi (normalized from dBd when needed).
    pub const fn gain_max_dbi(&self) -> f64 {
        self.gain_max_dbi
    }

    /// Nominal azimuth cut (`0..360°`, sorted ascending).
    pub fn azimuth_cut(&self) -> &[PatternSample] {
        &self.azimuth_cut
    }

    /// Nominal elevation cut (`-90..=90°`, sorted ascending).
    pub fn elevation_cut(&self) -> &[PatternSample] {
        &self.elevation_cut
    }

    /// All parsed cuts across planes and polarizations (`HH`, `VV`, `HV`, `VH`, …).
    pub fn cuts(&self) -> &[PatternCut] {
        &self.cuts
    }

    /// Finds the first cut matching `plane` and `polarisation`.
    pub fn cut(&self, plane: CutPlane, polarisation: PatternPolarisation) -> Option<&PatternCut> {
        self.cuts
            .iter()
            .find(|c| c.plane() == plane && c.polarisation() == polarisation)
    }

    /// Evaluates 2D separable gain (dBi) at `(azimuth_deg, elevation_deg)`.
    ///
    /// `azimuth_deg` wraps modulo `360°` over `[0, 360)`; `elevation_deg` must
    /// lie in `[-90, 90]`. Total attenuation combines the interpolated horizontal
    /// and vertical cut attenuations additively in dB.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidParameter`] if either angle is non-finite;
    /// [`PropagationError::AngleOutsidePattern`] if `elevation_deg` is outside
    /// `[-90, 90]`.
    pub fn gain_at_az_el_dbi(
        &self,
        azimuth_deg: f64,
        elevation_deg: f64,
    ) -> Result<f64, PropagationError> {
        let az = check_azimuth(azimuth_deg)?;
        let el = check_elevation(elevation_deg)?;
        let att_h = if self.azimuth_cut.is_empty() {
            0.0
        } else {
            interpolate_circular_360(&self.azimuth_cut, az)
        };
        let att_v = if self.elevation_cut.is_empty() {
            0.0
        } else {
            interpolate_linear_clamped(&self.elevation_cut, el)
        };
        Ok(self.gain_max_dbi - (att_h + att_v))
    }

    /// Evaluates 2D gain (dBi) using cuts recorded for `polarisation` when
    /// available (falling back to the nominal elevation cut if only an azimuth
    /// cut exists for that polarization).
    ///
    /// # Errors
    ///
    /// Same validation errors as [`Self::gain_at_az_el_dbi`], or
    /// [`PropagationError::AngleOutsidePattern`] if no cut exists for `polarisation`.
    pub fn gain_for_polarisation_dbi(
        &self,
        azimuth_deg: f64,
        elevation_deg: f64,
        polarisation: PatternPolarisation,
    ) -> Result<f64, PropagationError> {
        let az = check_azimuth(azimuth_deg)?;
        let el = check_elevation(elevation_deg)?;
        let az_cut = self.cut(CutPlane::Azimuth, polarisation);
        let el_cut = self.cut(CutPlane::Elevation, polarisation);
        if az_cut.is_none() && el_cut.is_none() {
            return Err(PropagationError::AngleOutsidePattern {
                off_axis_deg: azimuth_deg,
                min_deg: 0.0,
                max_deg: 0.0,
            });
        }
        let att_h = az_cut.map_or(0.0, |c| interpolate_circular_360(c.samples(), az));
        let att_v = el_cut.map_or_else(
            || {
                if self.elevation_cut.is_empty() {
                    0.0
                } else {
                    interpolate_linear_clamped(&self.elevation_cut, el)
                }
            },
            |c| interpolate_linear_clamped(c.samples(), el),
        );
        Ok(self.gain_max_dbi - (att_h + att_v))
    }
}

impl AntennaPattern for TabulatedAntennaPattern {
    fn gain_dbi(&self, off_axis_deg: f64) -> Result<f64, PropagationError> {
        let phi = off_axis(off_axis_deg, 0.0)?;
        if !self.azimuth_cut.is_empty() {
            self.gain_at_az_el_dbi(phi, 0.0)
        } else {
            self.gain_at_az_el_dbi(0.0, phi.min(90.0))
        }
    }
}

fn check_azimuth(azimuth_deg: f64) -> Result<f64, PropagationError> {
    let finite_az = finite("azimuth_deg", azimuth_deg)?;
    let r = libm::fmod(finite_az, 360.0);
    let wrapped = if r < 0.0 { r + 360.0 } else { r };
    Ok(if wrapped >= 360.0 { 0.0 } else { wrapped })
}

fn check_elevation(elevation_deg: f64) -> Result<f64, PropagationError> {
    let el = finite("elevation_deg", elevation_deg)?;
    if (-90.0..=90.0).contains(&el) {
        Ok(el)
    } else {
        Err(PropagationError::AngleOutsidePattern {
            off_axis_deg: el,
            min_deg: -90.0,
            max_deg: 90.0,
        })
    }
}
