//! Cubic splines through the temperature nodes.
//!
//! The node count never exceeds five, so the working arrays are fixed.

/// The most nodes a temperature profile segment has.
pub(crate) const MAX_NODES: usize = 5;

/// Second derivatives of the spline through the first `n` of `(x, y)`,
/// with end slopes `yp1` and `ypn`; a slope of 1e30 or more asks for a
/// zero second derivative at that end.
pub(crate) fn spline(
    x: &[f64; MAX_NODES],
    y: &[f64; MAX_NODES],
    n: usize,
    yp1: f64,
    ypn: f64,
) -> [f64; MAX_NODES] {
    let mut u = [0.0; MAX_NODES];
    let mut y2 = [0.0; MAX_NODES];
    if yp1 > 0.99E30 {
        y2[0] = 0.0;
        u[0] = 0.0;
    } else {
        y2[0] = -0.5;
        u[0] = (3.0 / (x[1] - x[0])) * ((y[1] - y[0]) / (x[1] - x[0]) - yp1);
    }
    for i in 1..(n - 1) {
        let sig = (x[i] - x[i - 1]) / (x[i + 1] - x[i - 1]);
        let p = sig * y2[i - 1] + 2.0;
        y2[i] = (sig - 1.0) / p;
        u[i] = (6.0
            * ((y[i + 1] - y[i]) / (x[i + 1] - x[i]) - (y[i] - y[i - 1]) / (x[i] - x[i - 1]))
            / (x[i + 1] - x[i - 1])
            - sig * u[i - 1])
            / p;
    }
    let (qn, un) = if ypn > 0.99E30 {
        (0.0, 0.0)
    } else {
        (
            0.5,
            (3.0 / (x[n - 1] - x[n - 2])) * (ypn - (y[n - 1] - y[n - 2]) / (x[n - 1] - x[n - 2])),
        )
    };
    y2[n - 1] = (un - qn * u[n - 2]) / (qn * y2[n - 2] + 1.0);
    for k in (0..(n - 1)).rev() {
        y2[k] = y2[k] * y2[k + 1] + u[k];
    }
    y2
}

/// The spline's value at `x`.
pub(crate) fn splint(
    xa: &[f64; MAX_NODES],
    ya: &[f64; MAX_NODES],
    y2a: &[f64; MAX_NODES],
    n: usize,
    x: f64,
) -> f64 {
    let mut klo = 0;
    let mut khi = n - 1;
    while khi - klo > 1 {
        let k = (khi + klo) / 2;
        if xa[k] > x {
            khi = k;
        } else {
            klo = k;
        }
    }
    let h = xa[khi] - xa[klo];
    let a = (xa[khi] - x) / h;
    let b = (x - xa[klo]) / h;
    a * ya[klo]
        + b * ya[khi]
        + ((a * a * a - a) * y2a[klo] + (b * b * b - b) * y2a[khi]) * h * h / 6.0
}

/// The spline's integral from its first node to `x`.
pub(crate) fn splini(
    xa: &[f64; MAX_NODES],
    ya: &[f64; MAX_NODES],
    y2a: &[f64; MAX_NODES],
    n: usize,
    x: f64,
) -> f64 {
    let mut yi = 0.0;
    for klo in 0..(n - 1) {
        if x <= xa[klo] {
            break;
        }
        let khi = klo + 1;
        let mut xx = x;
        if khi < n - 1 {
            xx = if x < xa[khi] { x } else { xa[khi] };
        }
        let h = xa[khi] - xa[klo];
        let a = (xa[khi] - xx) / h;
        let b = (xx - xa[klo]) / h;
        let a2 = a * a;
        let b2 = b * b;
        yi += ((1.0 - a2) * ya[klo] / 2.0
            + b2 * ya[khi] / 2.0
            + ((-(1.0 + a2 * a2) / 4.0 + a2 / 2.0) * y2a[klo]
                + (b2 * b2 / 4.0 - b2 / 2.0) * y2a[khi])
                * h
                * h
                / 6.0)
            * h;
    }
    yi
}

#[cfg(test)]
mod tests;
