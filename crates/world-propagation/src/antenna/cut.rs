//! Tabulated antenna cut types, metadata, and angular interpolation
//! (`no_std` + `alloc`).

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::{PatternParseErrorKind, PropagationError};

/// Standard dipole-to-isotropic gain offset (`2.15 dB`), used to normalize
/// `dBd` antenna gains to `dBi`.
pub const DBD_TO_DBI: f64 = 2.15;

/// Plane of a tabulated radiation pattern cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CutPlane {
    /// Horizontal / azimuth plane (`0..360°`, `0°` = boresight).
    Azimuth,
    /// Vertical / elevation plane (`-90..=90°`, `0°` = horizon boresight).
    Elevation,
}

/// Polarization state of a pattern or an individual cut (`HH`, `VV`, `HV`, `VH`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatternPolarisation {
    /// Horizontal co-polar (`H` or `H/H`).
    Horizontal,
    /// Vertical co-polar (`V` or `V/V`).
    Vertical,
    /// Cross-polar horizontal transmit, vertical receive (`H/V`).
    CrossHv,
    /// Cross-polar vertical transmit, horizontal receive (`V/H`).
    CrossVh,
    /// Dual linear polarization (`DUAL`).
    Dual,
    /// Dual slant ±45° polarization (`45/45`, `±45`, `XPOL`).
    Slant45,
    /// Circular polarization (`CP`, `RHCP`, `LHCP`).
    Circular,
}

impl PatternPolarisation {
    /// Parses standard MSI/NSMA polarization designators case-insensitively.
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        let mut buf = [0_u8; 24];
        let len = trimmed.len();
        if len == 0 || len > buf.len() {
            return None;
        }
        let slice = buf.get_mut(..len)?;
        for (dst, src) in slice.iter_mut().zip(trimmed.as_bytes()) {
            *dst = src.to_ascii_uppercase();
        }
        let upper = core::str::from_utf8(slice).ok()?;
        match upper {
            "H" | "HH" | "H/H" | "HORIZONTAL" => Some(Self::Horizontal),
            "V" | "VV" | "V/V" | "VERTICAL" => Some(Self::Vertical),
            "HV" | "H/V" => Some(Self::CrossHv),
            "VH" | "V/H" => Some(Self::CrossVh),
            "DUAL" | "H/V+V/H" | "V/H+H/V" => Some(Self::Dual),
            "45" | "+45" | "-45" | "+-45" | "45/45" | "SLANT" | "SLANT45" | "XPOL" => {
                Some(Self::Slant45)
            }
            "CP" | "CIRC" | "CIRCULAR" | "RHCP" | "LHCP" => Some(Self::Circular),
            _ => None,
        }
    }
}

/// One angular sample in a radiation pattern cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatternSample {
    /// Cut angle in degrees (`0..360` for azimuth, `-90..=90` for elevation).
    pub angle_deg: f64,
    /// Attenuation relative to peak gain `gain_max_dbi`, dB (`0.0` at peak).
    pub attenuation_db: f64,
}

/// One parsed angular cut (`PATCUT` + `POLARI` in NSMA, or `HORIZONTAL`/`VERTICAL`
/// in MSI).
#[derive(Clone, Debug, PartialEq)]
pub struct PatternCut {
    plane: CutPlane,
    polarisation: PatternPolarisation,
    samples: Vec<PatternSample>,
}

impl PatternCut {
    /// Creates a cut after verifying samples are non-empty, finite, and strictly
    /// increasing in angle.
    ///
    /// # Errors
    ///
    /// [`PropagationError::InvalidPatternFormat`] on empty or unordered samples.
    pub fn new(
        plane: CutPlane,
        polarisation: PatternPolarisation,
        samples: Vec<PatternSample>,
    ) -> Result<Self, PropagationError> {
        if samples.is_empty() {
            return Err(PropagationError::InvalidPatternFormat {
                line: 0,
                kind: PatternParseErrorKind::Empty,
            });
        }
        let mut prev: Option<f64> = None;
        for (index, s) in samples.iter().enumerate() {
            if !s.angle_deg.is_finite() {
                return Err(PropagationError::InvalidPatternFormat {
                    line: 0,
                    kind: PatternParseErrorKind::InvalidAngle {
                        angle_deg: s.angle_deg,
                    },
                });
            }
            if !s.attenuation_db.is_finite() {
                return Err(PropagationError::InvalidPatternFormat {
                    line: 0,
                    kind: PatternParseErrorKind::NonFiniteValue { index },
                });
            }
            if prev.is_some_and(|p| s.angle_deg <= p) {
                return Err(PropagationError::InvalidPatternFormat {
                    line: 0,
                    kind: PatternParseErrorKind::NonMonotonicAngle { index },
                });
            }
            prev = Some(s.angle_deg);
        }
        Ok(Self {
            plane,
            polarisation,
            samples,
        })
    }

    /// Pattern cut plane (azimuth or elevation).
    pub const fn plane(&self) -> CutPlane {
        self.plane
    }

    /// Polarization associated with this cut.
    pub const fn polarisation(&self) -> PatternPolarisation {
        self.polarisation
    }

