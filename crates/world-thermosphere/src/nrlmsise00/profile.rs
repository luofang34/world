//! Temperature and density profiles along the vertical.
//!
//! Above the 120 km lower boundary the temperature is a Bates profile and
//! each species is in diffusive equilibrium with it; below, the inverse
//! temperature is a cubic spline in geopotential height through fitted
//! nodes, integrated hydrostatically.

use crate::math::{exp, ln, pow, sq};

use super::call::Gravity;
use super::spline::{MAX_NODES, spline, splini, splint};

/// Gas constant in the reference's units: the scale height comes out in
/// km from K, cm/s² and atomic mass units.
pub(crate) const RGAS: f64 = 831.4;

/// The temperature nodes of one spline segment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Nodes<const N: usize> {
    /// Node altitudes, km, descending.
    pub(crate) zn: [f64; N],
    /// Node temperatures, K.
    pub(crate) tn: [f64; N],
    /// Temperature gradients at the first and last node, K/km.
    pub(crate) tgn: [f64; 2],
}

/// One segment's inverse-temperature spline, evaluated at an altitude.
struct Segment {
    xs: [f64; MAX_NODES],
    ys: [f64; MAX_NODES],
    y2: [f64; MAX_NODES],
    n: usize,
    /// The altitude as a fraction of the segment's geopotential span.
    x: f64,
    /// The segment's geopotential span, km.
    zgdif: f64,
    /// The first node's altitude, km.
    z1: f64,
    /// The first node's temperature, K.
    t1: f64,
}

impl Segment {
    fn new<const N: usize>(z: f64, nodes: &Nodes<N>, gravity: &Gravity) -> Self {
        let z1 = nodes.zn[0];
        let z2 = nodes.zn[N - 1];
        let t1 = nodes.tn[0];
        let t2 = nodes.tn[N - 1];
        let zg = gravity.zeta(z, z1);
        let zgdif = gravity.zeta(z2, z1);
        let mut xs = [0.0; MAX_NODES];
        let mut ys = [0.0; MAX_NODES];
        for k in 0..N {
            xs[k] = gravity.zeta(nodes.zn[k], z1) / zgdif;
            ys[k] = 1.0 / nodes.tn[k];
        }
        let yd1 = -nodes.tgn[0] / (t1 * t1) * zgdif;
        let yd2 = -nodes.tgn[1] / (t2 * t2) * zgdif * sq((gravity.re + z2) / (gravity.re + z1));
        let y2 = spline(&xs, &ys, N, yd1, yd2);
        Self {
            xs,
            ys,
            y2,
            n: N,
            x: zg / zgdif,
            zgdif,
            z1,
            t1,
        }
    }

    fn temperature(&self) -> f64 {
        1.0 / splint(&self.xs, &self.ys, &self.y2, self.n, self.x)
    }

    /// The hydrostatic exponent for a species of mass `xm` from the first
    /// node down to the altitude, capped as the reference caps it.
    fn exponent(&self, xm: f64, gravity: &Gravity) -> f64 {
        let glb = gravity.gsurf / sq(1.0 + self.z1 / gravity.re);
        let gamm = xm * glb * self.zgdif / RGAS;
        let expl = gamm * splini(&self.xs, &self.ys, &self.y2, self.n, self.x);
        if expl > 50.0 { 50.0 } else { expl }
    }
}

/// Chemistry and dissociation correction: `exp(r)` well below `zh`,
/// falling to 1 above it over the scale length `h1`.
pub(crate) fn ccor(alt: f64, r: f64, h1: f64, zh: f64) -> f64 {
    let e = (alt - zh) / h1;
    if e > 70.0 {
        return 1.0;
    }
    if e < -70.0 {
        return exp(r);
    }
    exp(r / (1.0 + exp(e)))
}

/// [`ccor`] with two scale lengths.
pub(crate) fn ccor2(alt: f64, r: f64, h1: f64, zh: f64, h2: f64) -> f64 {
    let e1 = (alt - zh) / h1;
    let e2 = (alt - zh) / h2;
    if e1 > 70.0 || e2 > 70.0 {
        return 1.0;
    }
    if e1 < -70.0 && e2 < -70.0 {
        return exp(r);
    }
    exp(r / (1.0 + 0.5 * (exp(e1) + exp(e2))))
}

/// Scale height, km, of a species of mass `xm` at `alt` km and `temp` K.
pub(crate) fn scalh(alt: f64, xm: f64, temp: f64, gravity: &Gravity) -> f64 {
    let g = gravity.gsurf / sq(1.0 + alt / gravity.re);
    RGAS * temp / (g * xm)
}

