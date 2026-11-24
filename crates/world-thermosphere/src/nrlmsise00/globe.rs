//! The horizontal and temporal expansion of an upper-thermosphere
//! parameter (the reference's `GLOBE7`).

use crate::math::{abs, cos, exp, pow, sin};

use super::call::{Call, Carry, DGTR, DR, HR, SR};

/// The seasonal carriers of one parameter set.
pub(crate) struct Seasonal {
    /// Annual, symmetrical phase.
    pub(crate) cd32: f64,
    /// Semiannual, symmetrical phase.
    pub(crate) cd18: f64,
    /// Annual, asymmetrical phase.
    pub(crate) cd14: f64,
    /// Semiannual, asymmetrical phase.
    pub(crate) cd39: f64,
}

impl Seasonal {
    pub(crate) fn new(p31: f64, p17: f64, p13: f64, p38: f64, doy: f64) -> Self {
        Self {
            cd32: cos(DR * (doy - p31)),
            cd18: cos(2.0 * DR * (doy - p17)),
            cd14: cos(DR * (doy - p13)),
            cd39: cos(2.0 * DR * (doy - p38)),
        }
    }
}

/// The 3-hour ap response (reference equation A24d).
fn g0(a: f64, p24: f64, p25: f64) -> f64 {
    a - 4.0 + (p25 - 1.0) * (a - 4.0 + (exp(-abs(p24) * (a - 4.0)) - 1.0) / abs(p24))
}

/// The normalization of the ap history weights (reference equation A24c).
fn sumex(ex: f64) -> f64 {
    1.0 + (1.0 - pow(ex, 19.0)) / (1.0 - ex) * pow(ex, 0.5)
}

/// The weighted ap history (reference equation A24a).
fn sg0(ex: f64, p24: f64, p25: f64, ap: &[f64; 7]) -> f64 {
    let g = |a| g0(a, p24, p25);
    (g(ap[1])
        + (g(ap[2]) * ex
            + g(ap[3]) * ex * ex
            + g(ap[4]) * pow(ex, 3.0)
            + (g(ap[5]) * pow(ex, 4.0) + g(ap[6]) * pow(ex, 12.0)) * (1.0 - pow(ex, 8.0))
                / (1.0 - ex)))
        / sumex(ex)
}

/// The solar-flux, latitude and seasonal terms: entries 0 to 5.
fn mean_and_seasonal(p: &[f64; 150], call: &Call, season: &Seasonal, t: &mut [f64; 14]) -> f64 {
    let plg = &call.harmonics.plg;
    let swc = &call.factors.swc;
    let dfa = call.dfa;
    let df = call.f107 - call.f107a;
    t[0] = p[19] * df * (1.0 + p[59] * dfa) + p[20] * df * df + p[21] * dfa + p[29] * dfa * dfa;
    let f1 = 1.0 + (p[47] * dfa + p[19] * df + p[20] * df * df) * swc[1];
    let f2 = 1.0 + (p[49] * dfa + p[19] * df + p[20] * df * df) * swc[1];
    t[1] = (p[1] * plg[0][2] + p[2] * plg[0][4] + p[22] * plg[0][6])
        + (p[14] * plg[0][2]) * dfa * swc[1]
        + p[26] * plg[0][1];
    t[2] = p[18] * season.cd32;
    t[3] = (p[15] + p[16] * plg[0][2]) * season.cd18;
    t[4] = f1 * (p[9] * plg[0][1] + p[10] * plg[0][3]) * season.cd14;
    t[5] = p[37] * plg[0][1] * season.cd39;
    f2
}

