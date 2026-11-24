//! CelesTrak CSV (`SW-Last5Years.csv` / `SW-All.csv`) reader.

use alloc::vec::Vec;

use crate::space_weather::error::{SpaceWeatherError, SpaceWeatherErrorKind};
use crate::space_weather::fields::{
    clean_text, parse_iso_date, parse_optional_ap, parse_optional_flux,
};
use crate::space_weather::record::{IndexQuality, SpaceWeatherDay, SpaceWeatherSource};
use crate::space_weather::table::SpaceWeatherTable;

struct CsvColumns {
    total: usize,
    date: usize,
    ap: [usize; 8],
    ap_avg: usize,
    f107_obs: usize,
    f107_adj: usize,
    f107_data_type: usize,
    f107_obs_center81: usize,
    f107_adj_center81: usize,
}

impl CsvColumns {
    fn from_header(header_line: &str, line_num: usize) -> Result<Self, SpaceWeatherError> {
        let cols: Vec<&str> = header_line.split(',').map(str::trim).collect();
        let find = |name: &'static str| {
            cols.iter()
                .position(|&c| c == name)
                .ok_or(SpaceWeatherError::at_line(
                    line_num,
                    SpaceWeatherErrorKind::MissingHeader(name),
                ))
        };
        Ok(Self {
            total: cols.len(),
            date: find("DATE")?,
            ap: [
                find("AP1")?,
                find("AP2")?,
                find("AP3")?,
                find("AP4")?,
                find("AP5")?,
                find("AP6")?,
                find("AP7")?,
                find("AP8")?,
            ],
            ap_avg: find("AP_AVG")?,
            f107_obs: find("F10.7_OBS")?,
            f107_adj: find("F10.7_ADJ")?,
            f107_data_type: find("F10.7_DATA_TYPE")?,
            f107_obs_center81: find("F10.7_OBS_CENTER81")?,
            f107_adj_center81: find("F10.7_ADJ_CENTER81")?,
        })
    }
}

/// Parses a CelesTrak space-weather CSV file (`SW-Last5Years.csv` or
/// `SW-All.csv`) into a [`SpaceWeatherTable`].
///
/// Validates all required header columns, handles `\r\n` and `\n` line
/// endings, maps `OBS`/`INT`/`PRD`/`PRM` data types to [`IndexQuality`], and
/// ignores the `ISN` sunspot number column.
///
/// # Errors
///
/// [`SpaceWeatherError`] on missing header columns, malformed rows, unknown
/// `F10.7_DATA_TYPE`, invalid numbers/dates, or empty input.
pub fn parse_celestrak_csv(text: &str) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    let cleaned = clean_text(text);
    let mut lines = cleaned.lines().enumerate();
    let (hdr_idx, hdr_line) = lines
        .by_ref()
        .find(|(_, l)| !l.trim().is_empty())
        .ok_or(SpaceWeatherError::at_query(SpaceWeatherErrorKind::Empty))?;
    let cols = CsvColumns::from_header(hdr_line, hdr_idx.wrapping_add(1))?;

    let mut days = Vec::new();
    for (idx, raw_line) in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        days.push(parse_csv_row(line, idx.wrapping_add(1), &cols)?);
    }
    if days.is_empty() {
        return Err(SpaceWeatherError::at_query(SpaceWeatherErrorKind::Empty));
    }
    SpaceWeatherTable::from_days(days)
}

fn parse_csv_row(
    line: &str,
    line_num: usize,
    cols: &CsvColumns,
) -> Result<SpaceWeatherDay, SpaceWeatherError> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() != cols.total {
        return Err(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("CSV column count does not match header"),
        ));
    }
    let get = |idx: usize| {
        fields.get(idx).copied().ok_or(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("missing CSV field"),
        ))
    };
    let date = parse_iso_date(get(cols.date)?, line_num)?;
    let mut ap = [None; 8];
    for (dst, &col_idx) in ap.iter_mut().zip(cols.ap.iter()) {
        *dst = parse_optional_ap(get(col_idx)?, line_num, "ap", None)?;
    }
    let daily_ap = parse_optional_ap(get(cols.ap_avg)?, line_num, "daily_ap", None)?;
    let f107_observed = parse_optional_flux(get(cols.f107_obs)?, line_num, "f107_observed", &[])?;
    let f107_adjusted = parse_optional_flux(get(cols.f107_adj)?, line_num, "f107_adjusted", &[])?;
    let f107_observed_center81 = parse_optional_flux(
        get(cols.f107_obs_center81)?,
        line_num,
        "f107_observed_center81",
        &[],
    )?;
    let f107_adjusted_center81 = parse_optional_flux(
        get(cols.f107_adj_center81)?,
        line_num,
        "f107_adjusted_center81",
        &[],
    )?;
    let quality = match get(cols.f107_data_type)?.trim() {
        "OBS" => IndexQuality::Definitive,
        "INT" => IndexQuality::Interpolated,
        "PRD" => IndexQuality::Predicted,
        "PRM" => IndexQuality::MonthlyPredicted,
        _ => {
            return Err(SpaceWeatherError::at_line(
                line_num,
                SpaceWeatherErrorKind::InvalidQuality,
            ));
        }
    };
    Ok(SpaceWeatherDay {
        date,
        ap,
        daily_ap,
        f107_observed,
        f107_adjusted,
        f107_observed_center81,
        f107_adjusted_center81,
        quality,
        source: SpaceWeatherSource::CelestrakCsv,
    })
}
