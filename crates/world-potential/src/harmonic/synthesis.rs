//! Synthesis of a harmonic series and its first and second derivatives.
//!
//! The series is evaluated with Cunningham's solid harmonics in
//! body-fixed Cartesian coordinates (Montenbruck & Gill, *Satellite
//! Orbits*, §3.2.4), carried fully normalised so no factorial ever forms:
//!
//! Φ = Σ C̄nm V̄nm + S̄nm W̄nm,  V̄nm + i W̄nm = (R/r)^(n+1) P̄nm(z/r) e^(imλ).
//!
//! The recursion runs in x, y, z, so there is no 1/cos φ and no pole
//! singularity, and it is the stable column recursion of the normalised
//! Legendre functions scaled by powers of R/r. A derivative of a solid
//! harmonic is a combination of at most two solid harmonics one degree
//! up, so the gradient reads one row up and the Hessian two; the normalised
//! factors below follow from Montenbruck & Gill eq. 3.33 by the ratio of
//! normalisations N̄nm / N̄(n+1)m'.

use crate::harmonic::workspace::row_len;
use crate::{CoefficientSource, FieldError, SymmetricTensor, Truncation, Workspace};

/// How many derivatives a synthesis returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Derivatives {
    /// The potential and its gradient.
    Gradient,
    /// The potential, its gradient and its Hessian.
    Hessian,
}

impl Derivatives {
    /// Harmonic rows needed above the highest summed degree.
    fn rows_above(self) -> usize {
        match self {
            Self::Gradient => 1,
            Self::Hessian => 2,
        }
    }
}

