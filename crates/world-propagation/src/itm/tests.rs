use std::vec::Vec;

use super::complex::Complex;
use super::variability::inverse_ccdf;
use super::*;
use crate::{Frequency, ProfilePoint};

const AVERAGE_GROUND: Ground = Ground {
    relative_permittivity: 15.0,
    conductivity_s_m: 0.005,
};

fn flat(length_m: f64, intervals: usize, height_m: f64) -> Vec<ProfilePoint> {
    (0..=intervals)
        .map(|i| ProfilePoint {
            distance_m: length_m * i as f64 / intervals as f64,
            height_m,
        })
        .collect()
}

fn hilly(length_m: f64, intervals: usize) -> Vec<ProfilePoint> {
    (0..=intervals)
        .map(|i| {
            let x = length_m * i as f64 / intervals as f64;
            let height_m = 120.0 * libm::sin(x / 1_500.0) + 45.0 * libm::cos(x / 600.0) + 180.0;
            ProfilePoint {
                distance_m: x,
                height_m,
            }
        })
        .collect()
}

fn percent(p: f64) -> Percentage {
    Percentage::new(p).unwrap()
}

fn signal(mhz: f64) -> Signal {
    Signal {
        frequency: Frequency::from_mhz(mhz).unwrap(),
        polarisation: Polarisation::Vertical,
    }
}

fn model(mode: VariabilityMode) -> Itm {
    Itm::new(Climate::ContinentalTemperate, 301.0, AVERAGE_GROUND, mode).unwrap()
}

fn loss_over(
    terrain: &[ProfilePoint],
    heights: [f64; 2],
    tls: [f64; 3],
    itm: &Itm,
    mhz: f64,
) -> ItmLoss {
    let path = ItmPath {
        profile: terrain,
        tx_height_m: heights[0],
        rx_height_m: heights[1],
        time: percent(tls[0]),
        location: percent(tls[1]),
        situation: percent(tls[2]),
    };
    itm.path_loss(&path, &signal(mhz)).unwrap()
}

#[test]
fn inverse_ccdf_is_the_normal_quantile_to_hastings_accuracy() {
    // Q⁻¹ of the standard normal: 0, 1.281552, 2.326348, 3.090232.
    assert!(inverse_ccdf(0.5).abs() < 4.5e-4, "Q⁻¹(0.5)");
    for (q, z) in [(0.1, 1.281_552), (0.01, 2.326_348), (0.001, 3.090_232)] {
        assert!((inverse_ccdf(q) - z).abs() < 4.5e-4, "Q⁻¹({q})");
        assert!(
            (inverse_ccdf(1.0 - q) + inverse_ccdf(q)).abs() < 1e-12,
            "Q⁻¹(1-{q})={} vs -Q⁻¹({q})={}",
            inverse_ccdf(1.0 - q),
            -inverse_ccdf(q)
        );
    }
    // Exact binary complements verify bit-exact antisymmetry for q != 0.5.
    for q in [0.25, 0.125, 0.0625] {
        assert_eq!(inverse_ccdf(1.0 - q), -inverse_ccdf(q));
    }
}

#[test]
fn complex_square_root_and_division() {
    let z = Complex::new(14.0, 18.0);
    let r = z.sqrt();
    let back = r * r;
    assert!((back.re - 14.0).abs() < 1e-12 && (back.im - 18.0).abs() < 1e-12);
    assert!(r.re > 0.0);
    let q = z / r;
    assert!((q.re - r.re).abs() < 1e-12 && (q.im - r.im).abs() < 1e-12);
}

