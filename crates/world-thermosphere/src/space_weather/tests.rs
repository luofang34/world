use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::error::Error as _;

use world_core::{GeodeticPosition, WorldInstant};

use crate::{GeomagneticActivity, ThermosphereError};

use super::*;

fn synthetic_day(date: CalendarDate, ap_val: f64, daily_ap: f64, f107: f64) -> SpaceWeatherDay {
    SpaceWeatherDay {
        date,
        ap: [Some(ap_val); 8],
        daily_ap: Some(daily_ap),
        f107_observed: Some(f107),
        f107_adjusted: Some(f107),
        f107_observed_center81: None,
        f107_adjusted_center81: None,
        quality: IndexQuality::Definitive,
        source: SpaceWeatherSource::Gfz,
    }
}

#[test]
fn calendar_leap_years_and_day_of_year() {
    assert!(is_leap_year(2000));
    assert!(!is_leap_year(1900));
    assert!(is_leap_year(2024));
    assert!(!is_leap_year(2025));

    assert_eq!(day_of_year(2024, 1, 1).unwrap(), 1);
    assert_eq!(day_of_year(2024, 2, 29).unwrap(), 60);
    assert_eq!(day_of_year(2024, 3, 1).unwrap(), 61);
    assert_eq!(day_of_year(2024, 12, 31).unwrap(), 366);
    assert_eq!(day_of_year(2025, 3, 1).unwrap(), 60);
    assert_eq!(day_of_year(2025, 12, 31).unwrap(), 365);

    assert_eq!(
        CalendarDate::new(2025, 2, 29).unwrap_err().kind,
        SpaceWeatherErrorKind::InvalidDate {
            year: 2025,
            month: 2,
            day: 29,
        }
    );
    let d = CalendarDate::new(2024, 3, 1).unwrap();
    assert_eq!(
        d.add_days(-1).unwrap(),
        CalendarDate::new(2024, 2, 29).unwrap()
    );
    let (u_date, u_sec) = CalendarDate::from_unix_seconds(0).unwrap();
    assert_eq!(u_date, CalendarDate::new(1970, 1, 1).unwrap());
    assert_eq!(u_sec, 0.0);
    assert_eq!(
        CalendarDate::from_days_since_unix_epoch(d.days_since_unix_epoch()).unwrap(),
        d
    );
}

#[test]
fn gfz_parses_sentinels_ignores_sn_and_checks_quality() {
    let gfz = "\
# comment
1947 02 14  5523  5523.5 1556 25  0.0 0.0 1.7 1.3 1.0 1.3 1.7 1.0   -1    0    6    5    4    5    6    4    -1 999     -1.0    254.0 0
2024 05 10 33733 33733.5 2601 20  2.7 2.7 2.3 2.0 3.7 7.7 8.7 8.7   12   12    9    7   22  179  300  300   105 172    223.4    227.9 2
";
    let table = parse_gfz(gfz).unwrap();
    assert_eq!(table.len(), 2);
    let early = table.get(CalendarDate::new(1947, 2, 14).unwrap()).unwrap();
    assert_eq!(early.ap[0], None);
    assert_eq!(early.ap[1], Some(0.0));
    assert_eq!(early.daily_ap, None);
    assert_eq!(early.f107_observed, None);
    assert_eq!(early.f107_adjusted, Some(254.0));
    assert_eq!(early.quality, IndexQuality::Provisional);

    let storm = table.get(CalendarDate::new(2024, 5, 10).unwrap()).unwrap();
    assert_eq!(storm.ap[6], Some(300.0));
    assert_eq!(storm.daily_ap, Some(105.0));
    assert_eq!(storm.f107_observed, Some(223.4));
    assert_eq!(storm.quality, IndexQuality::Definitive);

    let bad_d = "2024 05 10 1 1.5 1 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 100.0 100.0 9\n";
    let err = parse_gfz(bad_d).unwrap_err();
    assert_eq!(err.line, 1);
    assert_eq!(err.kind, SpaceWeatherErrorKind::InvalidQuality);
}

#[test]
fn noaa_dsd_parses_flux_and_missing_sentinels() {
    let dsd = "\
:Product: Daily Solar Data            DSD.txt
# comment
2026 11 21   99     39      230      0    -999      *   0  0  0  0  0  0  0
2026 11 22  -999    23      160      0    -999      *   0  0  0  1  0  0  0
2026 11 23   -1     23      150      0    -999      *   0  0  0  0  0  0  0
";
    let table = parse_noaa_dsd(dsd).unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(
        table
            .get(CalendarDate::new(2026, 11, 21).unwrap())
            .unwrap()
            .f107_observed,
        Some(99.0)
    );
    assert_eq!(
        table
            .get(CalendarDate::new(2026, 11, 22).unwrap())
            .unwrap()
            .f107_observed,
        None
    );
    assert_eq!(
        table
            .get(CalendarDate::new(2026, 11, 23).unwrap())
            .unwrap()
            .ap,
        [None; 8]
    );
}