/// A synthesised series, dimensionless in the potential: Φ, ∇Φ in 1/m and
/// the Hessian of Φ in 1/m². A gravity model scales it by GM/R, a magnetic
/// one by −a.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Synthesis {
    /// Φ.
    pub potential: f64,
    /// ∇Φ in body-fixed axes.
    pub gradient: [f64; 3],
    /// ∇∇Φ, when [`Derivatives::Hessian`] was asked for.
    pub hessian: Option<SymmetricTensor>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn scale(self, k: f64) -> Self {
        Self {
            re: self.re * k,
            im: self.im * k,
        }
    }

    fn add(self, o: Self) -> Self {
        Self {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }

    /// i·self.
    fn rotate(self) -> Self {
        Self {
            re: -self.im,
            im: self.re,
        }
    }

    /// Re(k·self) for k = c − i s: the cosine and sine terms of one
    /// coefficient pair.
    fn real_with(self, c: f64, s: f64) -> f64 {
        c * self.re + s * self.im
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

/// The three-row ring of solid harmonics and the factor tables.
struct Rows<'w, 'a> {
    ws: &'w mut Workspace<'a>,
    stride: usize,
}

impl Rows<'_, '_> {
    fn sq(&self, k: usize) -> f64 {
        self.ws.sqrt.get(k).copied().unwrap_or(0.0)
    }

    fn isq(&self, k: usize) -> f64 {
        self.ws.inv_sqrt.get(k).copied().unwrap_or(0.0)
    }

    fn slot(&self, n: usize, m: usize) -> usize {
        (n % 3) * self.stride + m
    }

    /// Ēnm; zero below order 0, which only the m − 1 terms of order zero
    /// ask for and multiply by zero.
    fn get(&self, n: usize, m: Option<usize>) -> Complex {
        let Some(m) = m else {
            return Complex::default();
        };
        let i = self.slot(n, m);
        Complex {
            re: self.ws.cos_rows.get(i).copied().unwrap_or(0.0),
            im: self.ws.sin_rows.get(i).copied().unwrap_or(0.0),
        }
    }

    fn set(&mut self, n: usize, m: usize, e: Complex) {
        let i = self.slot(n, m);
        if let Some(v) = self.ws.cos_rows.get_mut(i) {
            *v = e.re;
        }
        if let Some(w) = self.ws.sin_rows.get_mut(i) {
            *w = e.im;
        }
    }

    /// Row `k` for orders `0..=min(k, top_order)`. `p` is (x, y, z)·R/r²
    /// and `q` is R²/r².
    fn fill_row(&mut self, k: usize, top_order: usize, p: [f64; 3], q: f64, r0: f64) {
        if k == 0 {
            self.set(0, 0, Complex { re: r0, im: 0.0 });
            return;
        }
        let [u, v, w] = p;
        for m in 0..=k.min(top_order) {
            let e = if m == k {
                let f = if k == 1 {
                    self.sq(3)
                } else {
                    self.sq(2 * k + 1) * self.isq(2 * k)
                };
                let d = self.get(k - 1, Some(k - 1));
                Complex {
                    re: f * (u * d.re - v * d.im),
                    im: f * (u * d.im + v * d.re),
                }
            } else {
                let a = self.sq(2 * k + 1) * self.sq(2 * k - 1) * self.isq(k - m) * self.isq(k + m);
                let mut e = self.get(k - 1, Some(m)).scale(a * w);
                if m + 2 <= k {
                    let b = self.sq(2 * k + 1)
                        * self.sq(k + m - 1)
                        * self.sq(k - m - 1)
                        * self.isq(2 * k - 3)
                        * self.isq(k + m)
                        * self.isq(k - m);
                    e = e.add(self.get(k - 2, Some(m)).scale(-b * q));
                }
                e
            };
            self.set(k, m, e);
        }
    }

    /// Normalised factors of R·∂Ēnm on Ē(n+1)(m+1), Ē(n+1)(m−1) and
    /// Ē(n+1)m.
    fn factors(&self, n: usize, m: usize) -> Factors {
        let common = self.sq(2 * n + 1) * self.isq(2 * n + 3);
        let up = if m == 0 {
            core::f64::consts::FRAC_1_SQRT_2 * common * self.sq(n + 1) * self.sq(n + 2)
        } else {
            0.5 * common * self.sq(n + m + 1) * self.sq(n + m + 2)
        };
        let down = match m {
            0 => 0.0,
            1 => core::f64::consts::FRAC_1_SQRT_2 * common * self.sq(n) * self.sq(n + 1),
            _ => 0.5 * common * self.sq(n - m + 2) * self.sq(n - m + 1),
        };
        let vertical = -common * self.sq(n + m + 1) * self.sq(n - m + 1);
        Factors { up, down, vertical }
    }

    /// R·∇Ēnm, read from row n + 1. Ē(n)0 is real, so its derivatives
    /// are too; the complex rule would otherwise give them an imaginary
    /// part they lack.
    fn gradient(&self, n: usize, m: usize) -> [Complex; 3] {
        let f = self.factors(n, m);
        [Axis::X, Axis::Y, Axis::Z].map(|axis| {
            let mut sum = Complex::default();
            for (k, mm) in f.operator(axis, m) {
                sum = sum.add(mul(k, self.get(n + 1, mm)));
            }
            real_if_zonal(sum, m)
        })
    }

    /// R²·∇∇Ēnm, read from row n + 2, as (xx, xy, xz, yy, yz, zz).
    fn hessian(&self, n: usize, m: usize) -> [Complex; 6] {
        use Axis::{X, Y, Z};
        let f = self.factors(n, m);
        // The outer operator reaches orders m − 1, m and m + 1 of degree
        // n + 1; their gradients are shared by all six components.
        let below = m.checked_sub(1).map(|mm| self.gradient(n + 1, mm));
        let inner = [
            below,
            Some(self.gradient(n + 1, m)),
            Some(self.gradient(n + 1, m + 1)),
        ];
        let component = |a: Axis, b: usize| {
            let mut sum = Complex::default();
            for (k, mm) in f.operator(a, m) {
                let g = mm.and_then(|mm| inner.get(mm + 1 - m).copied().flatten());
                if let Some(g) = g.and_then(|g| g.get(b).copied()) {
                    sum = sum.add(mul(k, g));
                }
            }
            real_if_zonal(sum, m)
        };
        [
            component(X, 0),
            component(X, 1),
            component(X, 2),
            component(Y, 1),
            component(Y, 2),
            component(Z, 2),
        ]
    }
}

#[derive(Clone, Copy)]
struct Factors {
    up: f64,
    down: f64,
    vertical: f64,
}

impl Factors {
    /// The terms of R·∂/∂axis Ēnm as (factor, order) on degree n + 1.
    fn operator(self, axis: Axis, m: usize) -> [(Complex, Option<usize>); 2] {
        let real = |k: f64| Complex { re: k, im: 0.0 };
        let below = m.checked_sub(1);
        match axis {
            Axis::X => [(real(-self.up), Some(m + 1)), (real(self.down), below)],
            Axis::Y => [
                (real(self.up).rotate(), Some(m + 1)),
                (real(self.down).rotate(), below),
            ],
            Axis::Z => [(real(self.vertical), Some(m)), (real(0.0), None)],
        }
    }
}

fn mul(a: Complex, b: Complex) -> Complex {
    Complex {
        re: a.re * b.re - a.im * b.im,
        im: a.re * b.im + a.im * b.re,
    }
}

fn real_if_zonal(e: Complex, m: usize) -> Complex {
    if m == 0 {
        Complex { re: e.re, im: 0.0 }
    } else {
        e
    }
}

/// Sums `coefficients` at `position_m` (body-fixed Cartesian, metres) for
/// reference radius `radius_m`, over `truncation`.
///
/// # Errors
///
/// [`FieldError::InvalidTruncation`], [`FieldError::DegreeExceedsModel`],
/// [`FieldError::DegreeExceedsWorkspace`], [`FieldError::InvalidModel`] for
/// a non-finite or non-positive radius, and [`FieldError::InvalidPosition`]
/// for a non-finite position, the centre, or a point so far inside the
/// reference sphere that the series overflows.
pub fn synthesize<C: CoefficientSource + ?Sized>(
    workspace: &mut Workspace<'_>,
    coefficients: &C,
    position_m: [f64; 3],
    radius_m: f64,
    truncation: Truncation,
    derivatives: Derivatives,
) -> Result<Synthesis, FieldError> {
    check(workspace, coefficients, radius_m, truncation)?;
    let [x, y, z] = position_m;
    let r2 = x * x + y * y + z * z;
    if !r2.is_finite() || r2 <= 0.0 {
        return Err(FieldError::InvalidPosition);
    }
    let rho = radius_m / r2;
    let p = [x * rho, y * rho, z * rho];
    let q = radius_m * rho;
    let r0 = radius_m / libm::sqrt(r2);

    let extra = derivatives.rows_above();
    let top_order = truncation.order + extra;
    let stride = row_len(workspace.max_degree);
    let mut rows = Rows {
        ws: workspace,
        stride,
    };
    let mut acc = Accumulator::default();
    for k in 0..=truncation.degree + extra {
        rows.fill_row(k, top_order, p, q, r0);
        let Some(n) = k.checked_sub(extra) else {
            continue;
        };
        if n >= truncation.min_degree {
            acc.degree(&rows, coefficients, n, truncation.order, derivatives);
        }
    }
    let inv_r = 1.0 / radius_m;
    let synthesis = Synthesis {
        potential: acc.potential,
        gradient: acc.gradient.map(|g| g * inv_r),
        hessian: match derivatives {
            Derivatives::Gradient => None,
            Derivatives::Hessian => Some(acc.hessian * (inv_r * inv_r)),
        },
    };
    // (R/r)^(n+1) overflows far inside the reference sphere at high
    // degree, where the series diverges anyway.
    let finite = synthesis.potential.is_finite()
        && synthesis.gradient.iter().all(|g| g.is_finite())
        && synthesis
            .hessian
            .is_none_or(|h| h.to_matrix().iter().flatten().all(|v| v.is_finite()));
    if finite {
        Ok(synthesis)
    } else {
        Err(FieldError::InvalidPosition)
    }
}

fn check<C: CoefficientSource + ?Sized>(
    workspace: &Workspace<'_>,
    coefficients: &C,
    radius_m: f64,
    truncation: Truncation,
) -> Result<(), FieldError> {
    if !truncation.is_well_formed() {
        return Err(FieldError::InvalidTruncation);
    }
    if truncation.degree > coefficients.max_degree() {
        return Err(FieldError::DegreeExceedsModel {
            degree: truncation.degree,
            available: coefficients.max_degree(),
        });
    }
    if truncation.degree > workspace.max_degree {
        return Err(FieldError::DegreeExceedsWorkspace {
            degree: truncation.degree,
            capacity: workspace.max_degree,
        });
    }
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return Err(FieldError::InvalidModel);
    }
    Ok(())
}

#[derive(Default)]
struct Accumulator {
    potential: f64,
    gradient: [f64; 3],
    hessian: SymmetricTensor,
}

impl Accumulator {
    fn degree<C: CoefficientSource + ?Sized>(
        &mut self,
        rows: &Rows<'_, '_>,
        coefficients: &C,
        n: usize,
        order: usize,
        derivatives: Derivatives,
    ) {
        for m in 0..=n.min(order) {
            let (c, s) = coefficients.coefficient(n, m);
            let s = if m == 0 { 0.0 } else { s };
            if c == 0.0 && s == 0.0 {
                continue;
            }
            self.potential += rows.get(n, Some(m)).real_with(c, s);
            for (g, e) in self.gradient.iter_mut().zip(rows.gradient(n, m)) {
                *g += e.real_with(c, s);
            }
            if derivatives == Derivatives::Hessian {
                let [xx, xy, xz, yy, yz, zz] = rows.hessian(n, m).map(|e| e.real_with(c, s));
                self.hessian = self.hessian
                    + SymmetricTensor {
                        xx,
                        xy,
                        xz,
                        yy,
                        yz,
                        zz,
                    };
            }
        }
    }
}

#[cfg(test)]
mod tests;
