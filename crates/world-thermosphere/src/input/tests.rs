use world_core::GeodeticPosition;

use super::*;

fn input() -> MsisInput {
    MsisInput {
        position: GeodeticPosition::wgs84(60.0, -70.0, 400_000.0),
        day_of_year: 172,
        ut_seconds: 29_000.0,
        local_solar_time_h: 16.0,
        f107_previous_day: 150.0,
        f107_81_day_mean: 150.0,
        geomagnetic: GeomagneticActivity::DailyAp(4.0),
    }
}

fn refused(input: MsisInput) -> &'static str {
    match input.validate() {
        Err(ThermosphereError::InvalidInput { quantity, .. }) => quantity,
        other => panic!("expected an invalid input, got {other:?}"),
    }
}

#[test]
fn a_well_formed_input_is_accepted() {
    assert_eq!(input().validate(), Ok(()));
}

#[test]
fn each_malformed_field_is_named() {
    let mut bad = input();
    bad.position.latitude_deg = 90.5;
    assert_eq!(refused(bad), "position.latitude_deg");
    let mut bad = input();
    bad.position.longitude_deg = f64::NAN;
    assert_eq!(refused(bad), "position.longitude_deg");
    let mut bad = input();
    bad.day_of_year = 0;
    assert_eq!(refused(bad), "day_of_year");
    let mut bad = input();
    bad.day_of_year = 367;
    assert_eq!(refused(bad), "day_of_year");
    let mut bad = input();
    bad.ut_seconds = f64::INFINITY;
    assert_eq!(refused(bad), "ut_seconds");
    let mut bad = input();
    bad.f107_previous_day = 0.0;
    assert_eq!(refused(bad), "f107_previous_day");
    let mut bad = input();
    bad.f107_81_day_mean = f64::NAN;
    assert_eq!(refused(bad), "f107_81_day_mean");
    let mut bad = input();
    bad.geomagnetic = GeomagneticActivity::DailyAp(-1.0);
    assert_eq!(refused(bad), "geomagnetic");
}

#[test]
fn a_malformed_history_entry_is_refused() {
    let mut history = ApHistory {
        daily: 4.0,
        current: 4.0,
        hours_3_before: 4.0,
        hours_6_before: 4.0,
        hours_9_before: 4.0,
        mean_12_to_33_hours_before: 4.0,
        mean_36_to_57_hours_before: 4.0,
    };
    history.mean_36_to_57_hours_before = f64::NAN;
    let mut bad = input();
    bad.geomagnetic = GeomagneticActivity::History(history);
    assert_eq!(refused(bad), "geomagnetic");
}

#[test]
fn the_apparent_local_time_follows_the_reference_formula() {
    assert_eq!(MsisInput::apparent_local_solar_time_h(43_200.0, -90.0), 6.0);
}
