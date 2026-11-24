//! Second-derivative tensors of a potential.

use core::ops::{Add, Mul};

/// A symmetric 3×3 tensor in body-fixed Cartesian axes: the second
/// derivatives of a potential (the gravity-gradient tensor, or the
/// gradient of a magnetic field).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SymmetricTensor {
    /// ∂²/∂x².
    pub xx: f64,
    /// ∂²/∂x∂y.
    pub xy: f64,
    /// ∂²/∂x∂z.
    pub xz: f64,
    /// ∂²/∂y².
    pub yy: f64,
    /// ∂²/∂y∂z.
    pub yz: f64,
    /// ∂²/∂z².
    pub zz: f64,
}

impl SymmetricTensor {
    /// The trace. Zero for the Hessian of a potential outside its sources
    /// (Laplace's equation), so it measures synthesis error.
    pub fn trace(&self) -> f64 {
        self.xx + self.yy + self.zz
    }

    /// The tensor as a full row-major matrix.
    pub fn to_matrix(&self) -> [[f64; 3]; 3] {
        [
            [self.xx, self.xy, self.xz],
            [self.xy, self.yy, self.yz],
            [self.xz, self.yz, self.zz],
        ]
    }

    /// The tensor applied to `v`.
    pub fn apply(&self, v: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = v;
        [
            self.xx * x + self.xy * y + self.xz * z,
            self.xy * x + self.yy * y + self.yz * z,
            self.xz * x + self.yz * y + self.zz * z,
        ]
    }
}

impl Add for SymmetricTensor {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            xx: self.xx + other.xx,
            xy: self.xy + other.xy,
            xz: self.xz + other.xz,
            yy: self.yy + other.yy,
            yz: self.yz + other.yz,
            zz: self.zz + other.zz,
        }
    }
}

impl Mul<f64> for SymmetricTensor {
    type Output = Self;

    fn mul(self, k: f64) -> Self {
        Self {
            xx: self.xx * k,
            xy: self.xy * k,
            xz: self.xz * k,
            yy: self.yy * k,
            yz: self.yz * k,
            zz: self.zz * k,
        }
    }
}
