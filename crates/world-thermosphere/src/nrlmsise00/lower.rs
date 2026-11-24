//! The mesosphere, stratosphere and troposphere, below 72.5 km (the lower
//! part of the reference's `GTD7`).

use super::call::{Call, Carry};
use super::coefficients::{PAVGM, PDM, PMA};
use super::globe_lower::glob7s;
use super::profile::{Nodes, densm};
use super::thermosphere::{AMU_G, Upper};

/// The altitude of the mesosphere's upper node, km: the boundary between
/// the two formulations.
pub(crate) const ZN2_TOP: f64 = 72.5;
/// The altitude below which the atmosphere is fully mixed, km.
const ZMIX: f64 = 62.5;

/// The mesosphere nodes, 72.5 km to 32.5 km, joining the thermosphere's
/// lowest node.
fn mesosphere(call: &Call, carry: &Carry, upper: &Upper) -> Nodes<4> {
    let sw = &call.factors.sw;
    let mut tn = [0.0; 4];
    tn[0] = upper.nodes.tn[4];
    tn[1] = PMA[0][0] * PAVGM[0] / (1.0 - sw[20] * glob7s(&PMA[0], call, carry));
    tn[2] = PMA[1][0] * PAVGM[1] / (1.0 - sw[20] * glob7s(&PMA[1], call, carry));
    tn[3] = PMA[2][0] * PAVGM[2] / (1.0 - sw[20] * sw[22] * glob7s(&PMA[2], call, carry));
    let gradient = PAVGM[8]
        * PMA[9][0]
        * (1.0 + sw[20] * sw[22] * glob7s(&PMA[9], call, carry))
        * tn[3]
        * tn[3]
        / ((PMA[2][0] * PAVGM[2]) * (PMA[2][0] * PAVGM[2]));
    Nodes {
        zn: [ZN2_TOP, 55.0, 45.0, 32.5],
        tn,
        tgn: [upper.nodes.tgn[1], gradient],
    }
}

/// The stratosphere and troposphere nodes, 32.5 km to the ground, joining
/// the mesosphere's lowest node. Read only at and below their first node.
fn stratosphere(call: &Call, carry: &Carry, meso: &Nodes<4>) -> Nodes<5> {
    let sw = &call.factors.sw;
    let mut tn = [0.0; 5];
    tn[0] = meso.tn[3];
    tn[1] = PMA[3][0] * PAVGM[3] / (1.0 - sw[22] * glob7s(&PMA[3], call, carry));
    tn[2] = PMA[4][0] * PAVGM[4] / (1.0 - sw[22] * glob7s(&PMA[4], call, carry));
    tn[3] = PMA[5][0] * PAVGM[5] / (1.0 - sw[22] * glob7s(&PMA[5], call, carry));
    tn[4] = PMA[6][0] * PAVGM[6] / (1.0 - sw[22] * glob7s(&PMA[6], call, carry));
    let gradient =
        PMA[7][0] * PAVGM[7] * (1.0 + sw[22] * glob7s(&PMA[7], call, carry)) * tn[4] * tn[4]
            / ((PMA[6][0] * PAVGM[6]) * (PMA[6][0] * PAVGM[6]));
    Nodes {
        zn: [32.5, 20.0, 15.0, 10.0, 0.0],
        tn,
        tgn: [meso.tgn[1], gradient],
    }
}

/// Densities (reference order, cm⁻³ and g/cm³) and temperature at the
/// call's altitude below 72.5 km, from the thermospheric part evaluated at
/// 72.5 km and its fully mixed N2 density `dm28` there.
pub(crate) fn below_mesopause(call: &Call, upper: &Upper, dm28: f64) -> ([f64; 9], f64) {
    let alt = call.alt;
    let gravity = &call.gravity;
    let xmm = PDM[2][4];
    let meso = mesosphere(call, &upper.carry, upper);
    let strato = stratosphere(call, &upper.carry, &meso);
    // The thermosphere's departure from full mixing fades out linearly
    // down to the mixing altitude.
    let dmc = if alt > ZMIX {
        1.0 - (ZN2_TOP - alt) / (ZN2_TOP - ZMIX)
    } else {
        0.0
    };
    let dz28 = upper.d[2];
    let mut d = [0.0; 9];
    let dmr = upper.d[2] / dm28 - 1.0;
    d[2] = densm(alt, dm28, xmm, &meso, &strato, gravity).0;
    d[2] *= 1.0 + dmr * dmc;
    // He, O2 and Ar follow N2 at their ground mixing ratios; O, H, N and
    // anomalous O are not modelled here.
    for (species, ratio) in [(0, PDM[0][1]), (3, PDM[3][1]), (4, PDM[4][1])] {
        let dmr = upper.d[species] / (dz28 * ratio) - 1.0;
        d[species] = d[2] * ratio * (1.0 + dmr * dmc);
    }
    d[5] = AMU_G
        * (4.0 * d[0] + 16.0 * d[1] + 28.0 * d[2] + 32.0 * d[3] + 40.0 * d[4] + d[6] + 14.0 * d[7]);
    let (_, tz) = densm(alt, 1.0, 0.0, &meso, &strato, gravity);
    (d, tz)
}
