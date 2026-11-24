use super::*;
use crate::workspace_len;

#[test]
fn a_sum_adds_vectors_and_keeps_a_gradient_only_when_both_give_one() {
    let a = FieldSample {
        vector: [1.0, 2.0, 3.0],
        gradient: Some(SymmetricTensor {
            xx: 1.0,
            ..Default::default()
        }),
    };
    let b = FieldSample {
        vector: [10.0, 20.0, 30.0],
        gradient: Some(SymmetricTensor {
            yy: 2.0,
            ..Default::default()
        }),
    };
    let s = a + b;
    assert_eq!(s.vector, [11.0, 22.0, 33.0]);
    assert_eq!(s.gradient.unwrap().trace(), 3.0);
    let partial = a + FieldSample {
        gradient: None,
        ..b
    };
    assert_eq!(partial.gradient, None);
}

#[test]
fn no_field_is_zero_with_a_zero_gradient() {
    let mut buffer = std::vec![0.0; workspace_len(0)];
    let mut ws = Workspace::new(&mut buffer, 0).unwrap();
    let s = NoField
        .sample(
            &mut ws,
            [1.0, 2.0, 3.0],
            DecimalYear(2000.0),
            Derivatives::Hessian,
        )
        .unwrap();
    assert_eq!(s.vector, [0.0; 3]);
    assert_eq!(s.gradient, Some(SymmetricTensor::default()));
}

#[cfg(feature = "alloc")]
#[test]
fn fields_held_as_dyn_compose_in_a_stack() {
    use std::boxed::Box;
    use std::vec;

    use crate::GravityModel;

    const GM: f64 = 3.986_004_418e14;
    const R: f64 = 6_378_137.0;

    let earth = GravityModel::point_mass(GM, R).unwrap();
    let stack = FieldStack {
        layers: vec![
            Box::new(earth.clone()),
            Box::new(NoField),
            Box::new(earth.clone()),
        ],
    };
    assert_eq!(stack.workspace_degree(), 0);
    let mut buffer = std::vec![0.0; workspace_len(stack.workspace_degree())];
    let mut ws = Workspace::new(&mut buffer, stack.workspace_degree()).unwrap();
    let at = DecimalYear(2026.0);
    let p = [R, 0.0, 0.0];
    let single = earth.sample(&mut ws, p, at, Derivatives::Hessian).unwrap();
    let total = stack.sample(&mut ws, p, at, Derivatives::Hessian).unwrap();
    assert_eq!(total.vector[0], 2.0 * single.vector[0]);
    assert_eq!(
        total.gradient.unwrap().xx,
        2.0 * single.gradient.unwrap().xx
    );
    let pair = Sum(&earth, NoField);
    assert_eq!(
        pair.sample(&mut ws, p, at, Derivatives::Gradient)
            .unwrap()
            .vector,
        single.vector
    );
}
