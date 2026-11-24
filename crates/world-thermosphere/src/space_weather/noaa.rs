//! Readers for NOAA Space Weather Prediction Center (SWPC) products:
//! `daily-solar-indices.txt` (`DSD.txt`) and `f107_cm_flux.json`.

mod json;

use alloc::vec::Vec;

use world_core::{DatasetId, Provenance};

use super::calendar::CalendarDate;
use super::error::{SpaceWeatherError, SpaceWeatherErrorKind};
use super::fields::{clean_text, parse_optional_flux, parse_ymd_tokens};
use super::record::{IndexQuality, SpaceWeatherDay, SpaceWeatherSource};
use super::table::SpaceWeatherTable;

const NOAA_TERMS: &str = "public domain: work of the U.S. Government (NOAA Space Weather Prediction Center); see weather.gov/disclaimer";

/// Provenance metadata for NOAA SWPC Daily Solar Data (`DSD.txt`).
pub const NOAA_DSD_PROVENANCE: Provenance = Provenance {
    dataset: DatasetId {
        name: "NOAA SWPC DSD",
        edition: "daily-solar-indices.txt",
    },
    publisher: "U.S. Dept. of Commerce, NOAA, Space Weather Prediction Center",
    source_url: "https://services.swpc.noaa.gov/text/daily-solar-indices.txt",
    source_file: "daily-solar-indices.txt",
    source_sha256: "dynamic (live feed; caller-supplied text)",
    terms: NOAA_TERMS,
};

/// Provenance metadata for NOAA SWPC 10.7 cm radio flux JSON.
pub const NOAA_F107_PROVENANCE: Provenance = Provenance {
    dataset: DatasetId {
        name: "NOAA SWPC F10.7",
        edition: "f107_cm_flux.json",
    },
    publisher: "U.S. Dept. of Commerce, NOAA, Space Weather Prediction Center",
    source_url: "https://services.swpc.noaa.gov/json/f107_cm_flux.json",
    source_file: "f107_cm_flux.json",
    source_sha256: "dynamic (live feed; caller-supplied text)",
    terms: NOAA_TERMS,
};

/// Penticton reporting schedule recorded in `f107_cm_flux.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReportingSchedule {
    /// Morning flux report (typically 17:00 UTC).
    Morning,
    /// Local noon flux report (typically 20:00 UTC; standard daily F10.7).
    Noon,
    /// Afternoon flux report (typically 22:00 UTC).
    Afternoon,
    /// Any unrecognized or null schedule label.
    Other,
}

/// One observation entry parsed from NOAA SWPC `f107_cm_flux.json`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoaaF107Entry {
    /// UTC calendar date from `time_tag`.
    pub date: CalendarDate,
    /// Whole seconds into the UTC day from `time_tag`.
    pub time_seconds: u32,
    /// Observing frequency in MHz (2800 MHz = 10.7 cm).
    pub frequency_mhz: Option<f64>,
    /// Observed 10.7 cm flux (s.f.u.), if present.
    pub flux: Option<f64>,
    /// Penticton reporting schedule (`Morning`, `Noon`, `Afternoon`).
    pub schedule: ReportingSchedule,
    /// Trailing 90-day mean reported on `Noon` entries (s.f.u.), if present.
    ///
    /// Note: this is a trailing 90-day average, distinct from NRLMSISE-00's
    /// centered 81-day average (`f107_observed_center81`).
    pub ninety_day_mean: Option<f64>,
}

/// Parses NOAA SWPC `daily-solar-indices.txt` (`DSD.txt`) into a
/// [`SpaceWeatherTable`].
///
/// Header lines beginning with `:` or `#` and blank lines are skipped. Missing
/// flux sentinels (`-999` / `-1`) become `None`. Because `DSD.txt` carries no
/// geomagnetic indices, `ap` and `daily_ap` are set to `None`, and the SESC
/// sunspot number column is ignored.
///
/// # Errors
///
/// [`SpaceWeatherError`] on malformed rows, invalid dates/numbers, or empty
/// input.
pub fn parse_noaa_dsd(text: &str) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    let cleaned = clean_text(text);
    let mut days = Vec::new();
    for (idx, raw_line) in cleaned.lines().enumerate() {
        let line_num = idx.wrapping_add(1);
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(':') || line.starts_with('#') {
            continue;
        }
        days.push(parse_dsd_row(line, line_num)?);
    }
    if days.is_empty() {
        return Err(SpaceWeatherError::at_query(SpaceWeatherErrorKind::Empty));
    }
    SpaceWeatherTable::from_days(days)
}

