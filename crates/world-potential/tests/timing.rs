//! Evaluation cost by degree. Ignored by default; run with
//! `cargo test --release -p world-potential --test timing -- --ignored --nocapture`.

#![cfg(feature = "alloc")]

use std::hint::black_box;
use std::time::Instant;

use world_potential::{
    Derivatives, HarmonicCoefficients, Truncation, WorkspaceBuffer, synthesize, triangular_len,
};

#[test]
#[ignore = "timing, not a check"]
fn evaluation_cost_by_degree() {
    let degree = 360;
    let len = triangular_len(degree);
    let c: Vec<f64> = (0..len).map(|i| 1e-6 / (1.0 + i as f64)).collect();
    let coefficients = HarmonicCoefficients::new(degree, c.clone(), c).unwrap();
    let mut buffer = WorkspaceBuffer::new(degree);
    let mut ws = buffer.workspace().unwrap();
    for d in [12, 70, 360] {
        for derivatives in [Derivatives::Gradient, Derivatives::Hessian] {
            let iterations = 200_000 / (d * d / 100 + 1);
            let start = Instant::now();
            for i in 0..iterations {
                let p = [6.9e6 + i as f64, 1.0e6, 2.0e6];
                black_box(
                    synthesize(
                        &mut ws,
                        &coefficients,
                        black_box(p),
                        6.378e6,
                        Truncation::full(d),
                        derivatives,
                    )
                    .unwrap(),
                );
            }
            let per = start.elapsed().as_secs_f64() / iterations as f64;
            println!("degree {d:>3} {derivatives:?}: {:.2} µs", per * 1e6);
        }
    }
}
