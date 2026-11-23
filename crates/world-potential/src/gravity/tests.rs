use super::*;
use crate::workspace_len;

const GM: f64 = 3.986_004_418e14;
const R: f64 = 6_378_137.0;
const J2: f64 = 1.082_626_7e-3;

fn points() -> [[f64; 3]; 5] {
    [
        [R + 500e3, 0.0, 0.0],
        [4.0e6, -3.0e6, 4.5e6],
        [0.0, 0.0, R],
        [1.0, -2.0, -(R + 35_786e3)],
        [-2.0e6, 6.1e6, 1.0e5],
    ]
}

fn rel(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let n = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    n(d) / n(b)
}

#[test]
fn a_point_mass_is_gm_over_r_squared_and_its_tidal_tensor() {
    let model = GravityModel::point_mass(GM, R).unwrap();
    let mut buffer = [0.0; workspace_len(0)];
    let mut ws = Workspace::new(&mut buffer, 0).unwrap();
    for p in points() {
        let g = model.gravitation(&mut ws, p, Derivatives::Hessian).unwrap();
        let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        let expected = p.map(|x| -GM * x / (r * r * r));
        // Two recursions' rounding: a few ulps.
        assert!(rel(g.acceleration_mps2, expected) < 1e-14, "{p:?}");
        assert!((g.potential_m2ps2 - GM / r).abs() < 1e-14 * GM / r);
        // Second derivatives go through two derivative recursions; ten
        // times the acceleration's bound is still a few ulps of GM/r³.
        let t = g.gradient_per_s2.unwrap().to_matrix();
        let k = GM / (r * r * r);
        for i in 0..3 {
            for j in 0..3 {
                let delta = if i == j { 1.0 } else { 0.0 };
                let want = k * (3.0 * p[i] * p[j] / (r * r) - delta);
                assert!((t[i][j] - want).abs() < 1e-13 * k, "{p:?} [{i}][{j}]");
            }
        }
    }
}

#[test]
fn an_oblate_body_matches_the_closed_form_j2_acceleration() {
    let model = GravityModel::oblate(GM, R, J2, TideSystem::Unstated).unwrap();
    let mut buffer = std::vec![0.0; workspace_len(2)];
    let mut ws = Workspace::new(&mut buffer, 2).unwrap();
    for p in points() {
        let g = model
            .gravitation(&mut ws, p, Derivatives::Gradient)
            .unwrap();
        let [x, y, z] = p;
        let r = (x * x + y * y + z * z).sqrt();
        let (k, s) = (1.5 * J2 * (R / r) * (R / r), z * z / (r * r));
        let mu = GM / (r * r * r);
        let expected = [
            -mu * x * (1.0 + k * (1.0 - 5.0 * s)),
            -mu * y * (1.0 + k * (1.0 - 5.0 * s)),
            -mu * z * (1.0 + k * (3.0 - 5.0 * s)),
        ];
        // The closed form is exact; agreement is rounding.
        assert!(rel(g.acceleration_mps2, expected) < 1e-14, "{p:?}");
    }
}

#[test]
fn a_minimum_degree_of_two_leaves_only_the_perturbation() {
    let full = GravityModel::oblate(GM, R, J2, TideSystem::Unstated).unwrap();
    let perturbing = full
        .clone()
        .with_truncation(Truncation {
            min_degree: 2,
            degree: 2,
            order: 2,
        })
        .unwrap();
    let central = GravityModel::point_mass(GM, R).unwrap();
    let mut buffer = std::vec![0.0; workspace_len(2)];
    let mut ws = Workspace::new(&mut buffer, 2).unwrap();
    let p = points()[1];
    let a = |m: &GravityModel<[f64; 6]>, ws: &mut Workspace<'_>| {
        m.gravitation(ws, p, Derivatives::Gradient)
            .unwrap()
            .acceleration_mps2
    };
    let c = central
        .gravitation(&mut ws, p, Derivatives::Gradient)
        .unwrap();
    let (f, q) = (a(&full, &mut ws), a(&perturbing, &mut ws));
    // The split is the same terms summed in two groups: one rounding of
    // the total.
    for i in 0..3 {
        assert!((c.acceleration_mps2[i] + q[i] - f[i]).abs() < 1e-15 * f[i].abs().max(1.0));
    }
}

#[test]
fn malformed_constants_and_truncations_are_refused() {
    assert_eq!(
        GravityModel::point_mass(0.0, R),
        Err(FieldError::InvalidModel)
    );
    assert_eq!(
        GravityModel::point_mass(GM, f64::NAN),
        Err(FieldError::InvalidModel)
    );
    assert_eq!(
        GravityModel::oblate(GM, R, f64::INFINITY, TideSystem::Unstated),
        Err(FieldError::InvalidModel)
    );
    let m = GravityModel::point_mass(GM, R).unwrap();
    assert!(matches!(
        m.with_truncation(Truncation::full(3)),
        Err(FieldError::DegreeExceedsModel { .. })
    ));
}