    /// Angular samples sorted by ascending angle.
    pub fn samples(&self) -> &[PatternSample] {
        &self.samples
    }
}

/// Metadata parsed from an MSI or NSMA pattern header.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AntennaMetadata {
    /// Model name (`NAME` in MSI, `MODNUM` in NSMA).
    pub name: String,
    /// Manufacturer (`MAKE` in MSI, `ANTMAN` in NSMA).
    pub make: String,
    /// Nominal or mid-band operating frequency, MHz.
    pub frequency_mhz: Option<f64>,
    /// Lower band edge frequency, MHz (NSMA `LOWFRQ`).
    pub low_frequency_mhz: Option<f64>,
    /// Upper band edge frequency, MHz (NSMA `HGHFRQ`).
    pub high_frequency_mhz: Option<f64>,
    /// Nominal antenna polarization.
    pub polarisation: Option<PatternPolarisation>,
    /// Mechanical or electrical downtilt, degrees (`TILT` / `ELTILT`).
    pub tilt_deg: Option<f64>,
    /// Front-to-back ratio, dB (`FRONT_TO_BACK` / `FRTOBA`).
    pub front_to_back_db: Option<f64>,
    /// Horizontal 3 dB beamwidth, degrees (`H_WIDTH` / `AZWIDT`).
    pub h_width_deg: Option<f64>,
    /// Vertical 3 dB beamwidth, degrees (`V_WIDTH` / `ELWIDT`).
    pub v_width_deg: Option<f64>,
    /// Optional header comment text.
    pub comment: Option<String>,
}

/// Circular linear interpolation across a `[0, 360)` sorted cut.
pub(crate) fn interpolate_circular_360(samples: &[PatternSample], angle_deg: f64) -> f64 {
    let Some(first) = samples.first() else {
        return 0.0;
    };
    let Some(last) = samples.last() else {
        return 0.0;
    };
    if samples.len() == 1 {
        return first.attenuation_db;
    }
    let idx = samples.partition_point(|s| s.angle_deg <= angle_deg);
    let (a0, v0, a1, v1) = if idx == 0 {
        (
            last.angle_deg - 360.0,
            last.attenuation_db,
            first.angle_deg,
            first.attenuation_db,
        )
    } else if idx >= samples.len() {
        (
            last.angle_deg,
            last.attenuation_db,
            first.angle_deg + 360.0,
            first.attenuation_db,
        )
    } else {
        let prev = samples.get(idx - 1).unwrap_or(first);
        let next = samples.get(idx).unwrap_or(last);
        (
            prev.angle_deg,
            prev.attenuation_db,
            next.angle_deg,
            next.attenuation_db,
        )
    };
    let span = a1 - a0;
    if span <= 0.0 {
        v0
    } else {
        v0 + ((angle_deg - a0) / span) * (v1 - v0)
    }
}

/// Clamped linear interpolation over a monotonically increasing slice.
pub(crate) fn interpolate_linear_clamped(samples: &[PatternSample], angle_deg: f64) -> f64 {
    let Some(first) = samples.first() else {
        return 0.0;
    };
    let Some(last) = samples.last() else {
        return 0.0;
    };
    let idx = samples.partition_point(|s| s.angle_deg <= angle_deg);
    if idx == 0 {
        return first.attenuation_db;
    }
    if idx >= samples.len() {
        return last.attenuation_db;
    }
    let prev = samples.get(idx - 1).unwrap_or(first);
    let next = samples.get(idx).unwrap_or(last);
    let span = next.angle_deg - prev.angle_deg;
    if span <= 0.0 {
        prev.attenuation_db
    } else {
        prev.attenuation_db
            + ((angle_deg - prev.angle_deg) / span) * (next.attenuation_db - prev.attenuation_db)
    }
}

/// Wraps any finite angle in degrees into `[0.0, 360.0)` using `libm::fmod`.
pub(crate) fn wrap_360(deg: f64) -> f64 {
    let r = libm::fmod(deg, 360.0);
    let wrapped = if r < 0.0 { r + 360.0 } else { r };
    if wrapped >= 360.0 || (360.0 - wrapped) < 1.0e-9 {
        0.0
    } else {
        wrapped
    }
}

/// Normalizes azimuth angles (such as `-180..=180` or `0..=360`) into `[0, 360)`,
/// sorting ascending and deduplicating coincident seam points (`-180` / `+180`,
/// `0` / `360`).
pub(crate) fn normalize_azimuth_cut(raw: &[PatternSample]) -> Vec<PatternSample> {
    let mut mapped: Vec<PatternSample> = raw
        .iter()
        .map(|s| PatternSample {
            angle_deg: wrap_360(s.angle_deg),
            attenuation_db: s.attenuation_db,
        })
        .collect();
    mapped.sort_by(|a, b| a.angle_deg.total_cmp(&b.angle_deg));
    let mut deduped: Vec<PatternSample> = Vec::with_capacity(mapped.len());
    for sample in mapped {
        if let Some(prev) = deduped.last() {
            if libm::fabs(sample.angle_deg - prev.angle_deg) < 1.0e-9 {
                continue;
            }
        }
        deduped.push(sample);
    }
    deduped
}
