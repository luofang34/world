//! Reader for GFZ Helmholtz Centre for Geosciences `Kp_ap_Ap_SN_F107` files.

use alloc::vec::Vec;

use world_core::{DatasetId, Provenance};

use super::error::{SpaceWeatherError, SpaceWeatherErrorKind};
use super::fields::{clean_text, parse_optional_ap, parse_optional_flux, parse_ymd_tokens};
use super::record::{IndexQuality, SpaceWeatherDay, SpaceWeatherSource};
use super::table::SpaceWeatherTable;

/// Required attribution for GFZ Potsdam geomagnetic and DRAO solar flux data.
pub const GFZ_ATTRIBUTION: &str = "Matzka, J., Stolle, C., Yamazaki, Y., Bronkalla, O., & Morschhauser, A. (2021), The geomagnetic Kp index and derived indices of geomagnetic activity, Space Weather, doi:10.1029/2020SW002641; Geomagnetic Observatory Niemegk, GFZ Helmholtz Centre for Geosciences (https://doi.org/10.5880/Kp.0001), CC BY 4.0 (Kp, ap, Ap, F10.7; CC BY-NC 4.0 SN column excluded); F10.7 solar radio flux from DRAO / Natural Resources Canada (Tapping, 2013, doi:10.1002/swe.20064).";

/// Provenance metadata for the GFZ `Kp_ap_Ap_SN_F107_since_1932.txt` format.
pub const GFZ_PROVENANCE: Provenance = Provenance {
    dataset: DatasetId {
        name: "GFZ Kp_ap_Ap_SN_F107",
        edition: "Kp.0001 V1.0",
    },
    publisher: "Geomagnetic Observatory Niemegk, GFZ Helmholtz Centre for Geosciences",
    source_url: "https://kp.gfz.de/en/data",
    source_file: "Kp_ap_Ap_SN_F107_since_1932.txt",
    source_sha256: "dynamic (live feed; caller-supplied text)",
    terms: "CC BY 4.0 for Kp, ap, Ap, and F10.7 (cite Matzka et al., 2021); SN column is CC BY-NC 4.0 and is excluded by this reader",
};

/// Parses a GFZ `Kp_ap_Ap_SN_F107` text file into a [`SpaceWeatherTable`].
///
/// `#` header/comment lines and blank lines are skipped. Missing sentinels
/// (`-1` for `ap`/`Ap` and `-1.0` for `F10.7`) map to `None`. The `SN`
/// (International Sunspot Number, CC BY-NC 4.0) column is skipped without
/// parsing or storing its value.
///
/// # Errors
///
/// [`SpaceWeatherError`] naming the 1-based line of any malformed row, invalid
/// date, invalid `D` flag (`0`, `1`, or `2` expected), or an empty file.
pub fn parse_gfz(text: &str) -> Result<SpaceWeatherTable, SpaceWeatherError> {
    let cleaned = clean_text(text);
    let mut days = Vec::new();
    for (idx, raw_line) in cleaned.lines().enumerate() {
        let line_num = idx.wrapping_add(1);
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        days.push(parse_gfz_row(line, line_num)?);
    }
    if days.is_empty() {
        return Err(SpaceWeatherError::at_query(SpaceWeatherErrorKind::Empty));
    }
    SpaceWeatherTable::from_days(days)
}

fn parse_gfz_row(line: &str, line_num: usize) -> Result<SpaceWeatherDay, SpaceWeatherError> {
    let mut fields = line.split_whitespace();
    let mut next_tok = || {
        fields.next().ok_or(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("expected 28 GFZ columns"),
        ))
    };
    let date = parse_ymd_tokens(next_tok()?, next_tok()?, next_tok()?, line_num)?;
    // Skip `days`, `days_m`, `Bsr`, `dB`, and `Kp1..Kp8` (12 columns).
    for _ in 0..12 {
        let _ = next_tok()?;
    }
    let mut ap = [None; 8];
    for slot in &mut ap {
        *slot = parse_optional_ap(next_tok()?, line_num, "ap", Some(-1.0))?;
    }
    let daily_ap = parse_optional_ap(next_tok()?, line_num, "daily_ap", Some(-1.0))?;
    // Skip `SN` (CC BY-NC 4.0) without reading or retaining it.
    let _sn_ignored = next_tok()?;
    let f107_observed = parse_optional_flux(next_tok()?, line_num, "f107_observed", &[-1.0])?;
    let f107_adjusted = parse_optional_flux(next_tok()?, line_num, "f107_adjusted", &[-1.0])?;
    let quality = match next_tok()? {
        "0" => IndexQuality::Provisional,
        "1" | "2" => IndexQuality::Definitive,
        _ => {
            return Err(SpaceWeatherError::at_line(
                line_num,
                SpaceWeatherErrorKind::InvalidQuality,
            ));
        }
    };
    if fields.next().is_some() {
        return Err(SpaceWeatherError::at_line(
            line_num,
            SpaceWeatherErrorKind::Malformed("unexpected trailing GFZ columns"),
        ));
    }
    Ok(SpaceWeatherDay {
        date,
        ap,
        daily_ap,
        f107_observed,
        f107_adjusted,
        f107_observed_center81: None,
        f107_adjusted_center81: None,
        quality,
        source: SpaceWeatherSource::Gfz,
    })
}
