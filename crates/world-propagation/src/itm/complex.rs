//! The few complex operations the ground-impedance and two-ray terms
//! need; `core` has no complex type and a dependency is not worth it.

use core::ops::{Add, Div, Mul, Sub};

/// A complex number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// |z|.
    pub fn abs(self) -> f64 {
        libm::hypot(self.re, self.im)
    }

    /// |z|².
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// The principal square root (branch cut on the negative real axis).
    pub fn sqrt(self) -> Self {
        let r = self.abs();
        let re = libm::sqrt(0.5 * (r + self.re));
        let im = libm::copysign(libm::sqrt(0.5 * (r - self.re)), self.im);
        Self { re, im }
    }

    /// z scaled by a real factor.
    pub fn scale(self, k: f64) -> Self {
        Self {
            re: self.re * k,
            im: self.im * k,
        }
    }
}

impl Add for Complex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl Div for Complex {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        let d = rhs.norm_sqr();
        Self::new(
            (self.re * rhs.re + self.im * rhs.im) / d,
            (self.im * rhs.re - self.re * rhs.im) / d,
        )
    }
}
