//! NSMA WG16.99.050 (ANSI/TIA/EIA-804) ASCII antenna pattern reader
//! (`no_std` + `alloc`).

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::error::{PatternParseErrorKind, PropagationError};

use super::cut::{
    AntennaMetadata, CutPlane, PatternCut, PatternPolarisation, PatternSample,
    normalize_azimuth_cut,
};
use super::nsma_fields::{
    GainUnit, SampleUnit, nsma_err as err, parse_finite, parse_gunits, parse_nsma_sample,
    parse_patcut, parse_positive, update_mid_freq,
};
use super::tabulated::TabulatedAntennaPattern;

struct CutBuilder {
    plane: CutPlane,
    polarisation: Option<PatternPolarisation>,
    expected: Option<usize>,
    samples: Vec<PatternSample>,
}

struct NsmaState {
    meta: AntennaMetadata,
    rev_seen: bool,
    end_seen: bool,
    gain_unit: GainUnit,
    sample_unit: SampleUnit,
    mdgain_raw: Option<f64>,
    expected_cuts: Option<usize>,
    active_cut: Option<CutBuilder>,
    cuts: Vec<PatternCut>,
}

impl NsmaState {
    fn new() -> Self {
        Self {
            meta: AntennaMetadata::default(),
            rev_seen: false,
            end_seen: false,
            gain_unit: GainUnit::Dbi,
            sample_unit: SampleUnit::Dbr,
            mdgain_raw: None,
            expected_cuts: None,
            active_cut: None,
            cuts: Vec::new(),
        }
    }

    fn gain_max_dbi(&self) -> Result<f64, PropagationError> {
        let raw = self
            .mdgain_raw
            .ok_or_else(|| err(0, PatternParseErrorKind::MissingHeader("MDGAIN")))?;
        Ok(match self.gain_unit {
            GainUnit::Dbi => raw,
            GainUnit::Dbd => raw + super::cut::DBD_TO_DBI,
        })
    }
}

/// Parses an NSMA WG16.99.050 ASCII `.adf` pattern into [`TabulatedAntennaPattern`].
pub(crate) fn parse_nsma(text: &str) -> Result<TabulatedAntennaPattern, PropagationError> {
    let cleaned = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut state = NsmaState::new();
    for (idx, raw) in cleaned.lines().enumerate() {
        let line_num = idx.wrapping_add(1);
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if state.end_seen {
            return Err(err(
                line_num,
                PatternParseErrorKind::MalformedRecord("unexpected content after ENDFIL"),
            ));
        }
        process_nsma_line(&mut state, line, line_num)?;
    }
    finish_nsma(state)
}

fn process_nsma_line(
    state: &mut NsmaState,
    line: &str,
    line_num: usize,
) -> Result<(), PropagationError> {
    if let Some((raw_key, raw_val)) = line.split_once(":,") {
        let key = raw_key.trim().to_ascii_uppercase();
        let val = raw_val.trim();
        return dispatch_keyword(state, &key, val, line_num);
    }
    if state.active_cut.is_some() {
        let gain_max_dbi = state.gain_max_dbi()?;
        let sample_unit = state.sample_unit;
        if let Some(cut) = state.active_cut.as_mut() {
            let expected = cut.expected.ok_or_else(|| {
                err(
                    line_num,
                    PatternParseErrorKind::MissingHeader("NUPOIN/NUMAZP"),
                )
            })?;
            let sample = parse_nsma_sample(
                line,
                cut.samples.last().map(|s| s.angle_deg),
                cut.samples.len(),
                sample_unit,
                gain_max_dbi,
                line_num,
            )?;
            let new_len = cut.samples.len().wrapping_add(1);
            if new_len > expected {
                return Err(err(
                    line_num,
                    PatternParseErrorKind::InvalidCount {
                        expected,
                        found: new_len,
                    },
                ));
            }
            cut.samples.push(sample);
            return Ok(());
        }
    }
    Err(err(
        line_num,
        PatternParseErrorKind::MalformedRecord("expected NSMA `KEYWORD:,VALUE` record"),
    ))
}

