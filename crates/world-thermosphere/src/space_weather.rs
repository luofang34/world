//! Space-weather index readers and NRLMSISE-00 activity lookup tables.
//!
//! Available when the `alloc` feature is enabled. Every parser operates on
//! in-memory `&str` text, performs no filesystem or network I/O, adds zero
//! third-party dependencies, and excludes sunspot number (`SN`/`ISN`) columns.
//!
//! Supported formats:
//! - GFZ `Kp_ap_Ap_SN_F107_since_1932.txt` ([`parse_gfz`]; CC BY 4.0 excluding
//!   CC BY-NC 4.0 `SN`, see [`GFZ_ATTRIBUTION`])
//! - NOAA SWPC `daily-solar-indices.txt` ([`parse_noaa_dsd`]; public domain)
//! - NOAA SWPC `f107_cm_flux.json` ([`parse_noaa_f107_json`],
//!   [`parse_noaa_f107_entries`]; public domain)
//! - CelesTrak CSV `SW-Last5Years.csv` / `SW-All.csv` ([`parse_celestrak_csv`])
//! - CelesTrak legacy CSSI `SW-All.txt` ([`parse_celestrak_legacy`],
//!   [`parse_celestrak_legacy_with_monthly`])

mod calendar;
mod celestrak;
mod error;
mod fields;
mod gfz;
mod noaa;
mod record;
mod table;

#[cfg(test)]
mod tests;

pub use calendar::{CalendarDate, day_of_year, is_leap_year};
pub use celestrak::{
    CELESTRAK_CSV_PROVENANCE, CELESTRAK_LEGACY_PROVENANCE, parse_celestrak_csv,
    parse_celestrak_legacy, parse_celestrak_legacy_with_monthly,
};
pub use error::{SpaceWeatherError, SpaceWeatherErrorKind};
pub use gfz::{GFZ_ATTRIBUTION, GFZ_PROVENANCE, parse_gfz};
pub use noaa::{
    NOAA_DSD_PROVENANCE, NOAA_F107_PROVENANCE, NoaaF107Entry, ReportingSchedule, parse_noaa_dsd,
    parse_noaa_f107_entries, parse_noaa_f107_json,
};
pub use record::{
    IndexQuality, SolarFluxInputs, SpaceWeatherActivity, SpaceWeatherDay, SpaceWeatherSource,
};
pub use table::SpaceWeatherTable;
