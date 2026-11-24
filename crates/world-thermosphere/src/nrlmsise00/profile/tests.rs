use super::*;

use crate::nrlmsise00::call::Gravity;

const GRAVITY: Gravity = Gravity {
    gsurf: 980.665,
    re: 6356.77,
};

#[test]
fn the_chemistry_correction_saturates_at_both_ends() {
    assert_eq!(ccor(1_000.0, 2.0, 1.0, 100.0), 1.0);
    assert_eq!(ccor(-1_000.0, 2.0, 1.0, 100.0), libm::exp(2.0));
    assert_eq!(ccor(100.0, 2.0, 1.0, 100.0), libm::exp(1.0));
    assert_eq!(ccor2(1_000.0, 2.0, 1.0, 100.0, 5.0), 1.0);
    assert_eq!(ccor2(-1_000.0, 2.0, 1.0, 100.0, 5.0), libm::exp(2.0));
    assert_eq!(ccor2(100.0, 2.0, 1.0, 100.0, 5.0), libm::exp(1.0));
}

#[test]
fn the_turbopause_blend_tends_to_the_larger_branch() {
    // Far above the turbopause the diffusive density dominates a light
    // species; far below, the mixed one.
    assert_eq!(dnet(1.0e15, 1.0, 10.0, 28.95, 4.0), 1.0e15);
    assert_eq!(dnet(1.0, 1.0e15, 10.0, 28.95, 4.0), 1.0e15);
    // In between it is the power mean of exponent zhm / (xmm - xm).
    let a: f64 = 10.0 / (28.95 - 4.0);
    let blended = dnet(2.0e9, 1.0e9, 10.0, 28.95, 4.0);
    let power_mean = (2.0e9_f64.powf(a) + 1.0e9_f64.powf(a)).powf(1.0 / a);
    assert!((blended / power_mean - 1.0).abs() < 1e-12);
}

#[test]
fn the_turbopause_blend_survives_a_vanished_density() {
    assert_eq!(dnet(0.0, 0.0, 10.0, 28.95, 4.0), 1.0);
    assert_eq!(dnet(5.0, 0.0, 10.0, 28.95, 4.0), 5.0);
    assert_eq!(dnet(0.0, 7.0, 10.0, 28.95, 4.0), 7.0);
}

#[test]
fn the_scale_height_follows_gravity_with_altitude() {
    let low = scalh(120.0, 16.0, 1000.0, &GRAVITY);
    let high = scalh(1_000.0, 16.0, 1000.0, &GRAVITY);
    let stretch: f64 = 1.0 + 120.0 / 6356.77;
    assert!((low - 831.4 * 1000.0 / (16.0 * 980.665) * stretch * stretch).abs() < 1e-9);
    assert!(high > low);
}

#[test]
fn the_bates_profile_runs_from_the_boundary_to_the_exosphere() {
    let bates = Bates {
        tinf: 1000.0,
        tlb: 380.0,
        s2: 0.02,
        zlb: 120.0,
    };
    let mut nodes = Nodes {
        zn: [120.0, 110.0, 100.0, 90.0, 72.5],
        tn: [0.0, 240.0, 195.0, 187.0, 213.0],
        tgn: [0.0, -2.0],
    };
    let at = |alt: f64, nodes: &mut Nodes<5>| densu(alt, 1.0, &bates, 0.0, 0.0, nodes, &GRAVITY);
    assert_eq!(at(120.0, &mut nodes), (380.0, 380.0));
    let (_, exosphere) = at(2_000.0, &mut nodes);
    assert!((exosphere - 1000.0).abs() < 1e-6);
    // Below the boundary the spline joins the profile's value and slope.
    let (_, just_below) = at(119.999, &mut nodes);
    assert_eq!(nodes.tn[0], 380.0);
    assert!((nodes.tgn[0] - (1000.0 - 380.0) * 0.02).abs() < 1e-12);
    assert!((just_below - (380.0 - 0.001 * nodes.tgn[0])).abs() < 1e-5);
    assert_eq!(at(72.5, &mut nodes).1, 213.0);
}