fn dispatch_keyword(
    state: &mut NsmaState,
    key: &str,
    val: &str,
    line_num: usize,
) -> Result<(), PropagationError> {
    match key {
        "PATCUT" => {
            finalize_active_cut(state, line_num)?;
            let plane = parse_patcut(val, line_num)?;
            state.active_cut = Some(CutBuilder {
                plane,
                polarisation: None,
                expected: None,
                samples: Vec::new(),
            });
            Ok(())
        }
        "ENDFIL" => {
            finalize_active_cut(state, line_num)?;
            state.end_seen = true;
            Ok(())
        }
        "POLARI" => {
            let pol = PatternPolarisation::parse(val).ok_or_else(|| {
                err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("unrecognized POLARI"),
                )
            })?;
            if let Some(cut) = state.active_cut.as_mut() {
                cut.polarisation = Some(pol);
            } else {
                state.meta.polarisation = Some(pol);
            }
            Ok(())
        }
        "NUPOIN" | "NUMAZP" => {
            let count: usize = val
                .parse()
                .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("NUPOIN/NUMAZP")))?;
            if count < 2 {
                return Err(err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("cut point count must be at least 2"),
                ));
            }
            let cut = state.active_cut.as_mut().ok_or_else(|| {
                err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("NUPOIN/NUMAZP outside PATCUT block"),
                )
            })?;
            cut.expected = Some(count);
            cut.samples.reserve(count);
            Ok(())
        }
        "AZIMUT" | "ELVANG" => {
            if state.active_cut.is_none() {
                return Err(err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("cut orientation outside PATCUT block"),
                ));
            }
            let _angle = parse_finite(val, "cut angle", line_num)?;
            Ok(())
        }
        "FSTLST" | "SFACTR" => Ok(()),
        _ => dispatch_header_keyword(state, key, val, line_num),
    }
}

fn dispatch_header_keyword(
    state: &mut NsmaState,
    key: &str,
    val: &str,
    line_num: usize,
) -> Result<(), PropagationError> {
    match key {
        "REVNUM" => {
            let up = val.to_ascii_uppercase();
            if !up.contains("NSMA") && !up.contains("WG16.99") {
                return Err(err(
                    line_num,
                    PatternParseErrorKind::MalformedRecord("unsupported REVNUM"),
                ));
            }
            state.rev_seen = true;
        }
        "REVDAT" | "DTDATA" | "DESCR1" | "DESCR2" | "COMENT" | "FEEDER" | "FEDLTH" | "RADOME"
        | "PATTYP" | "NOFREQ" => {
            if key == "COMENT" && !val.is_empty() {
                state.meta.comment = Some(val.to_string());
            }
        }
        "ANTMAN" => state.meta.make = val.to_string(),
        "MODNUM" => state.meta.name = val.to_string(),
        "LOWFRQ" => {
            let f = parse_positive(val, "LOWFRQ", line_num)?;
            state.meta.low_frequency_mhz = Some(f);
            update_mid_freq(&mut state.meta);
        }
        "HGHFRQ" => {
            let f = parse_positive(val, "HGHFRQ", line_num)?;
            state.meta.high_frequency_mhz = Some(f);
            update_mid_freq(&mut state.meta);
        }
        "GUNITS" => {
            let (g_u, s_u) = parse_gunits(val, line_num)?;
            state.gain_unit = g_u;
            state.sample_unit = s_u;
        }
        "MDGAIN" => {
            let first = val.split(',').next().unwrap_or(val).trim();
            state.mdgain_raw = Some(parse_finite(first, "MDGAIN", line_num)?);
        }
        "AZWIDT" => state.meta.h_width_deg = Some(parse_positive(val, "AZWIDT", line_num)?),
        "ELWIDT" => state.meta.v_width_deg = Some(parse_positive(val, "ELWIDT", line_num)?),
        "ELTILT" => state.meta.tilt_deg = Some(parse_finite(val, "ELTILT", line_num)?),
        "FRTOBA" => state.meta.front_to_back_db = Some(parse_finite(val, "FRTOBA", line_num)?),
        "NUMCUT" => {
            let n: usize = val
                .parse()
                .map_err(|_| err(line_num, PatternParseErrorKind::BadNumber("NUMCUT")))?;
            state.expected_cuts = Some(n);
        }
        _ => {
            return Err(err(
                line_num,
                PatternParseErrorKind::MalformedRecord("unrecognized NSMA keyword"),
            ));
        }
    }
    Ok(())
}

