//! The synthesis against an unnormalised textbook oracle, finite
//! differences, Laplace's equation and the closed form on the polar axis.

use std::vec;
use std::vec::Vec;

use super::*;
use crate::harmonic::{triangular_index, triangular_len};
use crate::{HarmonicCoefficients, workspace_len};

const R: f64 = 6_378_137.0;

/// A deterministic pseudo-random sequence in [-1, 1).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }
}

/// Coefficients with a Kaula-like 1/n² decay, so high degrees stay at
/// realistic magnitudes.
fn synthetic(degree: usize, seed: u64) -> HarmonicCoefficients<Vec<f64>> {
    let mut rng = Lcg(seed);
    let len = triangular_len(degree);
    let (mut c, mut s) = (vec![0.0; len], vec![0.0; len]);
    for n in 0..=degree {
        let scale = if n == 0 { 1.0 } else { 1e-5 / (n * n) as f64 };
        for m in 0..=n {
            let i = triangular_index(n, m);
            c[i] = scale * rng.next();
            s[i] = if m == 0 { 0.0 } else { scale * rng.next() };
        }
    }
    HarmonicCoefficients::new(degree, c, s).unwrap()
}

fn run(
    coefficients: &HarmonicCoefficients<Vec<f64>>,
    position: [f64; 3],
    truncation: Truncation,
    derivatives: Derivatives,
) -> Synthesis {
    let mut buffer = vec![0.0; workspace_len(truncation.degree)];
    let mut ws = Workspace::new(&mut buffer, truncation.degree).unwrap();
    synthesize(&mut ws, coefficients, position, R, truncation, derivatives).unwrap()
}

fn factorial(k: usize) -> f64 {
    (1..=k).map(|i| i as f64).product()
}

