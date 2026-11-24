use super::*;

fn ghz(value: f64) -> Frequency {
    Frequency::from_ghz(value).unwrap()
}

/// A dish whose D/λ is `d_over_lambda` at `frequency`.
fn diameter_for(d_over_lambda: f64, frequency: Frequency) -> f64 {
    d_over_lambda * frequency.wavelength_m()
}

#[test]
fn s465_min_angle_follows_recommends_2() {
    let f = ghz(12.0);
    // D/λ ≥ 50: max(1°, 100 λ/D).
    let large = S465::new(diameter_for(200.0, f), f).unwrap();
    assert!((large.min_angle_deg() - 1.0).abs() < 1e-12);
    let medium = S465::new(diameter_for(80.0, f), f).unwrap();
    assert!((medium.min_angle_deg() - 1.25).abs() < 1e-9);
    // D/λ < 50: max(2°, 114 (D/λ)^−1.09).
    let small = S465::new(diameter_for(30.0, f), f).unwrap();
    assert!((small.min_angle_deg() - 114.0 * libm::pow(30.0, -1.09)).abs() < 1e-9);
    let tiny_floor = S465::new(diameter_for(49.0, f), f).unwrap();
    assert!((tiny_floor.min_angle_deg() - 2.0).abs() < 1e-12);
}

#[test]
fn s465_matches_its_formula_at_the_breakpoints() {
    let f = ghz(4.0);
    let pattern = S465::new(diameter_for(200.0, f), f).unwrap();
    assert!((pattern.gain_dbi(1.0).unwrap() - 32.0).abs() < 1e-12);
    assert!((pattern.gain_dbi(10.0).unwrap() - 7.0).abs() < 1e-12);
    let just_inside = pattern.gain_dbi(48.0 - 1e-9).unwrap();
    assert!((just_inside - (32.0 - 25.0 * libm::log10(48.0))).abs() < 1e-6);
    assert_eq!(pattern.gain_dbi(48.0).unwrap(), -10.0);
    assert_eq!(pattern.gain_dbi(180.0).unwrap(), -10.0);
    // The rounded constants leave a 0.03 dB step at 48°.
    assert!((just_inside - -10.0 - -0.0303).abs() < 1e-3);
}

#[test]
fn s465_refuses_its_undefined_main_beam_and_band_edges() {
    let f = ghz(4.0);
    let pattern = S465::new(diameter_for(200.0, f), f).unwrap();
    assert!(matches!(
        pattern.gain_dbi(0.5),
        Err(PropagationError::AngleOutsidePattern { .. })
    ));
    assert!(pattern.gain_dbi(180.1).is_err());
    assert!(pattern.gain_dbi(f64::NAN).is_err());
    assert!(S465::new(1.0, ghz(1.5)).is_err());
    assert!(S465::new(1.0, ghz(32.0)).is_err());
}

#[test]
fn s580_objective_bridge_and_handover() {
    let f = ghz(14.0);
    let pattern = S580::new(diameter_for(150.0, f), f).unwrap();
    assert!((pattern.min_angle_deg() - 1.0).abs() < 1e-12);
    assert!((pattern.gain_dbi(1.0).unwrap() - 29.0).abs() < 1e-12);
    let at_20 = pattern.gain_dbi(20.0).unwrap();
    assert!((at_20 - (29.0 - 25.0 * libm::log10(20.0))).abs() < 1e-12);
    // Note 5's −3.5 dBi bridge meets the objective at 20° and S.465 at
    // 26.3° to within the rounding of its constants.
    let bridge = pattern.gain_dbi(20.0 + 1e-9).unwrap();
    assert_eq!(bridge, -3.5);
    assert!((at_20 - bridge).abs() < 0.04);
    assert_eq!(pattern.gain_dbi(26.3).unwrap(), -3.5);
    let handover = pattern.gain_dbi(26.3 + 1e-9).unwrap();
    assert!((handover - -3.5).abs() < 0.01);
    assert_eq!(pattern.gain_dbi(60.0).unwrap(), -10.0);
}

#[test]
fn s580_needs_d_over_lambda_of_50() {
    let f = ghz(14.0);
    assert!(S580::new(diameter_for(49.0, f), f).is_err());
    let pattern = S580::new(diameter_for(60.0, f), f).unwrap();
    assert!((pattern.min_angle_deg() - 100.0 / 60.0).abs() < 1e-9);
    assert!(pattern.gain_dbi(1.0).is_err());
}

