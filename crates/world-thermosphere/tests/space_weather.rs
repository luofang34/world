//! Integration tests for `world_thermosphere::space_weather`: committed
//! GFZ (CC BY 4.0, SN blanked to -1) and NOAA SWPC (public domain) excerpts,
//! NRLMSISE-00 quiet-vs-storm evaluations, and ignored full-file checks
//! against user-supplied local datasets.

#![cfg(feature = "alloc")]

use std::env;
use std::fs;
use std::path::PathBuf;

use world_core::GeodeticPosition;
use world_thermosphere::space_weather::{
    CalendarDate, GFZ_ATTRIBUTION, IndexQuality, SpaceWeatherDay, SpaceWeatherErrorKind,
    SpaceWeatherSource, SpaceWeatherTable, parse_celestrak_csv, parse_celestrak_legacy,
    parse_celestrak_legacy_with_monthly, parse_gfz, parse_noaa_dsd, parse_noaa_f107_entries,
    parse_noaa_f107_json,
};
use world_thermosphere::{GeomagneticActivity, Nrlmsise00};

/// Excerpt of GFZ Potsdam `Kp_ap_Ap_SN_F107_since_1932.txt` (2024-03-28 through
/// 2024-06-25, May 2024 Gannon storm), CC BY 4.0, Matzka et al. (2021)
/// doi:10.1029/2020SW002641 / doi:10.5880/Kp.0001; CC BY-NC 4.0 SN column
/// replaced with `-1` on every row.
const GFZ_GANNON_EXCERPT: &str = include_str!("data/gfz_gannon_2024.txt");

/// Excerpt of NOAA SWPC `daily-solar-indices.txt` (`DSD.txt`, U.S. Government
/// public domain per weather.gov/disclaimer).
const NOAA_DSD_EXCERPT: &str = include_str!("data/noaa_dsd_excerpt.txt");

/// Excerpt of NOAA SWPC `f107_cm_flux.json` (U.S. Government public domain per
/// weather.gov/disclaimer).
const NOAA_F107_EXCERPT: &str = include_str!("data/noaa_f107_excerpt.json");

#[test]
fn gfz_gannon_storm_drives_nrlmsise00_across_utc_midnight() {
    assert!(GFZ_ATTRIBUTION.contains("Matzka"));
    assert!(GFZ_ATTRIBUTION.contains("CC BY 4.0"));

    let table = parse_gfz(GFZ_GANNON_EXCERPT).unwrap();
    assert_eq!(table.len(), 90);
    assert_eq!(
        table.date_range(),
        Some((
            CalendarDate::new(2024, 3, 28).unwrap(),
            CalendarDate::new(2024, 6, 25).unwrap(),
        ))
    );

    let pos = GeodeticPosition::wgs84(45.0, -75.0, 400_000.0);
    // Quiet period before storm onset: 2024-05-09 06:00 UT (ap = 9, 18, 12, ...).
    let quiet_date = CalendarDate::new(2024, 5, 9).unwrap();
    let quiet_input = table.msis_input_at(pos, quiet_date, 21_600.0).unwrap();
    let quiet_state = Nrlmsise00::STANDARD.evaluate(&quiet_input).unwrap();

    // Storm peak crossing UTC midnight: 2024-05-11 00:30 UT (ap=400, preceded by 300, 300, 179 on May 10).
    let storm_date = CalendarDate::new(2024, 5, 11).unwrap();
    let storm_input = table.msis_input_at(pos, storm_date, 1_800.0).unwrap();
    let GeomagneticActivity::History(h) = storm_input.geomagnetic else {
        panic!("expected ApHistory");
    };
    assert_eq!(h.daily, 271.0);
    assert_eq!(h.current, 400.0);
    assert_eq!(h.hours_3_before, 300.0);
    assert_eq!(h.hours_6_before, 300.0);
    assert_eq!(h.hours_9_before, 179.0);
    assert_eq!(storm_input.f107_previous_day, 223.4);

    // Check 81-day centered mean matches exact arithmetic mean over 2024-04-01..=2024-06-20.
    let expected_mean: f64 = (-40_i64..=40_i64)
        .map(|off| {
            table
                .get(storm_date.add_days(off).unwrap())
                .unwrap()
                .f107_observed
                .unwrap()
        })
        .sum::<f64>()
        / 81.0;
    assert!((storm_input.f107_81_day_mean - expected_mean).abs() < 1e-12);

    let storm_state = Nrlmsise00::STANDARD.evaluate(&storm_input).unwrap();
    assert!(storm_state.exospheric_temperature_k > quiet_state.exospheric_temperature_k + 300.0);
    assert!(storm_state.mass_density_kg_m3 > quiet_state.mass_density_kg_m3 * 2.0);
}