/// The diurnal, semidiurnal and terdiurnal tides: entries 6, 7 and 13.
fn tides(p: &[f64; 150], call: &Call, cd14: f64, f2: f64, t: &mut [f64; 14]) {
    let h = &call.harmonics;
    let plg = &h.plg;
    let sw = &call.factors.sw;
    let swc = &call.factors.swc;
    if sw[7] != 0.0 {
        let t71 = (p[11] * plg[1][2]) * cd14 * swc[5];
        let t72 = (p[12] * plg[1][2]) * cd14 * swc[5];
        t[6] = f2
            * ((p[3] * plg[1][1] + p[4] * plg[1][3] + p[27] * plg[1][5] + t71) * h.ctloc
                + (p[6] * plg[1][1] + p[7] * plg[1][3] + p[28] * plg[1][5] + t72) * h.stloc);
    }
    if sw[8] != 0.0 {
        let t81 = (p[23] * plg[2][3] + p[35] * plg[2][5]) * cd14 * swc[5];
        let t82 = (p[33] * plg[2][3] + p[36] * plg[2][5]) * cd14 * swc[5];
        t[7] = f2
            * ((p[5] * plg[2][2] + p[41] * plg[2][4] + t81) * h.c2tloc
                + (p[8] * plg[2][2] + p[42] * plg[2][4] + t82) * h.s2tloc);
    }
    if sw[14] != 0.0 {
        t[13] = f2
            * ((p[39] * plg[3][3] + (p[93] * plg[3][4] + p[46] * plg[3][6]) * cd14 * swc[5])
                * h.s3tloc
                + (p[40] * plg[3][3] + (p[94] * plg[3][4] + p[48] * plg[3][6]) * cd14 * swc[5])
                    * h.c3tloc);
    }
}

/// Geomagnetic activity, entry 8, leaving the set's activity function in
/// `carry`.
fn geomagnetic(p: &[f64; 150], call: &Call, cd14: f64, carry: &mut Carry, t: &mut [f64; 14]) {
    let plg = &call.harmonics.plg;
    let sw = &call.factors.sw;
    let swc = &call.factors.swc;
    if sw[9] == -1.0 {
        if p[51] == 0.0 {
            return;
        }
        let exp1 = exp(-10800.0 * abs(p[51]) / (1.0 + p[138] * (45.0 - abs(call.g_lat))));
        let exp1 = if exp1 > 0.99999 { 0.99999 } else { exp1 };
        let p24 = if p[24] < 1.0E-4 { 1.0E-4 } else { p[24] };
        carry.apt0 = sg0(exp1, p24, p[25], &call.ap_history);
        t[8] = carry.apt0
            * (p[50]
                + p[96] * plg[0][2]
                + p[54] * plg[0][4]
                + (p[125] * plg[0][1] + p[126] * plg[0][3] + p[127] * plg[0][5]) * cd14 * swc[5]
                + (p[128] * plg[1][1] + p[129] * plg[1][3] + p[130] * plg[1][5])
                    * swc[7]
                    * cos(HR * (call.lst - p[131])));
        return;
    }
    let apd = call.ap - 4.0;
    let p44 = if p[43] < 0.0 { 1.0E-5 } else { p[43] };
    let p45 = p[44];
    carry.apdf = apd + (p45 - 1.0) * (apd + (exp(-p44 * apd) - 1.0) / p44);
    if sw[9] != 0.0 {
        t[8] = carry.apdf
            * (p[32]
                + p[45] * plg[0][2]
                + p[34] * plg[0][4]
                + (p[100] * plg[0][1] + p[101] * plg[0][3] + p[102] * plg[0][5]) * cd14 * swc[5]
                + (p[121] * plg[1][1] + p[122] * plg[1][3] + p[123] * plg[1][5])
                    * swc[7]
                    * cos(HR * (call.lst - p[124])));
    }
}