/// Montenbruck & Gill eqs. 3.30–3.33 with unnormalised coefficients:
/// (Φ, ∇Φ), independent of the normalised factors under test.
fn oracle(coefficients: &HarmonicCoefficients<Vec<f64>>, p: [f64; 3]) -> (f64, [f64; 3]) {
    let nmax = coefficients.degree();
    let size = nmax + 2;
    let (mut v, mut w) = (
        vec![vec![0.0; size + 1]; size + 1],
        vec![vec![0.0; size + 1]; size + 1],
    );
    let [x, y, z] = p;
    let r2 = x * x + y * y + z * z;
    let (u, vv, ww, q) = (x * R / r2, y * R / r2, z * R / r2, R * R / r2);
    v[0][0] = R / r2.sqrt();
    for m in 0..=size {
        if m > 0 {
            let f = (2 * m - 1) as f64;
            v[m][m] = f * (u * v[m - 1][m - 1] - vv * w[m - 1][m - 1]);
            w[m][m] = f * (u * w[m - 1][m - 1] + vv * v[m - 1][m - 1]);
        }
        for n in m + 1..=size {
            let a = (2 * n - 1) as f64 / (n - m) as f64;
            let b = (n + m - 1) as f64 / (n - m) as f64;
            let (v2, w2) = if n >= m + 2 {
                (v[n - 2][m], w[n - 2][m])
            } else {
                (0.0, 0.0)
            };
            v[n][m] = a * ww * v[n - 1][m] - b * q * v2;
            w[n][m] = a * ww * w[n - 1][m] - b * q * w2;
        }
    }
    let (mut phi, mut g) = (0.0, [0.0; 3]);
    for n in 0..=nmax {
        for m in 0..=n {
            let (cn, sn) = coefficients.coefficient(n, m);
            let k = if m == 0 { 1.0 } else { 2.0 };
            let norm = (k * (2 * n + 1) as f64 * factorial(n - m) / factorial(n + m)).sqrt();
            let (c, s) = (cn * norm, if m == 0 { 0.0 } else { sn * norm });
            phi += c * v[n][m] + s * w[n][m];
            if m == 0 {
                g[0] -= c * v[n + 1][1];
                g[1] -= c * w[n + 1][1];
            } else {
                let f = factorial(n - m + 2) / factorial(n - m);
                g[0] += 0.5 * (-c * v[n + 1][m + 1] - s * w[n + 1][m + 1])
                    + 0.5 * f * (c * v[n + 1][m - 1] + s * w[n + 1][m - 1]);
                g[1] += 0.5 * (-c * w[n + 1][m + 1] + s * v[n + 1][m + 1])
                    + 0.5 * f * (-c * w[n + 1][m - 1] + s * v[n + 1][m - 1]);
            }
            g[2] += (n - m + 1) as f64 * (-c * v[n + 1][m] - s * w[n + 1][m]);
        }
    }
    (phi, g.map(|gi| gi / R))
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn points() -> Vec<[f64; 3]> {
    let r = R + 400_000.0;
    let at = |lat: f64, lon: f64, r: f64| {
        let (lat, lon) = (lat.to_radians(), lon.to_radians());
        [
            r * lat.cos() * lon.cos(),
            r * lat.cos() * lon.sin(),
            r * lat.sin(),
        ]
    };
    vec![
        at(0.0, 0.0, r),
        at(37.0, -122.0, r),
        at(-63.0, 171.0, R - 21_000.0),
        at(89.9999, 45.0, r),
        [0.0, 0.0, r],
        [0.0, 0.0, -R],
        at(12.0, 300.0, 3.0 * R),
    ]
}

#[test]
fn potential_and_gradient_match_the_unnormalised_oracle() {
    // Degree 12 keeps the oracle's factorials exact in f64; agreement to
    // 1e-12 of the magnitude is rounding in two different recursions.
    let coefficients = synthetic(12, 7);
    for p in points() {
        let got = run(
            &coefficients,
            p,
            Truncation::full(12),
            Derivatives::Gradient,
        );
        let (phi, g) = oracle(&coefficients, p);
        assert!((got.potential - phi).abs() <= 1e-12 * phi.abs(), "{p:?}");
        let err = norm([0, 1, 2].map(|i| got.gradient[i] - g[i]));
        assert!(err <= 1e-12 * norm(g), "{p:?}: {:?} vs {g:?}", got.gradient);
        assert!(got.hessian.is_none());
    }
}

#[test]
fn the_hessian_matches_central_differences_of_the_gradient() {
    // A 1 m step at orbital radius: truncation error ~ (h/r)² ≈ 1e-14,
    // rounding ~ ε·r/h ≈ 1e-9 of the tensor; 1e-7 leaves margin.
    for degree in [12, 70] {
        let coefficients = synthetic(degree, 11);
        for p in points() {
            let h = 1.0;
            let got = run(
                &coefficients,
                p,
                Truncation::full(degree),
                Derivatives::Hessian,
            );
            let t = got.hessian.unwrap().to_matrix();
            let scale = t.iter().flatten().fold(0.0f64, |a, b| a.max(b.abs()));
            for j in 0..3 {
                let shifted = |d: f64| {
                    let mut q = p;
                    q[j] += d;
                    run(
                        &coefficients,
                        q,
                        Truncation::full(degree),
                        Derivatives::Gradient,
                    )
                    .gradient
                };
                let (plus, minus) = (shifted(h), shifted(-h));
                for i in 0..3 {
                    let fd = (plus[i] - minus[i]) / (2.0 * h);
                    assert!(
                        (t[i][j] - fd).abs() <= 1e-7 * scale,
                        "degree {degree} {p:?} [{i}][{j}]: {} vs {fd}",
                        t[i][j]
                    );
                }
            }
            // Laplace: the trace cancels to rounding of its largest term.
            assert!(got.hessian.unwrap().trace().abs() <= 1e-12 * scale, "{p:?}");
        }
    }
}

#[test]
fn degree_360_is_finite_at_the_pole_and_the_equator_and_obeys_laplace() {
    let degree = 360;
    let coefficients = synthetic(degree, 3);
    for p in [
        [0.0, 0.0, R],
        [1e-3, 0.0, -R],
        [R, 0.0, 0.0],
        [0.0, R + 1.0, 0.0],
    ] {
        let got = run(
            &coefficients,
            p,
            Truncation::full(degree),
            Derivatives::Hessian,
        );
        let t = got.hessian.unwrap();
        assert!(got.potential.is_finite() && got.gradient.iter().all(|g| g.is_finite()));
        assert!(
            t.to_matrix().iter().flatten().all(|v| v.is_finite()),
            "{p:?}"
        );
        let scale = t
            .to_matrix()
            .iter()
            .flatten()
            .fold(0.0f64, |a, b| a.max(b.abs()));
        // 360 degrees of terms of alternating sign: 1e-10 of the largest
        // component is still far below any physical gradient signal.
        assert!(t.trace().abs() <= 1e-10 * scale, "{p:?}: {t:?}");
    }
}

#[test]
fn degree_360_on_the_polar_axis_is_the_zonal_closed_form() {
    // On the axis only the zonals survive: P̄n0(±1) = (±1)^n √(2n + 1).
    let degree = 360;
    let coefficients = synthetic(degree, 5);
    for z in [R, -R * 1.01] {
        let got = run(
            &coefficients,
            [0.0, 0.0, z],
            Truncation::full(degree),
            Derivatives::Gradient,
        );
        let ratio = R / z.abs();
        let sign = z.signum();
        let (mut phi, mut dphi) = (0.0, 0.0);
        for n in 0..=degree {
            let term = ratio.powi(n as i32 + 1)
                * sign.powi(n as i32)
                * ((2 * n + 1) as f64).sqrt()
                * coefficients.coefficient(n, 0).0;
            phi += term;
            dphi -= (n + 1) as f64 * term / z.abs() * sign;
        }
        // Summation order differs; 1e-12 relative is rounding of the
        // potential, and the gradient's (n + 1) weights cancel more, so it
        // gets 1e-10.
        assert!((got.potential - phi).abs() <= 1e-12 * phi.abs());
        assert!(
            (got.gradient[2] - dphi).abs() <= 1e-10 * dphi.abs(),
            "{} vs {dphi}",
            got.gradient[2]
        );
    }
}

#[cfg(feature = "alloc")]
#[test]
fn a_fixed_buffer_and_a_heap_buffer_give_identical_bits() {
    let coefficients = synthetic(12, 9);
    let p = points()[1];
    let mut fixed = [0.0; workspace_len(12)];
    let mut ws = Workspace::new(&mut fixed, 12).unwrap();
    let a = synthesize(
        &mut ws,
        &coefficients,
        p,
        R,
        Truncation::full(12),
        Derivatives::Hessian,
    )
    .unwrap();
    let mut heap = crate::WorkspaceBuffer::new(360);
    let b = synthesize(
        &mut heap.workspace().unwrap(),
        &coefficients,
        p,
        R,
        Truncation::full(12),
        Derivatives::Hessian,
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn truncation_by_order_and_minimum_degree_drops_exactly_those_terms() {
    let coefficients = synthetic(12, 13);
    let p = points()[2];
    let full = run(
        &coefficients,
        p,
        Truncation::full(12),
        Derivatives::Gradient,
    )
    .potential;
    let central = run(&coefficients, p, Truncation::full(0), Derivatives::Gradient).potential;
    let rest = run(
        &coefficients,
        p,
        Truncation {
            min_degree: 1,
            degree: 12,
            order: 12,
        },
        Derivatives::Gradient,
    )
    .potential;
    assert!((central + rest - full).abs() <= 1e-15 * full.abs());
    let zonal = run(
        &coefficients,
        p,
        Truncation {
            min_degree: 0,
            degree: 12,
            order: 0,
        },
        Derivatives::Gradient,
    );
    assert_ne!(zonal.potential, full);
}

#[test]
fn malformed_requests_are_refused() {
    let coefficients = synthetic(12, 1);
    let mut small = [0.0; 10];
    assert!(matches!(
        Workspace::new(&mut small, 12),
        Err(FieldError::WorkspaceTooSmall { .. })
    ));
    let mut buffer = [0.0; workspace_len(6)];
    let mut ws = Workspace::new(&mut buffer, 6).unwrap();
    let go = |ws: &mut Workspace<'_>, p, t| {
        synthesize(ws, &coefficients, p, R, t, Derivatives::Gradient)
    };
    assert!(matches!(
        go(&mut ws, [R, 0.0, 0.0], Truncation::full(12)),
        Err(FieldError::DegreeExceedsWorkspace { .. })
    ));
    assert!(matches!(
        go(&mut ws, [R, 0.0, 0.0], Truncation::full(13)),
        Err(FieldError::DegreeExceedsModel { .. })
    ));
    let bad = Truncation {
        min_degree: 0,
        degree: 4,
        order: 5,
    };
    assert_eq!(
        go(&mut ws, [R, 0.0, 0.0], bad),
        Err(FieldError::InvalidTruncation)
    );
    for p in [[0.0; 3], [f64::NAN, 0.0, 0.0], [f64::INFINITY, 0.0, 0.0]] {
        assert_eq!(
            go(&mut ws, p, Truncation::full(6)),
            Err(FieldError::InvalidPosition)
        );
    }
}

/// Φ over `band` by the plain spherical sum: normalised Legendre
/// functions of sin φ by the standard column recursion, times (R/r)^(n+1)
/// and cos/sin mλ. Independent of the Cartesian rows, the ring buffer and
/// the factor tables. Returns (Φ, Σ|terms|).
fn spherical(
    coefficients: &HarmonicCoefficients<Vec<f64>>,
    band: (usize, usize),
    lat: f64,
    lon: f64,
    r: f64,
) -> (f64, f64) {
    let (t, u) = (lat.sin(), lat.cos());
    let nmax = band.1;
    let mut p = vec![vec![0.0; nmax + 1]; nmax + 1];
    p[0][0] = 1.0;
    for m in 0..=nmax {
        if m == 1 {
            p[1][1] = 3.0_f64.sqrt() * u;
        } else if m > 1 {
            p[m][m] = ((2 * m + 1) as f64 / (2 * m) as f64).sqrt() * u * p[m - 1][m - 1];
        }
        for n in m + 1..=nmax {
            let (nf, mf) = (n as f64, m as f64);
            let a = ((2.0 * nf - 1.0) * (2.0 * nf + 1.0) / ((nf - mf) * (nf + mf))).sqrt();
            let b = ((2.0 * nf + 1.0) * (nf + mf - 1.0) * (nf - mf - 1.0)
                / ((2.0 * nf - 3.0) * (nf + mf) * (nf - mf)))
                .sqrt();
            let p2 = if n >= m + 2 { p[n - 2][m] } else { 0.0 };
            p[n][m] = a * t * p[n - 1][m] - b * p2;
        }
    }
    let (mut sum, mut scale) = (0.0, 0.0);
    for (n, row) in p.iter().enumerate().skip(band.0) {
        let radial = (R / r).powi(n as i32 + 1);
        for (m, pnm) in row.iter().enumerate().take(n + 1) {
            let (c, s) = coefficients.coefficient(n, m);
            let mf = m as f64 * lon;
            let term = radial * pnm * (c * mf.cos() + s * mf.sin());
            sum += term;
            scale += term.abs();
        }
    }
    (sum, scale)
}

#[test]
fn degrees_200_to_360_match_a_spherical_legendre_sum_off_the_axis() {
    // Every tesseral and sectoral row up to 360 enters this band. The two
    // sums differ in rounding only; 1e-11 of Σ|terms| bounds 65 000
    // terms of independent rounding with a wide margin.
    let coefficients = synthetic(360, 17);
    let band = Truncation {
        min_degree: 200,
        degree: 360,
        order: 360,
    };
    for (lat, lon) in [
        (37.0_f64, -122.0_f64),
        (0.3, 11.0),
        (-71.0, 250.0),
        (89.5, 30.0),
    ] {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        let p = [
            R * la.cos() * lo.cos(),
            R * la.cos() * lo.sin(),
            R * la.sin(),
        ];
        let got = run(&coefficients, p, band, Derivatives::Gradient).potential;
        let (want, scale) = spherical(&coefficients, (200, 360), la, lo, R);
        assert!(scale > 0.0);
        assert!(
            (got - want).abs() <= 1e-11 * scale,
            "{lat} {lon}: {got} vs {want} (scale {scale})"
        );
    }
}

#[test]
fn a_point_deep_inside_the_reference_sphere_is_refused_not_infinite() {
    // (R/r)^361 at r = R/10 is 1e361, beyond f64.
    let coefficients = synthetic(360, 19);
    let mut buffer = vec![0.0; workspace_len(360)];
    let mut ws = Workspace::new(&mut buffer, 360).unwrap();
    let deep = synthesize(
        &mut ws,
        &coefficients,
        [R / 10.0, 0.0, 0.0],
        R,
        Truncation::full(360),
        Derivatives::Gradient,
    );
    assert_eq!(deep, Err(FieldError::InvalidPosition));
}