/// Turbopause correction: the root-mean combination of the diffusive
/// density `dd` and the fully mixed density `dm`.
pub(crate) fn dnet(dd: f64, dm: f64, zhm: f64, xmm: f64, xm: f64) -> f64 {
    let a = zhm / (xmm - xm);
    if !(dm > 0.0 && dd > 0.0) {
        if dd == 0.0 && dm == 0.0 {
            return 1.0;
        }
        if dm == 0.0 {
            return dd;
        }
        if dd == 0.0 {
            return dm;
        }
    }
    let ylog = a * ln(dm / dd);
    if ylog < -10.0 {
        return dd;
    }
    if ylog > 10.0 {
        return dm;
    }
    dd * pow(1.0 + exp(ylog), 1.0 / a)
}

/// The Bates temperature profile above the lower boundary `zlb`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bates {
    /// Exospheric temperature, K.
    pub(crate) tinf: f64,
    /// Temperature at the lower boundary, K.
    pub(crate) tlb: f64,
    /// Shape parameter, 1/km.
    pub(crate) s2: f64,
    /// Lower-boundary altitude, km.
    pub(crate) zlb: f64,
}

/// Thermospheric temperature and density at `alt`.
///
/// Returns `(density, temperature)`: the density of a species of mass `xm`
/// and thermal diffusion coefficient `alpha` whose density at the lower
/// boundary is `dlb`. With `xm` zero the first value is the temperature.
/// The first node of `nodes` is the joining altitude; its temperature and
/// gradient are set here from the Bates profile, as the spline below it
/// must continue that profile.
pub(crate) fn densu(
    alt: f64,
    dlb: f64,
    bates: &Bates,
    xm: f64,
    alpha: f64,
    nodes: &mut Nodes<5>,
    gravity: &Gravity,
) -> (f64, f64) {
    let Bates { tinf, tlb, s2, zlb } = *bates;
    let za = nodes.zn[0];
    let z = if alt > za { alt } else { za };
    let zg2 = gravity.zeta(z, zlb);
    let tt = tinf - (tinf - tlb) * exp(-s2 * zg2);
    let ta = tt;
    let mut tz = tt;
    let mut segment = None;
    if alt < za {
        nodes.tgn[0] = (tinf - ta) * s2 * sq((gravity.re + zlb) / (gravity.re + za));
        nodes.tn[0] = ta;
        let lowest = nodes.zn[4];
        let below = Segment::new(if alt > lowest { alt } else { lowest }, nodes, gravity);
        tz = below.temperature();
        segment = Some(below);
    }
    if xm == 0.0 {
        return (tz, tz);
    }
    let glb = gravity.gsurf / sq(1.0 + zlb / gravity.re);
    let gamma = xm * glb / (s2 * RGAS * tinf);
    let mut expl = exp(-s2 * gamma * zg2);
    if expl > 50.0 || tt <= 0.0 {
        expl = 50.0;
    }
    let densa = dlb * pow(tlb / tt, 1.0 + alpha + gamma) * expl;
    let Some(below) = segment else {
        return (densa, tz);
    };
    let mut expl = below.exponent(xm, gravity);
    if tz <= 0.0 {
        expl = 50.0;
    }
    (densa * pow(below.t1 / tz, 1.0 + alpha) * exp(-expl), tz)
}

/// Temperature and density at `alt` below the mesosphere's upper node.
///
/// Returns `(density, temperature)` for a gas of mass `xm` whose density
/// at the upper node of `meso` is `d0`; with `xm` zero the first value is
/// the temperature. `meso` spans 72.5 km to 32.5 km and `strato` continues
/// it to the ground; `alt` must not be above the first node of `meso`.
pub(crate) fn densm(
    alt: f64,
    d0: f64,
    xm: f64,
    meso: &Nodes<4>,
    strato: &Nodes<5>,
    gravity: &Gravity,
) -> (f64, f64) {
    let mut density = d0;
    let lowest = meso.zn[3];
    let upper = Segment::new(if alt > lowest { alt } else { lowest }, meso, gravity);
    let mut tz = upper.temperature();
    if xm != 0.0 {
        density = density * (upper.t1 / tz) * exp(-upper.exponent(xm, gravity));
    }
    if alt <= strato.zn[0] {
        let lower = Segment::new(alt, strato, gravity);
        tz = lower.temperature();
        if xm != 0.0 {
            density = density * (lower.t1 / tz) * exp(-lower.exponent(xm, gravity));
        }
    }
    if xm == 0.0 { (tz, tz) } else { (density, tz) }
}

#[cfg(test)]
mod tests;
