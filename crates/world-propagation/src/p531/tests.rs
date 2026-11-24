use super::*;
use crate::units::{Frequency, Polarisation};

fn sig(ghz: f64) -> Signal {
    Signal {
        frequency: Frequency::from_ghz(ghz).unwrap(),
        polarisation: Polarisation::Horizontal,
    }
}

fn pct(p: f64) -> Percentage {
    Percentage::new(p).unwrap()
}

#[test]
fn thin_shell_obliquity_reduces_to_unity_at_zenith() {
    let model = P531::new();
    let m90 = model.obliquity_factor(90.0).unwrap();
    assert!((m90 - 1.0).abs() < 1e-15);

    let m30 = model.obliquity_factor(30.0).unwrap();
    let expected_30 = 1.0
        / libm::sqrt(
            1.0 - libm::pow(
                (EARTH_RADIUS_M / (EARTH_RADIUS_M + DEFAULT_SHELL_HEIGHT_M))
                    * libm::cos(30.0_f64.to_radians()),
                2.0,
            ),
        );
    assert!((m30 - expected_30).abs() < 1e-15);
    assert!(m30 > 1.0);

    let m0 = model.obliquity_factor(0.0).unwrap();
    assert!(m0 > m30);
    assert!(m0 < 3.5);
}

#[test]
fn group_delay_phase_advance_dispersion_and_faraday_follow_exact_scaling() {
    let model = P531::new();
    let path = P531Path::new(90.0, 100.0, pct(50.0)).with_geomagnetic_field_t(4.0e-5);
    let at_1ghz = model.path_loss(&path, &sig(1.0)).unwrap();
    let at_2ghz = model.path_loss(&path, &sig(2.0)).unwrap();

    // 100 TECU = 1e18 el/m^2 at 1 GHz = 1e9 Hz -> 40.3 / c seconds.
    let expected_delay_1 = 40.3e18 / (SPEED_OF_LIGHT_M_S * 1.0e18);
    assert!((at_1ghz.slant_tec_tecu - 100.0).abs() < 1e-12);
    assert!((at_1ghz.group_delay_s - expected_delay_1).abs() < 1e-18);
    assert!((at_1ghz.group_range_error_m - 40.3).abs() < 1e-12);

    // Phase advance scales as 1/f and equals 2*pi*f*tau_g.
    assert!(
        (at_1ghz.phase_advance_rad - 2.0 * core::f64::consts::PI * 1.0e9 * expected_delay_1).abs()
            < 1e-9
    );
    assert!((at_1ghz.phase_advance_rad / at_2ghz.phase_advance_rad - 2.0).abs() < 1e-12);

    // Group delay scales as 1/f^2 (4x from 1 to 2 GHz).
    assert!((at_1ghz.group_delay_s / at_2ghz.group_delay_s - 4.0).abs() < 1e-12);

    // Dispersion scales as 1/f^3 (8x from 1 to 2 GHz).
    assert!((at_1ghz.dispersion_ps_per_hz / at_2ghz.dispersion_ps_per_hz - 8.0).abs() < 1e-12);
    assert!(at_1ghz.dispersion_ps_per_hz < 0.0);

    // Faraday rotation scales as 1/f^2.
    let f1 = at_1ghz.faraday_rotation_rad.unwrap();
    let f2 = at_2ghz.faraday_rotation_rad.unwrap();
    assert!((f1 - 2.36e-14 * 4.0e-5 * 1.0e18).abs() < 1e-12);
    assert!((f1 / f2 - 4.0).abs() < 1e-12);
}