#[test]
fn noaa_f107_json_handles_escapes_whitespace_nulls_and_noon_preference() {
    let json = r#"[
      {
        "time_tag": "2026-11-20T17:00:00",
        "frequency": 2.8e3,
        "flux": 1.11e+02,
        "reporting_schedule": "Morning",
        "extra": {"nested": [true, false, null, "hi\u0021\n\"\\"]},
        "ninety_day_mean": null
      },
      {
        "time_tag": "2026-11-20T20:00:00",
        "frequency": 2800,
        "flux": 1.030000000000000e+002,
        "reporting_schedule": "Noon",
        "ninety_day_mean": 1.11e+02
      },
      {
        "time_tag": "2026-11-20T22:00:00",
        "frequency": 2800,
        "flux": 102.0,
        "reporting_schedule": "Afternoon",
        "ninety_day_mean": null
      },
      {
        "time_tag": "2026-11-21T17:00:00",
        "frequency": 2800,
        "flux": 100.0,
        "reporting_schedule": "Morning"
      },
      {
        "time_tag": "2026-11-21T22:00:00",
        "frequency": 2800,
        "flux": 104.0,
        "reporting_schedule": "Afternoon"
      }
    ]"#;
    let entries = parse_noaa_f107_entries(json).unwrap();
    assert_eq!(entries.len(), 5);
    let table = parse_noaa_f107_json(json).unwrap();
    let d20 = table.get(CalendarDate::new(2026, 11, 20).unwrap()).unwrap();
    assert_eq!(d20.f107_observed, Some(103.0));
    assert_eq!(d20.f107_observed_center81, None);
    let d21 = table.get(CalendarDate::new(2026, 11, 21).unwrap()).unwrap();
    assert_eq!(d21.f107_observed, Some(102.0));

    let bad = r#"[{"time_tag":"2026-11-20T00:00:00","flux":not_json}]"#;
    assert_eq!(parse_noaa_f107_json(bad).unwrap_err().line, 1);
    assert_eq!(
        parse_noaa_f107_json("[]").unwrap_err().kind,
        SpaceWeatherErrorKind::Empty
    );
}

#[test]
fn synthetic_celestrak_csv_handles_crlf_missing_ap_and_qualities() {
    let csv = concat!(
        "DATE,BSRN,ND,KP1,KP2,KP3,KP4,KP5,KP6,KP7,KP8,KP_SUM,AP1,AP2,AP3,AP4,AP5,AP6,AP7,AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,F10.7_OBS_CENTER81,F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81\r\n",
        "2025-01-01,2610,1,3,0,0,3,7,10,13,17,53,2,0,0,2,3,4,5,6,3,0.1,0,999,150.5,145.7,OBS,160.2,162.0,155.1,157.0\r\n",
        "2025-01-02,2610,2,10,10,10,10,10,10,10,10,80,4,4,4,4,4,4,4,4,4,0.1,0,111,152.0,147.2,INT,160.5,162.1,155.4,157.1\r\n",
        "2025-01-03,2610,3,13,13,13,13,13,13,13,13,104,5,5,5,5,5,5,5,5,5,0.2,1,70,148.0,143.3,PRD,160.1,161.9,155.0,156.9\r\n",
        "2025-02-01,2611,5,,,,,,,,,,,,,,,,,,,,,50,140.0,136.0,PRM,142.0,145.0,138.0,141.0\r\n",
    );
    let table = parse_celestrak_csv(csv).unwrap();
    assert_eq!(table.len(), 4);
    let d1 = table.get(CalendarDate::new(2025, 1, 1).unwrap()).unwrap();
    assert_eq!(d1.quality, IndexQuality::Definitive);
    assert_eq!(d1.f107_observed, Some(150.5));
    assert_eq!(d1.f107_observed_center81, Some(160.2));
    assert_eq!(
        table
            .get(CalendarDate::new(2025, 1, 2).unwrap())
            .unwrap()
            .quality,
        IndexQuality::Interpolated
    );
    assert_eq!(
        table
            .get(CalendarDate::new(2025, 1, 3).unwrap())
            .unwrap()
            .quality,
        IndexQuality::Predicted
    );
    let prm = table.get(CalendarDate::new(2025, 2, 1).unwrap()).unwrap();
    assert_eq!(prm.quality, IndexQuality::MonthlyPredicted);
    assert_eq!(prm.ap, [None; 8]);
    assert_eq!(prm.daily_ap, None);

    let bad_hdr = csv.replace("AP_AVG", "MISSING");
    assert_eq!(
        parse_celestrak_csv(&bad_hdr).unwrap_err().kind,
        SpaceWeatherErrorKind::MissingHeader("AP_AVG")
    );
}

