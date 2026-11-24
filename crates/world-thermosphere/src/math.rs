//! The scalar functions the models are written in.
//!
//! Every model reaches its transcendental functions through this module
//! and nowhere else, so the scalar type can be widened (to carry
//! derivatives, say) by changing these signatures alone, and so every
//! target evaluates the same pure-Rust `libm` routines.

pub(crate) fn exp(x: f64) -> f64 {
    libm::exp(x)
}

pub(crate) fn ln(x: f64) -> f64 {
    libm::log(x)
}

pub(crate) fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub(crate) fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub(crate) fn abs(x: f64) -> f64 {
    libm::fabs(x)
}

pub(crate) fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

pub(crate) fn sq(x: f64) -> f64 {
    x * x
}

#[cfg(feature = "alloc")]
pub(crate) fn floor(x: f64) -> f64 {
    libm::floor(x)
}
