//! The horizontal and temporal expansion of a lower-atmosphere parameter
//! (the reference's `GLOB7S`).

use crate::math::{abs, cos, sin};

use super::call::{Call, Carry, DGTR, DR};
use super::globe::Seasonal;

/// The longitudinal term, entry 10.
fn longitudinal(p: &[f64; 100], call: &Call) -> f64 {
    let plg = &call.harmonics.plg;
    let swc = &call.factors.swc;
    (1.0 + plg[0][1]
        * (p[80] * swc[5] * cos(DR * (call.doy - p[81]))
            + p[85] * swc[6] * cos(2.0 * DR * (call.doy - p[86])))
        + p[83] * swc[3] * cos(DR * (call.doy - p[84]))
        + p[87] * swc[4] * cos(2.0 * DR * (call.doy - p[88])))
        * ((p[64] * plg[1][2]
            + p[65] * plg[1][4]
            + p[66] * plg[1][6]
            + p[74] * plg[1][1]
            + p[75] * plg[1][3]
            + p[76] * plg[1][5])
            * cos(DGTR * call.g_long)
            + (p[90] * plg[1][2]
                + p[91] * plg[1][4]
                + p[92] * plg[1][6]
                + p[77] * plg[1][1]
                + p[78] * plg[1][3]
                + p[79] * plg[1][5])
                * sin(DGTR * call.g_long))
}

/// The expansion of the parameter set `p`, with the geomagnetic activity
/// functions as the upper thermosphere left them in `carry`.
pub(crate) fn glob7s(p: &[f64; 100], call: &Call, carry: &Carry) -> f64 {
    let h = &call.harmonics;
    let plg = &h.plg;
    let sw = &call.factors.sw;
    let swc = &call.factors.swc;
    let mut t = [0.0; 14];
    let Seasonal {
        cd32,
        cd18,
        cd14,
        cd39,
    } = Seasonal::new(p[31], p[17], p[13], p[38], call.doy);
    t[0] = p[21] * call.dfa;
    t[1] = p[1] * plg[0][2]
        + p[2] * plg[0][4]
        + p[22] * plg[0][6]
        + p[26] * plg[0][1]
        + p[14] * plg[0][3]
        + p[59] * plg[0][5];
    t[2] = (p[18] + p[47] * plg[0][2] + p[29] * plg[0][4]) * cd32;
    t[3] = (p[15] + p[16] * plg[0][2] + p[30] * plg[0][4]) * cd18;
    t[4] = (p[9] * plg[0][1] + p[10] * plg[0][3] + p[20] * plg[0][5]) * cd14;
    t[5] = (p[37] * plg[0][1]) * cd39;
    if sw[7] != 0.0 {
        let t71 = p[11] * plg[1][2] * cd14 * swc[5];
        let t72 = p[12] * plg[1][2] * cd14 * swc[5];
        t[6] = (p[3] * plg[1][1] + p[4] * plg[1][3] + t71) * h.ctloc
            + (p[6] * plg[1][1] + p[7] * plg[1][3] + t72) * h.stloc;
    }
    if sw[8] != 0.0 {
        let t81 = (p[23] * plg[2][3] + p[35] * plg[2][5]) * cd14 * swc[5];
        let t82 = (p[33] * plg[2][3] + p[36] * plg[2][5]) * cd14 * swc[5];
        t[7] = (p[5] * plg[2][2] + p[41] * plg[2][4] + t81) * h.c2tloc
            + (p[8] * plg[2][2] + p[42] * plg[2][4] + t82) * h.s2tloc;
    }
    if sw[14] != 0.0 {
        t[13] = p[39] * plg[3][3] * h.s3tloc + p[40] * plg[3][3] * h.c3tloc;
    }
    if sw[9] == 1.0 {
        t[8] = carry.apdf * (p[32] + p[45] * plg[0][2] * swc[2]);
    }
    if sw[9] == -1.0 {
        t[8] = p[50] * carry.apt0 + p[96] * plg[0][2] * carry.apt0 * swc[2];
    }
    if sw[10] != 0.0 && sw[11] != 0.0 {
        t[10] = longitudinal(p, call);
    }
    let mut tt = 0.0;
    for (term, sw) in t.iter().zip(sw.iter().skip(1)) {
        tt += abs(*sw) * term;
    }
    tt
}