#[test]
fn f1245_large_antenna_is_continuous_through_the_first_side_lobe() {
    let f = ghz(18.0);
    let pattern = F1245::new(diameter_for(300.0, f), f).unwrap();
    let g1 = 2.0 + 15.0 * libm::log10(300.0);
    let gmax = 20.0 * libm::log10(300.0) + 7.7;
    assert!((pattern.first_side_lobe_dbi() - g1).abs() < 1e-12);
    assert!((pattern.gain_dbi(0.0).unwrap() - gmax).abs() < 1e-12);
    let phi_m = pattern.main_beam_until_deg();
    assert!((phi_m - 20.0 / 300.0 * libm::sqrt(gmax - g1)).abs() < 1e-12);
    // The main beam reaches G₁ exactly at φm.
    let below = pattern.gain_dbi(phi_m * (1.0 - 1e-12)).unwrap();
    assert!((below - g1).abs() < 1e-8);
    assert_eq!(pattern.gain_dbi(phi_m).unwrap(), g1);
    // 29 − 25 log φr = G₁ up to the rounding of 12.02.
    let phi_r = pattern.first_side_lobe_until_deg();
    assert!((phi_r - 12.02 * libm::pow(300.0, -0.6)).abs() < 1e-12);
    assert!(phi_r > phi_m);
    let after = pattern.gain_dbi(phi_r).unwrap();
    assert!((after - (29.0 - 25.0 * libm::log10(phi_r))).abs() < 1e-12);
    assert!((after - g1).abs() < 3e-3);
    // Far out: −13 dBi from 48°, after a 0.03 dB step.
    assert_eq!(pattern.gain_dbi(48.0).unwrap(), -13.0);
    let inside = pattern.gain_dbi(48.0 - 1e-9).unwrap();
    assert!((inside - -13.0).abs() < 0.04);
}

#[test]
fn f1245_upper_band_floor_moves_to_120_degrees() {
    let f = ghz(80.0);
    let pattern = F1245::new(diameter_for(300.0, f), f).unwrap();
    assert_eq!(pattern.floor_from_deg(), 120.0);
    let at_60 = pattern.gain_dbi(60.0).unwrap();
    assert!((at_60 - (29.0 - 25.0 * libm::log10(60.0))).abs() < 1e-12);
    assert_eq!(pattern.gain_dbi(120.0).unwrap(), -23.0);
    assert!((pattern.gain_dbi(120.0 - 1e-9).unwrap() - -23.0).abs() < 0.03);
    // 70 GHz itself is the lower band.
    let at_70 = F1245::new(diameter_for(300.0, ghz(70.0)), ghz(70.0)).unwrap();
    assert_eq!(at_70.floor_from_deg(), 48.0);
}

#[test]
fn f1245_small_antenna_follows_recommends_2_2() {
    let f = ghz(7.0);
    let d_over_lambda = 40.0;
    let pattern = F1245::new(diameter_for(d_over_lambda, f), f).unwrap();
    let phi_m = pattern.main_beam_until_deg();
    let side = |phi: f64| 39.0 - 5.0 * libm::log10(d_over_lambda) - 25.0 * libm::log10(phi);
    assert_eq!(pattern.gain_dbi(phi_m).unwrap(), side(phi_m));
    assert!((pattern.gain_dbi(20.0).unwrap() - side(20.0)).abs() < 1e-12);
    let floor = -3.0 - 5.0 * libm::log10(d_over_lambda);
    assert_eq!(pattern.gain_dbi(48.0).unwrap(), floor);
    // §2.2 does not join the main beam to the side lobes: at φm the
    // parabola gives G₁ while the side-lobe line gives its own value.
    let main_end = pattern.gain_dbi(phi_m * (1.0 - 1e-12)).unwrap();
    assert!((main_end - pattern.first_side_lobe_dbi()).abs() < 1e-8);
}

#[test]
fn f1245_refuses_gain_below_first_side_lobe_and_out_of_band() {
    let f = ghz(18.0);
    let d = diameter_for(300.0, f);
    assert!(F1245::with_max_gain(d, f, 30.0).is_err());
    let explicit = F1245::with_max_gain(d, f, 58.0).unwrap();
    assert_eq!(explicit.max_gain_dbi(), 58.0);
    assert!(F1245::new(d, ghz(0.9)).is_err());
    assert!(F1245::new(d, ghz(87.0)).is_err());
}
