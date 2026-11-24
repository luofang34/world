use super::*;

const X: [f64; MAX_NODES] = [0.0, 0.25, 0.5, 0.75, 1.0];

fn cubic(x: f64) -> f64 {
    2.0 * x * x * x - x * x + 3.0 * x - 1.0
}

fn cubic_slope(x: f64) -> f64 {
    6.0 * x * x - 2.0 * x + 3.0
}

fn cubic_integral(x: f64) -> f64 {
    0.5 * x * x * x * x - x * x * x / 3.0 + 1.5 * x * x - x
}

#[test]
fn a_clamped_spline_reproduces_a_cubic_and_its_integral() {
    let y = X.map(cubic);
    let y2 = spline(&X, &y, MAX_NODES, cubic_slope(0.0), cubic_slope(1.0));
    for x in [0.0, 0.1, 0.25, 0.4, 0.6, 0.9, 1.0] {
        assert!((splint(&X, &y, &y2, MAX_NODES, x) - cubic(x)).abs() < 1e-13);
        assert!((splini(&X, &y, &y2, MAX_NODES, x) - cubic_integral(x)).abs() < 1e-13);
    }
}

#[test]
fn a_natural_spline_through_a_line_is_the_line() {
    let y = X.map(|x| 4.0 - 3.0 * x);
    let y2 = spline(&X, &y, MAX_NODES, 1e30, 1e30);
    assert!(y2.iter().all(|v| v.abs() < 1e-13));
    assert!((splint(&X, &y, &y2, MAX_NODES, 0.3) - 3.1).abs() < 1e-13);
}

#[test]
fn a_shorter_node_set_ignores_the_unused_entries() {
    let x = [0.0, 0.5, 1.0, 9.0, 9.0];
    let y = [cubic(0.0), cubic(0.5), cubic(1.0), 9.0, 9.0];
    let y2 = spline(&x, &y, 3, cubic_slope(0.0), cubic_slope(1.0));
    // Two intervals cannot hold the cubic exactly; the nodes are exact.
    assert!((splint(&x, &y, &y2, 3, 0.5) - cubic(0.5)).abs() < 1e-13);
    assert!((splint(&x, &y, &y2, 3, 0.2) - cubic(0.2)).abs() < 2e-2);
}