#[test]
fn synthetic_celestrak_legacy_handles_fixed_variable_spacing_and_monthly() {
    let legacy = "\
DATATYPE CssiSpaceWeather\r
VERSION 1.2\r
BEGIN OBSERVED\r
2025 01 01 2610  1  3  0  0  3  7 10 13 17  53   2   0   0   2   3   4   5   6   3 0.1 0 120 145.7 0 155.1 157.0 150.5 160.2 162.0\r
2025 1 2 2610 2 10 10 10 10 10 10 10 10 80 4 4 4 4 4 4 4 4 4 0.1 0 99 147.2 4 155.4 157.1 152.0\r
END OBSERVED\r
BEGIN DAILY_PREDICTED\r
2025 01 03 2610  3 13 13 13 13 13 13 13 13 104   5   5   5   5   5   5   5   5   5 0.2 1  70 143.3   155.0 156.9 148.0 160.1 161.9\r
END DAILY_PREDICTED\r
BEGIN MONTHLY_PREDICTED\r
2025 02 01 2611  5                                                                        50 136.0   138.0 141.0 140.0 142.0 145.0\r
END MONTHLY_PREDICTED\r
";
    let daily_only = parse_celestrak_legacy(legacy).unwrap();
    assert_eq!(daily_only.len(), 3);
    let d1 = daily_only
        .get(CalendarDate::new(2025, 1, 1).unwrap())
        .unwrap();
    assert_eq!(d1.f107_observed, Some(150.5));
    assert_eq!(d1.f107_observed_center81, Some(160.2));
    assert_eq!(d1.quality, IndexQuality::Definitive);

    let d2 = daily_only
        .get(CalendarDate::new(2025, 1, 2).unwrap())
        .unwrap();
    assert_eq!(d2.quality, IndexQuality::Interpolated);
    assert_eq!(d2.f107_observed, Some(152.0));
    assert_eq!(d2.f107_observed_center81, None);

    let d3 = daily_only
        .get(CalendarDate::new(2025, 1, 3).unwrap())
        .unwrap();
    assert_eq!(d3.quality, IndexQuality::Predicted);
    assert_eq!(d3.ap[0], Some(5.0));

    let with_monthly = parse_celestrak_legacy_with_monthly(legacy, true).unwrap();
    assert_eq!(with_monthly.len(), 4);
    let m1 = with_monthly
        .get(CalendarDate::new(2025, 2, 1).unwrap())
        .unwrap();
    assert_eq!(m1.quality, IndexQuality::MonthlyPredicted);
    assert_eq!(m1.ap, [None; 8]);
}

#[test]
fn ap_history_crosses_utc_midnight_and_81_day_mean_is_exact() {
    let start = CalendarDate::new(2024, 2, 1).unwrap();
    let mut days = Vec::new();
    for offset in 0..90_i64 {
        let d = start.add_days(offset).unwrap();
        let mut day = synthetic_day(
            d,
            (offset as f64) + 1.0,
            20.0 + offset as f64,
            100.0 + offset as f64,
        );
        // Vary individual 3-hour buckets so midnight indexing is strictly distinguished.
        for (b, slot) in day.ap.iter_mut().enumerate() {
            *slot = Some((offset * 10 + b as i64) as f64);
        }
        days.push(day);
    }
    let table = SpaceWeatherTable::from_days(days).unwrap();
    // 2024-03-20 is offset 48 (2024 is leap year: Feb has 29 days, + 19 = 48).
    let target = CalendarDate::new(2024, 3, 20).unwrap();
    assert_eq!(
        target.days_since_unix_epoch() - start.days_since_unix_epoch(),
        48
    );
    // Test UT 01:30 -> bucket 0 (00-03 UT), which wraps immediately across midnight into offset 47!
    let act = table.activity_at(target, 5_400.0).unwrap();
    let GeomagneticActivity::History(h) = act.geomagnetic else {
        panic!("expected ApHistory");
    };
    assert_eq!(h.daily, 68.0);
    assert_eq!(h.current, 480.0);
    assert_eq!(h.hours_3_before, 477.0);
    assert_eq!(h.hours_6_before, 476.0);
    assert_eq!(h.hours_9_before, 475.0);
    let m12_33: f64 = [474.0, 473.0, 472.0, 471.0, 470.0, 467.0, 466.0, 465.0]
        .iter()
        .sum::<f64>()
        / 8.0;
    let m36_57: f64 = [464.0, 463.0, 462.0, 461.0, 460.0, 457.0, 456.0, 455.0]
        .iter()
        .sum::<f64>()
        / 8.0;
    assert_eq!(h.mean_12_to_33_hours_before, m12_33);
    assert_eq!(h.mean_36_to_57_hours_before, m36_57);
    assert_eq!(act.f107_previous_day, 147.0);
    // Symmetric [-40..=+40] centered on 100 + 48 = 148.0 has exact mean 148.0!
    assert!((act.f107_81_day_mean - 148.0).abs() < 1e-12);

    // Check WorldInstant integration
    let pos = GeodeticPosition::wgs84(45.0, 10.0, 400_000.0);
    let secs = target.days_since_unix_epoch() * 86_400 + 5_400;
    let inst = WorldInstant::new(0, secs * 1_000_000);
    let msis = table.msis_input_at_instant(pos, inst).unwrap();
    assert_eq!(msis.day_of_year, 80);
    assert_eq!(msis.ut_seconds, 5_400.0);
}