#[test]
fn rayleigh_limit_and_nakagami_scintillation_monotonicity() {
    let model = P531::new();
    // At S4 = 1.0 (m = 1, exponential intensity): P(I <= I0) = 1 - exp(-I0) = p.
    // For p = 1 % (0.01), I0 = -ln(0.99), fade = -10 log10(-ln(0.99)) ≈ 19.978 dB.
    let path_1 = P531Path::new(45.0, 20.0, pct(1.0)).with_scintillation_s4(1.0);
    let loss_1 = model.path_loss(&path_1, &sig(1.5)).unwrap();
    let expected_fade = -10.0 * libm::log10(-libm::log(0.99));
    assert!((loss_1.scintillation_fade_db - expected_fade).abs() < 1e-6);
    assert!((loss_1.scintillation_peak_to_peak_db - 27.5).abs() < 1e-9);

    // Rarer percentage -> deeper fade; stronger S4 -> deeper fade.
    let path_01 = P531Path::new(45.0, 20.0, pct(0.1)).with_scintillation_s4(1.0);
    let path_weak = P531Path::new(45.0, 20.0, pct(1.0)).with_scintillation_s4(0.3);
    let loss_01 = model.path_loss(&path_01, &sig(1.5)).unwrap();
    let loss_weak = model.path_loss(&path_weak, &sig(1.5)).unwrap();
    assert!(loss_01.scintillation_fade_db > loss_1.scintillation_fade_db);
    assert!(loss_1.scintillation_fade_db > loss_weak.scintillation_fade_db);

    // Continuity across Wilson–Hilferty boundary around m = 1000 (S4 ≈ 0.03162).
    let fade_just_below = gamma::nakagami_fade_db(0.0317, 0.01);
    let fade_just_above = gamma::nakagami_fade_db(0.0315, 0.01);
    assert!((fade_just_below - fade_just_above).abs() < 0.01);
}

#[test]
fn auroral_absorption_matches_p531_table_2_at_127_mhz() {
    let model = P531::new();
    let sig_127 = Signal {
        frequency: Frequency::from_mhz(127.0).unwrap(),
        polarisation: Polarisation::Horizontal,
    };
    let path_20 = P531Path::new(20.0, 15.0, pct(1.0)).with_zone(IonosphericZone::Auroral);
    let path_5 = P531Path::new(5.0, 15.0, pct(0.1)).with_zone(IonosphericZone::Auroral);
    let loss_20 = model.path_loss(&path_20, &sig_127).unwrap();
    let loss_5 = model.path_loss(&path_5, &sig_127).unwrap();
    assert!((loss_20.absorption_db - 0.9).abs() < 1e-9);
    assert!((loss_5.absorption_db - 2.9).abs() < 1e-9);
}

#[test]
fn rejects_invalid_inputs_and_below_30_mhz() {
    let model = P531::new();
    let low_freq = Signal {
        frequency: Frequency::from_mhz(29.9).unwrap(),
        polarisation: Polarisation::Vertical,
    };
    let valid_path = P531Path::new(30.0, 20.0, pct(1.0));
    assert!(matches!(
        model.path_loss(&valid_path, &low_freq),
        Err(PropagationError::FrequencyOutOfRange { .. })
    ));
    assert!(P531::with_shell_height_m(0.0).is_err());
    assert!(P531::with_shell_height_m(-10.0).is_err());
    assert!(
        model
            .path_loss(&P531Path::new(-0.1, 20.0, pct(1.0)), &sig(1.5))
            .is_err()
    );
    assert!(
        model
            .path_loss(&P531Path::new(90.1, 20.0, pct(1.0)), &sig(1.5))
            .is_err()
    );
    assert!(
        model
            .path_loss(&P531Path::new(30.0, -1.0, pct(1.0)), &sig(1.5))
            .is_err()
    );
    assert!(
        model
            .path_loss(
                &P531Path::new(30.0, 20.0, pct(1.0)).with_scintillation_s4(1.6),
                &sig(1.5)
            )
            .is_err()
    );
    assert!(
        model
            .path_loss(
                &P531Path::new(30.0, 20.0, pct(1.0)).with_geomagnetic_field_t(f64::NAN),
                &sig(1.5)
            )
            .is_err()
    );
}
