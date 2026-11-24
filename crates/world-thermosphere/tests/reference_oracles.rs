//! The model against values computed once by the two reference
//! implementations: the NRL Fortran original built in double precision,
//! and its C translation. The data files state how each was produced.

use world_core::GeodeticPosition;
use world_thermosphere::{
    ApHistory, Effect, GeomagneticActivity, MsisInput, Nrlmsise00, Switch, Switches,
    ThermosphereState,
};

const CASES: &str = include_str!("data/nrlmsise00_cases.txt");
const FORTRAN: &str = include_str!("data/nrlmsise00_fortran_r8.txt");
const C: &str = include_str!("data/nrlmsise00_c_bc9a2fe.txt");

/// The model and the references evaluate the same formulas in the same
/// order, in different mathematical libraries.
const TOLERANCE: f64 = 1e-12;

struct Case {
    altitude_km: f64,
    model: Nrlmsise00,
    input: MsisInput,
}

#[cfg(test)]
fn rows(text: &str) -> Vec<Vec<f64>> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            line.split_whitespace()
                .map(|field| field.parse().expect("a number"))
                .collect()
        })
        .collect()
}

#[cfg(test)]
fn case(fields: &[f64]) -> Case {
    let ap = &fields[9..16];
    let geomagnetic = if fields[8] == 1.0 {
        GeomagneticActivity::History(ApHistory {
            daily: ap[0],
            current: ap[1],
            hours_3_before: ap[2],
            hours_6_before: ap[3],
            hours_9_before: ap[4],
            mean_12_to_33_hours_before: ap[5],
            mean_36_to_57_hours_before: ap[6],
        })
    } else {
        GeomagneticActivity::DailyAp(ap[0])
    };
    let mut switches = Switches::STANDARD;
    for (effect, setting) in Effect::ALL.into_iter().zip(&fields[16..39]) {
        let switch = match *setting as u8 {
            0 => Switch::Off,
            1 => Switch::On,
            _ => Switch::CrossTermsOnly,
        };
        switches = switches.with(effect, switch);
    }
    Case {
        altitude_km: fields[2],
        model: Nrlmsise00::new(switches),
        input: MsisInput {
            position: GeodeticPosition::wgs84(fields[3], fields[4], fields[2] * 1000.0),
            day_of_year: fields[0] as u16,
            ut_seconds: fields[1],
            local_solar_time_h: fields[5],
            f107_81_day_mean: fields[6],
            f107_previous_day: fields[7],
            geomagnetic,
        },
    }
}

/// The state in the reference's order and units: `D(1..9)`, `T(1)`, `T(2)`
/// and the `GTD7D` `D(6)`.
fn in_reference_units(state: &ThermosphereState) -> [f64; 12] {
    let n = state.number_density;
    [
        n.helium_m3 / 1e6,
        n.atomic_oxygen_m3 / 1e6,
        n.molecular_nitrogen_m3 / 1e6,
        n.molecular_oxygen_m3 / 1e6,
        n.argon_m3 / 1e6,
        state.mass_density_kg_m3 / 1e3,
        n.atomic_hydrogen_m3 / 1e6,
        n.atomic_nitrogen_m3 / 1e6,
        n.anomalous_oxygen_m3 / 1e6,
        state.exospheric_temperature_k,
        state.temperature_k,
        state.drag_mass_density_kg_m3 / 1e3,
    ]
}

/// The largest relative difference from `oracle` over the cases `include`
/// keeps, and how many values were compared.
#[cfg(test)]
fn worst_difference(oracle: &str, include: impl Fn(&Case) -> bool) -> (f64, usize) {
    let cases = rows(CASES);
    let expected = rows(oracle);
    assert_eq!(cases.len(), expected.len());
    let mut worst = 0.0_f64;
    let mut compared = 0;
    for (fields, expected) in cases.iter().zip(&expected) {
        let case = case(fields);
        if !include(&case) {
            continue;
        }
        let state = case.model.evaluate(&case.input).expect("a state");
        for (ours, theirs) in in_reference_units(&state).iter().zip(expected) {
            let scale = ours.abs().max(theirs.abs());
            if scale > 0.0 {
                worst = worst.max((ours - theirs).abs() / scale);
            }
            compared += 1;
        }
    }
    (worst, compared)
}

#[test]
fn the_fortran_original_is_reproduced() {
    // At exactly 32.5 km the original answers NaN.
    let (worst, compared) = worst_difference(FORTRAN, |case| case.altitude_km != 32.5);
    assert_eq!(compared, (298 - 6) * 12);
    assert!(worst < TOLERANCE, "worst relative difference {worst:e}");
}

#[test]
fn the_fortran_original_has_no_answer_at_the_stratopause_node() {
    let nan_lines = rows(FORTRAN)
        .iter()
        .zip(rows(CASES))
        .filter(|(values, _)| values.iter().any(|v| v.is_nan()))
        .map(|(_, fields)| fields[2])
        .collect::<Vec<_>>();
    assert_eq!(nan_lines, [32.5; 6]);
}

#[test]
fn the_c_translation_is_reproduced() {
    // At exactly 200 km the translation leaves helium unmixed, which the
    // original does not.
    let (worst, compared) = worst_difference(C, |case| case.altitude_km != 200.0);
    assert_eq!(compared, (298 - 6) * 12);
    assert!(worst < TOLERANCE, "worst relative difference {worst:e}");
}

#[test]
fn the_c_translation_differs_only_in_helium_at_its_mixing_limit() {
    let cases = rows(CASES);
    let expected = rows(C);
    let mut seen = 0;
    for (fields, expected) in cases.iter().zip(&expected) {
        let case = case(fields);
        if case.altitude_km != 200.0 {
            continue;
        }
        seen += 1;
        let state = case.model.evaluate(&case.input).expect("a state");
        let ours = in_reference_units(&state);
        for index in [1, 2, 3, 4, 6, 7, 8, 9, 10] {
            let scale = ours[index].abs().max(expected[index].abs());
            assert!((ours[index] - expected[index]).abs() <= TOLERANCE * scale);
        }
        let helium = (ours[0] - expected[0]).abs() / expected[0];
        assert!(
            helium > 1e-4 && helium < 1e-2,
            "helium differs by {helium:e}"
        );
    }
    assert_eq!(seen, 6);
}