/// Longitude and universal time: entries 10 and 11.
fn longitude_and_ut(p: &[f64; 150], call: &Call, cd14: f64, t: &mut [f64; 14]) {
    let plg = &call.harmonics.plg;
    let sw = &call.factors.sw;
    let swc = &call.factors.swc;
    let dfa = call.dfa;
    if sw[11] != 0.0 {
        t[10] = (1.0 + p[80] * dfa * swc[1])
            * ((p[64] * plg[1][2]
                + p[65] * plg[1][4]
                + p[66] * plg[1][6]
                + p[103] * plg[1][1]
                + p[104] * plg[1][3]
                + p[105] * plg[1][5]
                + swc[5] * (p[109] * plg[1][1] + p[110] * plg[1][3] + p[111] * plg[1][5]) * cd14)
                * cos(DGTR * call.g_long)
                + (p[90] * plg[1][2]
                    + p[91] * plg[1][4]
                    + p[92] * plg[1][6]
                    + p[106] * plg[1][1]
                    + p[107] * plg[1][3]
                    + p[108] * plg[1][5]
                    + swc[5]
                        * (p[112] * plg[1][1] + p[113] * plg[1][3] + p[114] * plg[1][5])
                        * cd14)
                    * sin(DGTR * call.g_long));
    }
    if sw[12] != 0.0 {
        t[11] = (1.0 + p[95] * plg[0][1])
            * (1.0 + p[81] * dfa * swc[1])
            * (1.0 + p[119] * plg[0][1] * swc[5] * cd14)
            * ((p[68] * plg[0][1] + p[69] * plg[0][3] + p[70] * plg[0][5])
                * cos(SR * (call.sec - p[71])));
        t[11] += swc[11]
            * (p[76] * plg[2][3] + p[77] * plg[2][5] + p[78] * plg[2][7])
            * cos(SR * (call.sec - p[79]) + 2.0 * DGTR * call.g_long)
            * (1.0 + p[137] * dfa * swc[1]);
    }
}

/// Mixed geomagnetic activity, longitude and universal time: entry 12.
fn geomagnetic_mixed(p: &[f64; 150], call: &Call, cd14: f64, carry: &Carry, t: &mut [f64; 14]) {
    let plg = &call.harmonics.plg;
    let sw = &call.factors.sw;
    let swc = &call.factors.swc;
    if sw[9] == -1.0 {
        if p[51] != 0.0 {
            let apt = carry.apt0;
            t[12] = apt
                * swc[11]
                * (1. + p[132] * plg[0][1])
                * ((p[52] * plg[1][2] + p[98] * plg[1][4] + p[67] * plg[1][6])
                    * cos(DGTR * (call.g_long - p[97])))
                + apt
                    * swc[11]
                    * swc[5]
                    * (p[133] * plg[1][1] + p[134] * plg[1][3] + p[135] * plg[1][5])
                    * cd14
                    * cos(DGTR * (call.g_long - p[136]))
                + apt
                    * swc[12]
                    * (p[55] * plg[0][1] + p[56] * plg[0][3] + p[57] * plg[0][5])
                    * cos(SR * (call.sec - p[58]));
        }
        return;
    }
    let apdf = carry.apdf;
    t[12] = apdf
        * swc[11]
        * (1.0 + p[120] * plg[0][1])
        * ((p[60] * plg[1][2] + p[61] * plg[1][4] + p[62] * plg[1][6])
            * cos(DGTR * (call.g_long - p[63])))
        + apdf
            * swc[11]
            * swc[5]
            * (p[115] * plg[1][1] + p[116] * plg[1][3] + p[117] * plg[1][5])
            * cd14
            * cos(DGTR * (call.g_long - p[118]))
        + apdf
            * swc[12]
            * (p[83] * plg[0][1] + p[84] * plg[0][3] + p[85] * plg[0][5])
            * cos(SR * (call.sec - p[75]));
}

/// The expansion `G(L)` of the parameter set `p` at the call's position,
/// time and activity. Parameters 82, 89, 99 and 139 to 149 are unused.
pub(crate) fn globe7(p: &[f64; 150], call: &Call, carry: &mut Carry) -> f64 {
    let sw = &call.factors.sw;
    let mut t = [0.0; 14];
    let season = Seasonal::new(p[31], p[17], p[13], p[38], call.doy);
    let f2 = mean_and_seasonal(p, call, &season, &mut t);
    tides(p, call, season.cd14, f2, &mut t);
    geomagnetic(p, call, season.cd14, carry, &mut t);
    if sw[10] != 0.0 {
        longitude_and_ut(p, call, season.cd14, &mut t);
        if sw[13] != 0.0 {
            geomagnetic_mixed(p, call, season.cd14, carry, &mut t);
        }
    }
    let mut tinf = p[30];
    for (term, sw) in t.iter().zip(sw.iter().skip(1)) {
        tinf += abs(*sw) * term;
    }
    tinf
}