fn parse_dsd_row(line: &str, line_num: usize) -> Result<SpaceWeatherDay, SpaceWeatherError> {
    let mut fields = line.split_whitespace();
    let mut next_tok = || {
        fields.next().ok_or(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("expected date and 10.7cm flux in DSD row"),
        ))
    };
    let date = parse_ymd_tokens(next_tok()?, next_tok()?, next_tok()?, line_num)?;
    let f107_observed =
        parse_optional_flux(next_tok()?, line_num, "f107_observed", &[-999.0, -1.0])?;
    Ok(SpaceWeatherDay {
        date,
        ap: [None; 8],
        daily_ap: None,
        f107_observed,
        f107_adjusted: None,
        f107_observed_center81: None,
        f107_adjusted_center81: None,
        quality: IndexQuality::Provisional,
        source: SpaceWeatherSource::NoaaDailySolarData,
    })
}

/// Parses raw NOAA SWPC `f107_cm_flux.json` observation entries using a
/// zero-dependency pure-Rust JSON parser.
///
/// # Errors
///
/// [`SpaceWeatherError`] on JSON syntax errors, missing `time_tag`, invalid
/// numbers or non-2800 MHz frequencies, or an empty JSON array.
pub fn parse_noaa_f107_entries(text: &str) -> Result<Vec<NoaaF107Entry>, SpaceWeatherError> {
    let cleaned = clean_text(text);
    let entries = json::parse_entries(cleaned)?;
    if entries.is_empty() {
        return Err(SpaceWeatherError::at_query(SpaceWeatherErrorKind::Empty));
    }
    Ok(entries)
}

/// Parses NOAA SWPC `f107_cm_flux.json` into a daily [`SpaceWeatherTable`].
///
/// Multiple reports per UTC day (`Morning`, `Noon`, `Afternoon`) are aggregated
/// cleanly: `Noon` (20:00 UTC, matching `DSD.txt` and GFZ) is preferred whenever
/// present with a valid flux; otherwise available observations for that day are
/// averaged. Trailing `ninety_day_mean` is not placed in
/// `f107_observed_center81`.
///
/// # Errors
///
/// As [`parse_noaa_f107_entries`].
pub fn parse_noaa_f107_json(text: &str) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    let mut entries = parse_noaa_f107_entries(text)?;
    entries.sort_by_key(|e| (e.date, e.time_seconds));
    let mut days = Vec::new();
    let mut start = 0usize;
    while let Some(first) = entries.get(start) {
        let current_date = first.date;
        let mut end = start.wrapping_add(1);
        while let Some(next_entry) = entries.get(end) {
            if next_entry.date != current_date {
                break;
            }
            end = end.wrapping_add(1);
        }
        let group = entries.get(start..end).unwrap_or(&[]);
        days.push(aggregate_noaa_day(current_date, group));
        start = end;
    }
    SpaceWeatherTable::from_days(days)
}

fn aggregate_noaa_day(date: CalendarDate, group: &[NoaaF107Entry]) -> SpaceWeatherDay {
    let noon = group
        .iter()
        .rfind(|e| e.schedule == ReportingSchedule::Noon && e.flux.is_some())
        .and_then(|e| e.flux);
    let f107_observed = match noon {
        Some(v) => Some(v),
        None => {
            let mut sum = 0.0;
            let mut count = 0usize;
            for entry in group {
                if let Some(v) = entry.flux {
                    sum += v;
                    count = count.wrapping_add(1);
                }
            }
            if count > 0 {
                Some(sum / (count as f64))
            } else {
                None
            }
        }
    };
    SpaceWeatherDay {
        date,
        ap: [None; 8],
        daily_ap: None,
        f107_observed,
        f107_adjusted: None,
        f107_observed_center81: None,
        f107_adjusted_center81: None,
        quality: IndexQuality::Provisional,
        source: SpaceWeatherSource::NoaaF107Json,
    }
}