#[test]
fn noaa_excerpts_agree_on_noon_flux_and_handle_partial_days() {
    let dsd = parse_noaa_dsd(NOAA_DSD_EXCERPT).unwrap();
    assert_eq!(dsd.len(), 15);
    assert_eq!(
        SpaceWeatherSource::NoaaDailySolarData
            .provenance()
            .publisher,
        "U.S. Dept. of Commerce, NOAA, Space Weather Prediction Center"
    );

    let entries = parse_noaa_f107_entries(NOAA_F107_EXCERPT).unwrap();
    assert_eq!(entries.len(), 23);
    let json_table = parse_noaa_f107_json(NOAA_F107_EXCERPT).unwrap();
    assert_eq!(json_table.len(), 8);

    // 2026-10-22 has Noon + Morning (no Afternoon), Noon = 106.0.
    assert_eq!(
        json_table
            .get(CalendarDate::new(2026, 10, 22).unwrap())
            .unwrap()
            .f107_observed,
        Some(106.0)
    );
    // Overlapping days 2026-11-20..2026-11-23 match DSD exactly.
    for day_num in 20..=23 {
        let d = CalendarDate::new(2026, 11, day_num).unwrap();
        assert_eq!(
            dsd.get(d).unwrap().f107_observed,
            json_table.get(d).unwrap().f107_observed
        );
    }
    // Missing ap prevents ApHistory synthesis.
    let err = json_table
        .activity_at(CalendarDate::new(2026, 11, 23).unwrap(), 43_200.0)
        .unwrap_err();
    assert_eq!(
        err.kind,
        SpaceWeatherErrorKind::MissingIndex {
            date: CalendarDate::new(2026, 11, 23).unwrap(),
            index: "daily_ap",
        }
    );
}

#[test]
fn celestrak_synthetic_table_drives_nrlmsise00_using_supplied_center81() {
    let hdr = "DATE,BSRN,ND,KP1,KP2,KP3,KP4,KP5,KP6,KP7,KP8,KP_SUM,AP1,AP2,AP3,AP4,AP5,AP6,AP7,AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,F10.7_OBS_CENTER81,F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81\r\n";
    let mut csv = String::from(hdr);
    for day in 1..=5_u8 {
        csv.push_str(&format!(
            "2026-01-{day:02},2624,{day},10,10,10,10,10,10,10,10,80,4,5,6,7,8,9,10,11,8,0.4,2,42,{:.1},140.0,OBS,155.0,152.0,150.0,148.0\r\n",
            140.0 + f64::from(day)
        ));
    }
    let table = parse_celestrak_csv(&csv).unwrap();
    assert_eq!(table.len(), 5);
    let d = CalendarDate::new(2026, 1, 5).unwrap();
    // Even with only 5 days in the table, `F10.7_OBS_CENTER81` enables lookup on day 5.
    let input = table
        .msis_input_at(GeodeticPosition::wgs84(0.0, 0.0, 350_000.0), d, 0.0)
        .unwrap();
    assert_eq!(input.f107_previous_day, 144.0);
    assert_eq!(input.f107_81_day_mean, 155.0);
    let state = Nrlmsise00::STANDARD.evaluate(&input).unwrap();
    assert!(state.mass_density_kg_m3 > 0.0);
}

