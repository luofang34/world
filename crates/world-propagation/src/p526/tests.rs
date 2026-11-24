use core::f64::consts::{PI, SQRT_2};

use super::*;
use crate::{Frequency, Ground, Loss, PathLoss, Polarisation, Signal};

/// C(x) and S(x) to ten decimals, from the Maclaurin series of eq. (6)
/// summed in 60-digit decimal arithmetic (an independent oracle; the
/// Boersma expansions under test are not series).
const FRESNEL_TABLE: [(f64, f64, f64); 8] = [
    (0.5, 0.492_344_225_9, 0.064_732_432_9),
    (1.0, 0.779_893_400_4, 0.438_259_147_4),
    (1.5, 0.445_261_176_0, 0.697_504_960_1),
    (2.0, 0.488_253_406_1, 0.343_415_678_4),
    (2.5, 0.457_413_009_6, 0.619_181_755_8),
    (3.0, 0.605_720_789_3, 0.496_312_999_0),
    (4.0, 0.498_426_033_0, 0.420_515_754_2),
    (5.0, 0.563_631_188_7, 0.499_191_381_9),
];

/// Boersma's expansions are accurate to about 1e-9.
const BOERSMA_TOLERANCE: f64 = 5.0e-9;

#[test]
fn fresnel_integrals_match_the_table_on_both_expansions() {
    for (nu, c, s) in FRESNEL_TABLE {
        let (got_c, got_s) = fresnel_integrals(nu);
        assert!((got_c - c).abs() < BOERSMA_TOLERANCE, "C({nu}) = {got_c}");
        assert!((got_s - s).abs() < BOERSMA_TOLERANCE, "S({nu}) = {got_s}");
    }
}

#[test]
fn fresnel_integrals_are_odd() {
    for (nu, _, _) in FRESNEL_TABLE {
        let (c, s) = fresnel_integrals(nu);
        assert_eq!(fresnel_integrals(-nu), (-c, -s)); // eq. (10)
    }
    assert_eq!(fresnel_integrals(0.0), (0.0, 0.0));
}

/// Eq. (30) evaluated on the tabulated C and S.
fn j_from_table(c: f64, s: f64) -> f64 {
    let a = 1.0 - c - s;
    let b = c - s;
    -20.0 * libm::log10(libm::sqrt(a * a + b * b) / 2.0)
}

#[test]
fn knife_edge_loss_is_eq_30_on_the_fresnel_table() {
    for (nu, c, s) in FRESNEL_TABLE {
        let expected = j_from_table(c, s);
        let got = knife_edge_loss_db(nu);
        assert!(
            (got - expected).abs() < 1.0e-6,
            "J({nu}) = {got}, eq. 30 {expected}"
        );
    }
}

#[test]
fn grazing_incidence_loses_6_db() {
    // An edge exactly on the line of sight halves the field.
    let half = -20.0 * libm::log10(0.5);
    assert!((knife_edge_loss_db(0.0) - half).abs() < 1.0e-12);
    assert!((half - 6.0206).abs() < 1.0e-4);
    // Eq. (31) rounds it.
    assert!((knife_edge_loss_approx_db(0.0) - 6.0).abs() < 0.05);
}

#[test]
fn deep_shadow_follows_the_asymptote() {
    // Far behind the edge the field is 1 / (√2 π ν).
    for nu in [5.0, 10.0, 30.0, 100.0, 1000.0] {
        let asymptote = 20.0 * libm::log10(SQRT_2 * PI * nu);
        let got = knife_edge_loss_db(nu);
        assert!(
            (got - asymptote).abs() < 0.01,
            "J({nu}) = {got}, asymptote {asymptote}"
        );
        // Eq. (31) tends to 12.92 + 20 log ν, 0.03 dB under the asymptote.
        assert!((knife_edge_loss_approx_db(nu) - asymptote).abs() < 0.15);
    }
}

#[test]
fn no_loss_well_clear_of_the_first_fresnel_zone() {
    // The edge's ripple decays as 1 / (√2 π |ν|) about free space.
    for tenths in 30..=200 {
        let nu = -f64::from(tenths) / 10.0;
        let ripple_db = -20.0 * libm::log10(1.0 - 1.0 / (SQRT_2 * PI * -nu));
        let got = knife_edge_loss_db(nu);
        assert!(got.abs() <= ripple_db + 1.0e-6, "J({nu}) = {got}");
        assert_eq!(knife_edge_loss_approx_db(nu), 0.0);
    }
    assert_eq!(knife_edge_loss_approx_db(APPROXIMATION_CUTOFF), 0.0);
    // Eq. (31) is 0.004 dB at the cut-off, so the cut is nearly seamless.
    assert!(knife_edge_loss_approx_db(APPROXIMATION_CUTOFF + 1.0e-9).abs() < 0.01);
}

