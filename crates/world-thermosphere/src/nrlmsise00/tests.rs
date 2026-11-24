use std::format;
use std::string::String;
use std::vec::Vec;

use world_core::GeodeticPosition;

use crate::{ApHistory, Effect, GeomagneticActivity, MsisInput, Switch};

use super::*;

/// The output the reference distribution states its test driver prints
/// (section 7, "Testing Output", of the `DOCUMENTATION` file of the C
/// release at commit bc9a2fe of <https://git.linta.de/~brodo/nrlmsise-00.git>,
/// sha256 e7db72964c22861e446d4b61931a22a58c32eeaa7f9bb23b45d887751b4ecae3):
/// one row per test case, `D(1)` to `D(9)` then `T(1)` and `T(2)`, each
/// printed with C's `%E`.
const REFERENCE_OUTPUT: [[&str; 11]; 17] = [
    [
        "6.665177E+05",
        "1.138806E+08",
        "1.998211E+07",
        "4.022764E+05",
        "3.557465E+03",
        "4.074714E-15",
        "3.475312E+04",
        "4.095913E+06",
        "2.667273E+04",
        "1.250540E+03",
        "1.241416E+03",
    ],
    [
        "3.407293E+06",
        "1.586333E+08",
        "1.391117E+07",
        "3.262560E+05",
        "1.559618E+03",
        "5.001846E-15",
        "4.854208E+04",
        "4.380967E+06",
        "6.956682E+03",
        "1.166754E+03",
        "1.161710E+03",
    ],
    [
        "1.123767E+05",
        "6.934130E+04",
        "4.247105E+01",
        "1.322750E-01",
        "2.618848E-05",
        "2.756772E-18",
        "2.016750E+04",
        "5.741256E+03",
        "2.374394E+04",
        "1.239892E+03",
        "1.239891E+03",
    ],
    [
        "5.411554E+07",
        "1.918893E+11",
        "6.115826E+12",
        "1.225201E+12",
        "6.023212E+10",
        "3.584426E-10",
        "1.059880E+07",
        "2.615737E+05",
        "2.819879E-42",
        "1.027318E+03",
        "2.068878E+02",
    ],
    [
        "1.851122E+06",
        "1.476555E+08",
        "1.579356E+07",
        "2.633795E+05",
        "1.588781E+03",
        "4.809630E-15",
        "5.816167E+04",
        "5.478984E+06",
        "1.264446E+03",
        "1.212396E+03",
        "1.208135E+03",
    ],
    [
        "8.673095E+05",
        "1.278862E+08",
        "1.822577E+07",
        "2.922214E+05",
        "2.402962E+03",
        "4.355866E-15",
        "3.686389E+04",
        "3.897276E+06",
        "2.667273E+04",
        "1.220146E+03",
        "1.212712E+03",
    ],
    [
        "5.776251E+05",
        "6.979139E+07",
        "1.236814E+07",
        "2.492868E+05",
        "1.405739E+03",
        "2.470651E-15",
        "5.291986E+04",
        "1.069814E+06",
        "2.667273E+04",
        "1.116385E+03",
        "1.112999E+03",
    ],
    [
        "3.740304E+05",
        "4.782720E+07",
        "5.240380E+06",
        "1.759875E+05",
        "5.501649E+02",
        "1.571889E-15",
        "8.896776E+04",
        "1.979741E+06",
        "9.121815E+03",
        "1.031247E+03",
        "1.024848E+03",
    ],
    [
        "6.748339E+05",
        "1.245315E+08",
        "2.369010E+07",
        "4.911583E+05",
        "4.578781E+03",
        "4.564420E-15",
        "3.244595E+04",
        "5.370833E+06",
        "2.667273E+04",
        "1.306052E+03",
        "1.293374E+03",
    ],
    [
        "5.528601E+05",
        "1.198041E+08",
        "3.495798E+07",
        "9.339618E+05",
        "1.096255E+04",
        "4.974543E-15",
        "2.686428E+04",
        "4.889974E+06",
        "2.805445E+04",
        "1.361868E+03",
        "1.347389E+03",
    ],
    [
        "1.375488E+14",
        "0.000000E+00",
        "2.049687E+19",
        "5.498695E+18",
        "2.451733E+17",
        "1.261066E-03",
        "0.000000E+00",
        "0.000000E+00",
        "0.000000E+00",
        "1.027318E+03",
        "2.814648E+02",
    ],
    [
        "4.427443E+13",
        "0.000000E+00",
        "6.597567E+18",
        "1.769929E+18",
        "7.891680E+16",
        "4.059139E-04",
        "0.000000E+00",
        "0.000000E+00",
        "0.000000E+00",
        "1.027318E+03",
        "2.274180E+02",
    ],
    [
        "2.127829E+12",
        "0.000000E+00",
        "3.170791E+17",
        "8.506280E+16",
        "3.792741E+15",
        "1.950822E-05",
        "0.000000E+00",
        "0.000000E+00",
        "0.000000E+00",
        "1.027318E+03",
        "2.374389E+02",
    ],
    [
        "1.412184E+11",
        "0.000000E+00",
        "2.104370E+16",
        "5.645392E+15",
        "2.517142E+14",
        "1.294709E-06",
        "0.000000E+00",
        "0.000000E+00",
        "0.000000E+00",
        "1.027318E+03",
        "2.795551E+02",
    ],
    [
        "1.254884E+10",
        "0.000000E+00",
        "1.874533E+15",
        "4.923051E+14",
        "2.239685E+13",
        "1.147668E-07",
        "0.000000E+00",
        "0.000000E+00",
        "0.000000E+00",
        "1.027318E+03",
        "2.190732E+02",
    ],
    [
        "5.196477E+05",
        "1.274494E+08",
        "4.850450E+07",
        "1.720838E+06",
        "2.354487E+04",
        "5.881940E-15",
        "2.500078E+04",
        "6.279210E+06",
        "2.667273E+04",
        "1.426412E+03",
        "1.408608E+03",
    ],
    [
        "4.260860E+07",
        "1.241342E+11",
        "4.929562E+12",
        "1.048407E+12",
        "4.993465E+10",
        "2.914304E-10",
        "8.831229E+06",
        "2.252516E+05",
        "2.415246E-42",
        "1.027318E+03",
        "1.934071E+02",
    ],
];