fn finalize_active_cut(state: &mut NsmaState, line_num: usize) -> Result<(), PropagationError> {
    let Some(cut) = state.active_cut.take() else {
        return Ok(());
    };
    let expected = cut.expected.ok_or_else(|| {
        err(
            line_num,
            PatternParseErrorKind::MissingHeader("NUPOIN/NUMAZP"),
        )
    })?;
    if cut.samples.len() != expected {
        return Err(err(
            line_num,
            PatternParseErrorKind::InvalidCount {
                expected,
                found: cut.samples.len(),
            },
        ));
    }
    let pol = cut
        .polarisation
        .or(state.meta.polarisation)
        .unwrap_or(PatternPolarisation::Vertical);
    if state.meta.polarisation.is_none() {
        state.meta.polarisation = Some(pol);
    }
    state
        .cuts
        .push(PatternCut::new(cut.plane, pol, cut.samples)?);
    Ok(())
}

fn finish_nsma(state: NsmaState) -> Result<TabulatedAntennaPattern, PropagationError> {
    if !state.rev_seen && state.cuts.is_empty() && state.mdgain_raw.is_none() {
        return Err(err(0, PatternParseErrorKind::Empty));
    }
    if !state.rev_seen {
        return Err(err(0, PatternParseErrorKind::MissingHeader("REVNUM")));
    }
    if !state.end_seen {
        return Err(err(0, PatternParseErrorKind::MissingEndMarker));
    }
    let gain_max_dbi = state.gain_max_dbi()?;
    if let Some(expected) = state.expected_cuts {
        if state.cuts.len() != expected {
            return Err(err(
                0,
                PatternParseErrorKind::InvalidCount {
                    expected,
                    found: state.cuts.len(),
                },
            ));
        }
    }
    if state.cuts.is_empty() {
        return Err(err(0, PatternParseErrorKind::MissingHeader("PATCUT")));
    }
    let (mut az_cut, mut el_cut) =
        collect_nominal_cuts(&state.cuts, state.meta.polarisation, false);
    if az_cut.is_empty() && el_cut.is_empty() && !state.cuts.is_empty() {
        (az_cut, el_cut) = collect_nominal_cuts(&state.cuts, state.meta.polarisation, true);
    }
    TabulatedAntennaPattern::from_parts(state.meta, gain_max_dbi, az_cut, el_cut, state.cuts)
}

fn collect_nominal_cuts(
    cuts: &[PatternCut],
    preferred: Option<PatternPolarisation>,
    allow_cross_polar: bool,
) -> (Vec<PatternSample>, Vec<PatternSample>) {
    let az_cut = select_primary_cut(cuts, CutPlane::Azimuth, preferred, allow_cross_polar)
        .map_or_else(Vec::new, |c| normalize_azimuth_cut(c.samples()));
    let el_cut = select_primary_cut(cuts, CutPlane::Elevation, preferred, allow_cross_polar)
        .map_or_else(Vec::new, |c| {
            c.samples()
                .iter()
                .copied()
                .filter(|s| (-90.0..=90.0).contains(&s.angle_deg))
                .collect()
        });
    (az_cut, el_cut)
}

fn is_cross_polar(polarisation: PatternPolarisation) -> bool {
    matches!(
        polarisation,
        PatternPolarisation::CrossHv | PatternPolarisation::CrossVh
    )
}

fn select_primary_cut(
    cuts: &[PatternCut],
    plane: CutPlane,
    preferred: Option<PatternPolarisation>,
    allow_cross_polar: bool,
) -> Option<&PatternCut> {
    if let Some(pref) = preferred.filter(|p| allow_cross_polar || !is_cross_polar(*p)) {
        if let Some(found) = cuts
            .iter()
            .find(|c| c.plane() == plane && c.polarisation() == pref)
        {
            return Some(found);
        }
    }
    cuts.iter()
        .find(|c| {
            c.plane() == plane
                && matches!(
                    c.polarisation(),
                    PatternPolarisation::Vertical | PatternPolarisation::Horizontal
                )
        })
        .or_else(|| {
            cuts.iter().find(|c| {
                c.plane() == plane && (allow_cross_polar || !is_cross_polar(c.polarisation()))
            })
        })
}