#[test]
fn approximation_tracks_eq_30_where_it_applies() {
    for hundredths in -70..=400 {
        let nu = f64::from(hundredths) / 100.0;
        let error = knife_edge_loss_approx_db(nu) - knife_edge_loss_db(nu);
        assert!(error.abs() < 0.6, "ν = {nu}: eq. 31 differs by {error} dB");
    }
}

#[test]
fn equivalent_forms_of_nu_agree() {
    let (h, d1, d2, lambda) = (12.0, 3_000.0, 7_000.0, 0.15);
    let nu = diffraction_parameter(h, d1, d2, lambda).unwrap();
    let theta = h / d1 + h / d2; // small-angle diffraction angle
    let eq_28 = libm::sqrt(2.0 * h * theta / lambda);
    let eq_29 = libm::sqrt(2.0 * (d1 + d2) / lambda * (h / d1) * (h / d2));
    assert!((nu - eq_28).abs() < 1.0e-12);
    assert!((nu - eq_29).abs() < 1.0e-12);
    assert!(diffraction_parameter(-h, d1, d2, lambda).unwrap() < 0.0);
    assert!(diffraction_parameter(h, 0.0, d2, lambda).is_err());
}

#[test]
fn knife_edge_model_reports_nu_and_loss() {
    let signal = Signal {
        frequency: Frequency::from_ghz(1.0).unwrap(),
        polarisation: Polarisation::Vertical,
    };
    let geometry = KnifeEdgeGeometry {
        height_m: 0.0,
        d1_m: 1_000.0,
        d2_m: 2_000.0,
    };
    let exact = KnifeEdge::default().path_loss(&geometry, &signal).unwrap();
    assert_eq!(exact.nu, 0.0);
    assert!((exact.loss_db() - 6.0206).abs() < 1.0e-4);
    let approx = KnifeEdge {
        method: KnifeEdgeMethod::Approximation,
    };
    assert!((approx.path_loss(&geometry, &signal).unwrap().loss_db() - 6.0).abs() < 0.05);
}

#[test]
fn smooth_earth_refuses_below_10_mhz_and_degenerate_paths() {
    let model = SmoothEarth::new(DEFAULT_EFFECTIVE_EARTH_RADIUS_M, Ground::LAND).unwrap();
    let path = TerminalPath {
        distance_m: 50_000.0,
        tx_height_m: 10.0,
        rx_height_m: 10.0,
    };
    let low = Signal {
        frequency: Frequency::from_mhz(5.0).unwrap(),
        polarisation: Polarisation::Vertical,
    };
    assert!(model.path_loss(&path, &low).is_err());
    let signal = Signal {
        frequency: Frequency::from_mhz(100.0).unwrap(),
        polarisation: Polarisation::Vertical,
    };
    let flat = TerminalPath {
        tx_height_m: 0.0,
        rx_height_m: 0.0,
        ..path
    };
    assert!(model.path_loss(&flat, &signal).is_err());
    assert!(SmoothEarth::new(0.0, Ground::LAND).is_err());
}

#[test]
fn smooth_earth_loss_grows_continuously_through_the_horizon() {
    let model = SmoothEarth::new(DEFAULT_EFFECTIVE_EARTH_RADIUS_M, Ground::LAND).unwrap();
    let signal = Signal {
        frequency: Frequency::from_mhz(900.0).unwrap(),
        polarisation: Polarisation::Horizontal,
    };
    let loss_at = |distance_m: f64| {
        let path = TerminalPath {
            distance_m,
            tx_height_m: 30.0,
            rx_height_m: 10.0,
        };
        model.path_loss(&path, &signal).unwrap()
    };
    // Marginal line of sight, eq. (21).
    let d_los =
        libm::sqrt(2.0 * DEFAULT_EFFECTIVE_EARTH_RADIUS_M) * (libm::sqrt(30.0) + libm::sqrt(10.0));
    let inside = loss_at(d_los * (1.0 - 1.0e-9));
    let outside = loss_at(d_los * (1.0 + 1.0e-9));
    assert!(inside.line_of_sight && !outside.line_of_sight);
    assert!((inside.loss_db - outside.loss_db).abs() < 1.0e-3);
    // Short paths clear the Fresnel zone: zero loss.
    assert_eq!(loss_at(1_000.0).loss_db, 0.0);
    let mut previous = 0.0;
    for km in 5..=120 {
        let loss = loss_at(f64::from(km) * 1.0e3).loss_db;
        assert!(loss >= previous, "{km} km: {loss} dB after {previous} dB");
        previous = loss;
    }
}
