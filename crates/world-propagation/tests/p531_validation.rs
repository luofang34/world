//! Public API validation tests for ITU-R P.531 ionospheric propagation.

use core::f64::consts::PI;
use world_propagation::p531::{
    DEFAULT_SHELL_HEIGHT_M, IonosphericZone, P531, P531Path, TECU_ELECTRONS_PER_M2,
    scale_s4_by_frequency,
};
use world_propagation::{
    Frequency, Loss, PathLoss, Percentage, Polarisation, PropagationError, SPEED_OF_LIGHT_M_S,
    Signal,
};

#[cfg(test)]
fn signal_mhz(mhz: f64) -> Signal {
    Signal {
        frequency: Frequency::from_mhz(mhz).unwrap(),
        polarisation: Polarisation::Horizontal,
    }
}

#[test]
fn p531_analytical_group_delay_faraday_and_dispersion_across_bands() {
    let model = P531::new();
    assert_eq!(model.shell_height_m(), DEFAULT_SHELL_HEIGHT_M);

    let pct = Percentage::new(1.0).unwrap();
    let vtec = 45.0;
    let b_long = 3.2e-5;
    let path = P531Path::new(35.0, vtec, pct).with_geomagnetic_field_t(b_long);

    let l1 = model.path_loss(&path, &signal_mhz(1575.42)).unwrap();
    let l2 = model.path_loss(&path, &signal_mhz(1227.60)).unwrap();
    let ka = model.path_loss(&path, &signal_mhz(20_000.0)).unwrap();

    // Analytical formula verification: 40.3 * STEC / (c * f^2)
    let stec_el = l1.slant_tec_tecu * TECU_ELECTRONS_PER_M2;
    let f1 = 1575.42e6;
    let expected_tau1 = 40.3 * stec_el / (SPEED_OF_LIGHT_M_S * f1 * f1);
    assert!((l1.group_delay_s - expected_tau1).abs() < 1e-18);
    assert!((l1.phase_advance_rad - 2.0 * PI * f1 * expected_tau1).abs() < 1e-10);
    assert!((l1.dispersion_ps_per_hz - (-2.0 * expected_tau1 * 1.0e12 / f1)).abs() < 1e-18);

    // Exact frequency monotonicity: L2 < L1 -> larger delay, dispersion, and Faraday rotation.
    assert!(l2.group_delay_s > l1.group_delay_s);
    assert!(l1.group_delay_s > ka.group_delay_s);
    assert!(l2.faraday_rotation_rad.unwrap() > l1.faraday_rotation_rad.unwrap());
    assert!(l1.faraday_rotation_rad.unwrap() > ka.faraday_rotation_rad.unwrap());
    assert!(l2.dispersion_ps_per_hz.abs() > l1.dispersion_ps_per_hz.abs());
}

#[test]
fn p531_zenith_obliquity_and_zero_tec_behavior() {
    let model = P531::with_shell_height_m(350_000.0).unwrap();
    let pct = Percentage::new(50.0).unwrap();

    let zenith = model
        .path_loss(&P531Path::new(90.0, 30.0, pct), &signal_mhz(1_200.0))
        .unwrap();
    assert!((zenith.obliquity_factor - 1.0).abs() < 1e-15);
    assert!((zenith.slant_tec_tecu - 30.0).abs() < 1e-12);

    let zero_tec = model
        .path_loss(&P531Path::new(25.0, 0.0, pct), &signal_mhz(400.0))
        .unwrap();
    assert_eq!(zero_tec.slant_tec_tecu, 0.0);
    assert_eq!(zero_tec.group_delay_s, 0.0);
    assert_eq!(zero_tec.phase_advance_rad, 0.0);
    assert_eq!(zero_tec.dispersion_ps_per_hz, 0.0);
    assert_eq!(zero_tec.loss_db(), 0.0);
    assert!(zero_tec.faraday_rotation_rad.is_none());
}

#[test]
fn p531_scintillation_and_zone_absorption_integration() {
    let model = P531::new();
    let f_l = Frequency::from_ghz(1.5).unwrap();
    let f_c = Frequency::from_ghz(4.0).unwrap();
    let s4_l = 0.6;
    let s4_c = scale_s4_by_frequency(s4_l, f_l, f_c).unwrap();
    assert!(s4_c < s4_l);

    let p01 = Percentage::new(0.1).unwrap();
    let p5 = Percentage::new(5.0).unwrap();
    let path_l_01 = P531Path::new(20.0, 40.0, p01)
        .with_scintillation_s4(s4_l)
        .with_zone(IonosphericZone::from_latitude_deg(65.0));
    let path_l_5 = P531Path::new(20.0, 40.0, p5)
        .with_scintillation_s4(s4_l)
        .with_zone(IonosphericZone::Auroral);

    let out_01 = model.path_loss(&path_l_01, &signal_mhz(1500.0)).unwrap();
    let out_5 = model.path_loss(&path_l_5, &signal_mhz(1500.0)).unwrap();
    assert!(out_01.loss_db() > out_5.loss_db());
    assert!(out_01.scintillation_peak_to_peak_db > 10.0);
    assert!(matches!(
        model.path_loss(&path_l_01, &signal_mhz(20.0)),
        Err(PropagationError::FrequencyOutOfRange { .. })
    ));
}