#[test]
fn reference_attenuation_is_continuous_through_every_mode_boundary() {
    // Over a flat sea-level path the line-of-sight, diffraction and
    // troposcatter lines are joined at d_sML and d_x by construction.
    let itm = model(VariabilityMode::Broadcast);
    let mut previous: Option<(f64, ItmLoss)> = None;
    let mut modes = Vec::new();
    let mut boundaries = Vec::new();
    for step in 0..=1200 {
        let length_m = 5_000.0 + 500.0 * f64::from(step);
        let terrain = flat(length_m, 200, 0.0);
        let loss = loss_over(&terrain, [30.0, 10.0], [50.0, 50.0, 50.0], &itm, 300.0);
        if let Some((prev_len, prev)) = previous {
            let jump = loss.reference_attenuation_db - prev.reference_attenuation_db;
            assert!(jump > -1e-9, "{length_m} m: A_ref falls by {jump} dB");
            if prev.mode != loss.mode {
                assert!(
                    jump.abs() < 0.5,
                    "{length_m} m: A_ref jumps {jump} dB across {:?} -> {:?}",
                    prev.mode,
                    loss.mode
                );
                boundaries.push((prev_len, length_m));
            }
        }
        if modes.last() != Some(&loss.mode) {
            modes.push(loss.mode);
        }
        previous = Some((length_m, loss));
    }
    assert_eq!(
        modes,
        [
            PropagationMode::LineOfSight,
            PropagationMode::Diffraction,
            PropagationMode::Troposcatter
        ]
    );
    assert_eq!(boundaries.len(), 2);
    for (start_m, end_m) in boundaries {
        let mut prev_fine: Option<ItmLoss> = None;
        for step in 0..=500 {
            let length_m = start_m + (end_m - start_m) * f64::from(step) / 500.0;
            let terrain = flat(length_m, 200, 0.0);
            let loss = loss_over(&terrain, [30.0, 10.0], [50.0, 50.0, 50.0], &itm, 300.0);
            if let Some(prev) = prev_fine {
                let jump = loss.reference_attenuation_db - prev.reference_attenuation_db;
                assert!(jump.abs() < 0.1, "{length_m} m: fine jump {jump} dB");
            }
            prev_fine = Some(loss);
        }
    }
}

#[test]
fn loss_grows_with_each_percentage() {
    let terrain = hilly(60_000.0, 300);
    let itm = model(VariabilityMode::Broadcast);
    let at = |tls: [f64; 3]| loss_over(&terrain, [20.0, 10.0], tls, &itm, 900.0);
    assert!(at([50.0, 50.0, 50.0]).warnings.is_empty());
    for axis in 0..3 {
        let mut previous = f64::NEG_INFINITY;
        for p in [1.0, 10.0, 30.0, 50.0, 70.0, 90.0, 99.0] {
            let mut tls = [50.0; 3];
            tls[axis] = p;
            let loss = at(tls).basic_transmission_loss_db;
            assert!(loss > previous, "axis {axis}, {p} %: {loss} dB");
            previous = loss;
        }
    }
    let tails = at([0.05, 50.0, 50.0]);
    assert!(tails.warnings.contains(ItmWarnings::EXTREME_VARIABILITIES));
}

#[test]
fn variability_modes_fold_the_quantiles_they_say() {
    let terrain = hilly(40_000.0, 200);
    let check = |itm: Itm, same: [[f64; 3]; 2]| {
        let a = loss_over(&terrain, [20.0, 10.0], same[0], &itm, 450.0);
        let b = loss_over(&terrain, [20.0, 10.0], same[1], &itm, 450.0);
        assert_eq!(a.basic_transmission_loss_db, b.basic_transmission_loss_db);
    };
    check(
        model(VariabilityMode::SingleMessage),
        [[10.0, 90.0, 30.0], [70.0, 20.0, 30.0]],
    );
    check(
        model(VariabilityMode::Accidental),
        [[10.0, 90.0, 30.0], [10.0, 20.0, 30.0]],
    );
    check(
        model(VariabilityMode::Mobile),
        [[10.0, 90.0, 30.0], [10.0, 20.0, 30.0]],
    );
    let known_site = model(VariabilityMode::Broadcast).without_location_variability();
    check(known_site, [[10.0, 90.0, 30.0], [10.0, 20.0, 30.0]]);
    let itm = model(VariabilityMode::Broadcast);
    let a = loss_over(&terrain, [20.0, 10.0], [10.0, 90.0, 30.0], &itm, 450.0);
    let b = loss_over(&terrain, [20.0, 10.0], [10.0, 20.0, 30.0], &itm, 450.0);
    assert!(a.basic_transmission_loss_db > b.basic_transmission_loss_db);
}