#[test]
fn table_merge_prefers_higher_quality_and_fills_gaps() {
    let d1 = CalendarDate::new(2026, 5, 9).unwrap();
    let mut gfz_day = synthetic_day(d1, 15.0, 15.0, 120.0);
    gfz_day.f107_observed = None;
    let noaa_day = SpaceWeatherDay {
        date: d1,
        ap: [None; 8],
        daily_ap: None,
        f107_observed: Some(122.6),
        f107_adjusted: None,
        f107_observed_center81: None,
        f107_adjusted_center81: None,
        quality: IndexQuality::Provisional,
        source: SpaceWeatherSource::NoaaDailySolarData,
    };
    let mut t1 = SpaceWeatherTable::from_days([gfz_day]).unwrap();
    let t2 = SpaceWeatherTable::from_days([noaa_day]).unwrap();
    t1.merge(&t2);
    let merged = t1.get(d1).unwrap();
    assert_eq!(merged.ap[0], Some(15.0));
    assert_eq!(merged.f107_observed, Some(122.6));
    assert_eq!(merged.quality, IndexQuality::Definitive);
    assert_eq!(merged.source, SpaceWeatherSource::Merged);
}

#[test]
fn missing_history_and_invalid_time_return_typed_errors_preserving_source() {
    let d = CalendarDate::new(2024, 5, 10).unwrap();
    let table = SpaceWeatherTable::from_days([synthetic_day(d, 10.0, 10.0, 150.0)]).unwrap();
    assert_eq!(
        table.ap_history_at(d, 86_400.0).unwrap_err().kind,
        SpaceWeatherErrorKind::InvalidTime {
            ut_seconds: 86_400.0
        }
    );
    assert_eq!(
        table.ap_history_at(d, 0.0).unwrap_err().kind,
        SpaceWeatherErrorKind::MissingDate {
            date: d.add_days(-1).unwrap()
        }
    );
    let err = SpaceWeatherError::from(ThermosphereError::NotFinite);
    assert!(err.source().is_some());
    assert!(!format!("{err}").is_empty());
    assert!(!SpaceWeatherSource::Gfz.attribution().is_empty());
    assert!(!GFZ_ATTRIBUTION.to_string().is_empty());
}

#[test]
fn json_unicode_surrogate_pairs_and_empty_table_and_validation_work() {
    let json = r#"[{"time_tag":"2026-11-23T17:00:00Z","frequency":2800,"flux":95.5,"reporting_schedule":"Morning","note":"\uD83D\uDE80"}]"#;
    let table = parse_noaa_f107_json(json).unwrap();
    let d = CalendarDate::new(2026, 11, 23).unwrap();
    assert_eq!(table.get(d).unwrap().f107_observed, Some(95.5));

    let empty = SpaceWeatherTable::new();
    assert!(empty.is_empty());
    assert_eq!(empty.date_range(), None);

    let mut bad_day = synthetic_day(d, 4.0, 4.0, 100.0);
    bad_day.ap[3] = Some(-1.0);
    assert_eq!(
        SpaceWeatherTable::from_days([bad_day]).unwrap_err().kind,
        SpaceWeatherErrorKind::InvalidValue {
            field: "ap",
            value: -1.0,
        }
    );

    let unclosed = "BEGIN OBSERVED\n2025 01 01 2610 1 3 0 0 3 7 10 13 17 53 2 0 0 2 3 4 5 6 3 0.1 0 1 145.7 0 155.1 157.0 150.5 160.2 162.0\n";
    assert_eq!(
        parse_celestrak_legacy(unclosed).unwrap_err().kind,
        SpaceWeatherErrorKind::UnclosedSection("OBSERVED")
    );
}