/// The 17 inputs of the reference test driver (`nrlmsise-00_test.c` of
/// the same release), which vary one quantity at a time about the first.
fn reference_inputs() -> Vec<MsisInput> {
    let base = MsisInput {
        position: GeodeticPosition::wgs84(60.0, -70.0, 400_000.0),
        day_of_year: 172,
        ut_seconds: 29_000.0,
        local_solar_time_h: 16.0,
        f107_previous_day: 150.0,
        f107_81_day_mean: 150.0,
        geomagnetic: GeomagneticActivity::DailyAp(4.0),
    };
    let storm = GeomagneticActivity::History(ApHistory {
        daily: 100.0,
        current: 100.0,
        hours_3_before: 100.0,
        hours_6_before: 100.0,
        hours_9_before: 100.0,
        mean_12_to_33_hours_before: 100.0,
        mean_36_to_57_hours_before: 100.0,
    });
    let mut inputs = Vec::from([base; 17]);
    inputs[1].day_of_year = 81;
    inputs[2].ut_seconds = 75_000.0;
    inputs[2].position.ellipsoidal_height_m = 1_000_000.0;
    inputs[3].position.ellipsoidal_height_m = 100_000.0;
    inputs[4].position.latitude_deg = 0.0;
    inputs[5].position.longitude_deg = 0.0;
    inputs[6].local_solar_time_h = 4.0;
    inputs[7].f107_81_day_mean = 70.0;
    inputs[8].f107_previous_day = 180.0;
    inputs[9].geomagnetic = GeomagneticActivity::DailyAp(40.0);
    inputs[10].position.ellipsoidal_height_m = 0.0;
    inputs[11].position.ellipsoidal_height_m = 10_000.0;
    inputs[12].position.ellipsoidal_height_m = 30_000.0;
    inputs[13].position.ellipsoidal_height_m = 50_000.0;
    inputs[14].position.ellipsoidal_height_m = 70_000.0;
    inputs[15].geomagnetic = storm;
    inputs[16].geomagnetic = storm;
    inputs[16].position.ellipsoidal_height_m = 100_000.0;
    inputs
}

/// `value` as C's `%E` prints it: six decimals and a signed exponent of at
/// least two digits.
fn c_exponent_format(value: f64) -> String {
    let rust = format!("{value:.6E}");
    let (mantissa, exponent) = rust.split_once('E').expect("an exponent");
    let exponent: i32 = exponent.parse().expect("an integer exponent");
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}E{sign}{:02}", exponent.abs())
}

#[test]
fn the_c_format_matches_printf() {
    assert_eq!(c_exponent_format(666_517.7), "6.665177E+05");
    assert_eq!(c_exponent_format(4.074_714e-15), "4.074714E-15");
    assert_eq!(c_exponent_format(0.0), "0.000000E+00");
    assert_eq!(c_exponent_format(2.819_879e-42), "2.819879E-42");
}

#[test]
fn the_seventeen_reference_cases_print_as_the_reference_states() {
    for (case, (input, expected)) in reference_inputs().iter().zip(REFERENCE_OUTPUT).enumerate() {
        let Reference { d, t } = Nrlmsise00::STANDARD.reference(input).expect("a state");
        let printed: Vec<String> = d.iter().chain(&t).map(|v| c_exponent_format(*v)).collect();
        assert_eq!(printed, expected, "test case {}", case + 1);
    }
}