#[test]
fn excess_loss_is_basic_loss_less_itm_free_space() {
    let terrain = flat(10_000.0, 100, 0.0);
    let itm = model(VariabilityMode::Broadcast);
    let loss = loss_over(&terrain, [30.0, 30.0], [50.0, 50.0, 50.0], &itm, 1000.0);
    let free_space = 32.45 + 20.0 * libm::log10(1000.0) + 20.0 * libm::log10(10.0);
    assert!((loss.free_space_loss_db - free_space).abs() < 1e-12);
    assert!((loss.loss_db() - (loss.basic_transmission_loss_db - free_space)).abs() < 1e-12);
    assert_eq!(loss.distance_m, 10_000.0);
}

#[test]
fn inputs_outside_the_model_are_refused() {
    let terrain = flat(20_000.0, 100, 0.0);
    let itm = model(VariabilityMode::Broadcast);
    let path = |h_tx: f64| ItmPath {
        profile: terrain.as_slice(),
        tx_height_m: h_tx,
        rx_height_m: 10.0,
        time: percent(50.0),
        location: percent(50.0),
        situation: percent(50.0),
    };
    assert!(itm.path_loss(&path(0.4), &signal(300.0)).is_err());
    assert!(itm.path_loss(&path(3001.0), &signal(300.0)).is_err());
    assert!(itm.path_loss(&path(10.0), &signal(19.0)).is_err());
    assert!(itm.path_loss(&path(10.0), &signal(20_001.0)).is_err());
    let near_edge = itm.path_loss(&path(0.7), &signal(30.0)).unwrap();
    assert!(near_edge.warnings.contains(ItmWarnings::TX_TERMINAL_HEIGHT));
    assert!(near_edge.warnings.contains(ItmWarnings::FREQUENCY));

    let ground = |eps, sigma| Ground {
        relative_permittivity: eps,
        conductivity_s_m: sigma,
    };
    let mode = VariabilityMode::Broadcast;
    let climate = Climate::Desert;
    assert!(Itm::new(climate, 249.0, AVERAGE_GROUND, mode).is_err());
    assert!(Itm::new(climate, 401.0, AVERAGE_GROUND, mode).is_err());
    assert!(Itm::new(climate, 301.0, ground(0.9, 0.005), mode).is_err());
    assert!(Itm::new(climate, 301.0, ground(15.0, 0.0), mode).is_err());

    let mut uneven = flat(20_000.0, 100, 0.0);
    uneven[40].distance_m += 1.0;
    let uneven_path = ItmPath {
        profile: uneven.as_slice(),
        ..path(10.0)
    };
    assert!(itm.path_loss(&uneven_path, &signal(300.0)).is_err());
    let short = flat(20_000.0, 1, 0.0);
    let short_path = ItmPath {
        profile: short.as_slice(),
        ..path(10.0)
    };
    assert!(itm.path_loss(&short_path, &signal(300.0)).is_err());
}

#[test]
fn confidence_reliability_maps_as_the_reference_does() {
    let terrain = flat(30_000.0, 150, 0.0);
    let itm = model(VariabilityMode::Accidental);
    let cr = ItmPath::confidence_reliability(
        terrain.as_slice(),
        20.0,
        5.0,
        percent(90.0),
        percent(95.0),
    );
    assert_eq!(cr.time, percent(95.0));
    assert_eq!(cr.location, percent(50.0));
    assert_eq!(cr.situation, percent(90.0));
    let tls = loss_over(&terrain, [20.0, 5.0], [95.0, 50.0, 90.0], &itm, 600.0);
    assert_eq!(itm.path_loss(&cr, &signal(600.0)).unwrap(), tls);
}