fn resolve_local_sw_path(env_var: &str, relative_under_handoff: &str) -> PathBuf {
    if let Some(p) = env::var_os(env_var) {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("..")
        .join("..")
        .join("basis-handoff")
        .join("sw")
        .join(relative_under_handoff)
}

#[test]
#[ignore = "reads external full space-weather files from WORLD_SW_* or ../basis-handoff/sw"]
fn local_full_gfz_parses_and_evaluates_historical_storms() {
    let path = resolve_local_sw_path("WORLD_SW_GFZ", "gfz/Kp_ap_Ap_SN_F107_since_1932.txt");
    let text = fs::read_to_string(&path).expect("read GFZ file");
    let table = parse_gfz(&text).expect("parse full GFZ");
    assert!(table.len() >= 34_000, "got {} GFZ rows", table.len());

    let pos = GeodeticPosition::wgs84(40.0, 0.0, 400_000.0);
    // Halloween storm 2003-10-30 00:30 UT
    let halloween = table
        .msis_input_at(pos, CalendarDate::new(2003, 10, 30).unwrap(), 1_800.0)
        .expect("Halloween storm input");
    let state = Nrlmsise00::STANDARD.evaluate(&halloween).expect("evaluate");
    assert!(state.exospheric_temperature_k > 1_300.0);
}

#[test]
#[ignore = "reads external full CelesTrak CSV from WORLD_SW_CELESTRAK_CSV or ../basis-handoff/sw"]
fn local_full_celestrak_csv_parses_and_agrees_with_computed_81d_mean() {
    let path = resolve_local_sw_path("WORLD_SW_CELESTRAK_CSV", "celestrak/SW-Last5Years.csv");
    let text = fs::read_to_string(&path).expect("read CelesTrak CSV");
    let table = parse_celestrak_csv(&text).expect("parse CelesTrak CSV");
    assert!(table.len() >= 2_000, "got {} CSV rows", table.len());

    // Compare CelesTrak's F10.7_OBS_CENTER81 against an explicit 81-day mean on an observed day.
    let probe = CalendarDate::new(2024, 5, 11).unwrap();
    let row = table.get(probe).unwrap();
    let csv_c81 = row.f107_observed_center81.unwrap();
    let mut cleared: SpaceWeatherDay = *row;
    cleared.f107_observed_center81 = None;
    let mut stripped = SpaceWeatherTable::from_days(
        table
            .days()
            .iter()
            .map(|&d| if d.date == probe { cleared } else { d }),
    )
    .unwrap();
    let computed = stripped.f107_at(probe).unwrap().mean_81_day;
    assert!((csv_c81 - computed).abs() < 0.1, "{csv_c81} vs {computed}");
    stripped.merge(&table);
    assert_eq!(
        stripped.get(probe).unwrap().f107_observed_center81,
        Some(csv_c81)
    );
}

#[test]
#[ignore = "reads external full CelesTrak legacy file from WORLD_SW_CELESTRAK_LEGACY or ../basis-handoff/sw"]
fn local_full_celestrak_legacy_parses_observed_predicted_and_monthly() {
    let path = resolve_local_sw_path("WORLD_SW_CELESTRAK_LEGACY", "celestrak/SW-All.txt");
    let text = fs::read_to_string(&path).expect("read SW-All.txt");
    let daily = parse_celestrak_legacy(&text).expect("parse SW-All daily");
    assert!(daily.len() >= 25_000, "got {} daily rows", daily.len());
    let with_monthly = parse_celestrak_legacy_with_monthly(&text, true).expect("parse SW-All all");
    assert!(with_monthly.len() > daily.len());
    assert_eq!(
        with_monthly.days().last().unwrap().quality,
        IndexQuality::MonthlyPredicted
    );
}

#[test]
#[ignore = "reads external full NOAA files from WORLD_SW_NOAA_* or ../basis-handoff/sw"]
fn local_full_noaa_files_parse_cleanly() {
    let dsd_path = resolve_local_sw_path("WORLD_SW_NOAA_DSD", "noaa/daily-solar-indices.txt");
    let json_path = resolve_local_sw_path("WORLD_SW_NOAA_F107_JSON", "noaa/f107_cm_flux.json");
    let dsd = parse_noaa_dsd(&fs::read_to_string(dsd_path).unwrap()).unwrap();
    let json = parse_noaa_f107_json(&fs::read_to_string(json_path).unwrap()).unwrap();
    assert_eq!(dsd.len(), 30);
    assert!(json.len() >= 30);
    for day in dsd.days() {
        assert_eq!(
            day.f107_observed,
            json.get(day.date).and_then(|d| d.f107_observed)
        );
    }
}