#[test]
fn the_public_state_is_the_reference_output_in_si() {
    for input in reference_inputs() {
        let Reference { d, t } = Nrlmsise00::STANDARD.reference(&input).expect("a state");
        let state = Nrlmsise00::STANDARD.evaluate(&input).expect("a state");
        let n = state.number_density;
        assert_eq!(n.helium_m3, d[0] * 1e6);
        assert_eq!(n.atomic_oxygen_m3, d[1] * 1e6);
        assert_eq!(n.molecular_nitrogen_m3, d[2] * 1e6);
        assert_eq!(n.molecular_oxygen_m3, d[3] * 1e6);
        assert_eq!(n.argon_m3, d[4] * 1e6);
        assert_eq!(n.atomic_hydrogen_m3, d[6] * 1e6);
        assert_eq!(n.atomic_nitrogen_m3, d[7] * 1e6);
        assert_eq!(n.anomalous_oxygen_m3, d[8] * 1e6);
        assert!((state.mass_density_kg_m3 / (d[5] * 1e3) - 1.0).abs() < 1e-15);
        assert_eq!(state.exospheric_temperature_k, t[0]);
        assert_eq!(state.temperature_k, t[1]);
    }
}

#[test]
fn the_drag_density_adds_anomalous_oxygen_to_the_thermal_species() {
    for input in reference_inputs() {
        let state = Nrlmsise00::STANDARD.evaluate(&input).expect("a state");
        let anomalous = AMU_G * 16.0 * state.number_density.anomalous_oxygen_m3 * 1e-6 * 1e3;
        let added = state.drag_mass_density_kg_m3 - state.mass_density_kg_m3;
        assert!(added >= 0.0);
        assert!((added - anomalous).abs() <= 1e-12 * state.drag_mass_density_kg_m3);
    }
}

#[test]
fn an_evaluation_leaves_nothing_behind_for_the_next() {
    let inputs = reference_inputs();
    let fresh: Vec<Reference> = inputs
        .iter()
        .map(|input| Nrlmsise00::STANDARD.reference(input).expect("a state"))
        .collect();
    // In the opposite order every call follows a different one.
    for (input, expected) in inputs.iter().zip(&fresh).rev() {
        assert_eq!(Nrlmsise00::STANDARD.reference(input).as_ref(), Ok(expected));
    }
}

#[test]
fn every_lower_parameter_set_is_of_the_set_the_expansion_expects() {
    // The reference refuses a lower-atmosphere set whose entry 99 names a
    // different parameter set than 2; zero stands for 2.
    for set in coefficients::PTL.iter().chain(&coefficients::PMA) {
        assert!(set[99] == 0.0 || set[99] == 2.0);
    }
}

#[test]
fn the_ap_history_function_is_set_before_the_lower_atmosphere_reads_it() {
    // A parameter set whose entry 51 is zero leaves the function as the
    // set before it did. The lower-boundary temperature set is evaluated
    // on every call before anything reads the function, so its entry
    // being non-zero keeps one call from depending on another.
    assert_ne!(coefficients::PD[3][51], 0.0);
}

#[test]
fn below_the_ellipsoid_is_refused() {
    let mut input = reference_inputs()[0];
    input.position.ellipsoidal_height_m = -1.0;
    assert_eq!(
        Nrlmsise00::STANDARD.evaluate(&input),
        Err(ThermosphereError::BelowModel {
            altitude_km: -0.001,
            lowest_km: 0.0
        })
    );
}

#[test]
fn the_mixing_switch_is_needed_only_below_the_mesopause() {
    let model = Nrlmsise00::new(
        Switches::STANDARD.with(Effect::DiffusiveEquilibriumDeparture, Switch::Off),
    );
    assert_eq!(
        model.switches().get(Effect::DiffusiveEquilibriumDeparture),
        Switch::Off
    );
    let mut input = reference_inputs()[0];
    assert!(model.evaluate(&input).is_ok());
    input.position.ellipsoidal_height_m = 72_500.0;
    assert!(model.evaluate(&input).is_ok());
    input.position.ellipsoidal_height_m = 50_000.0;
    assert_eq!(
        model.evaluate(&input),
        Err(ThermosphereError::MixingSwitchedOff { altitude_km: 50.0 })
    );
}

#[test]
fn an_overflowing_evaluation_is_an_error_not_a_number() {
    let mut input = reference_inputs()[0];
    input.f107_81_day_mean = 1e300;
    assert_eq!(
        Nrlmsise00::STANDARD.evaluate(&input),
        Err(ThermosphereError::NotFinite)
    );
}
