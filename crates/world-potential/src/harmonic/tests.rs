use super::*;

#[test]
fn the_triangle_layout_is_dense_and_ordered() {
    assert_eq!(triangular_len(0), 1);
    assert_eq!(triangular_len(360), 65_341);
    assert_eq!(triangular_index(0, 0), 0);
    assert_eq!(triangular_index(2, 0), 3);
    assert_eq!(triangular_index(12, 12) + 1, triangular_len(12));
}

#[test]
fn storage_of_the_wrong_length_is_refused() {
    assert_eq!(
        HarmonicCoefficients::new(2, &[0.0; 6][..], &[0.0; 5][..]),
        Err(FieldError::CoefficientStorage {
            degree: 2,
            expected: 6,
            cosine: 6,
            sine: 5
        })
    );
    let set = HarmonicCoefficients::new(1, [1.0, 2.0, 3.0], [0.0, 0.0, 4.0]).unwrap();
    assert_eq!(set.coefficient(1, 1), (3.0, 4.0));
    assert_eq!(set.coefficient(1, 2), (0.0, 0.0));
    assert_eq!(set.coefficient(5, 0), (0.0, 0.0));
}

#[test]
fn truncations_outside_their_degree_are_malformed() {
    assert!(Truncation::full(4).is_well_formed());
    assert!(
        !Truncation {
            min_degree: 5,
            degree: 4,
            order: 0
        }
        .is_well_formed()
    );
    assert!(
        !Truncation {
            min_degree: 0,
            degree: 4,
            order: 5
        }
        .is_well_formed()
    );
}
