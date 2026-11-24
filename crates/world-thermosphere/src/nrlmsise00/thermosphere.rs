//! The thermosphere and upper mesosphere, above 72.5 km (the reference's
//! `GTS7`).

use crate::math::{abs, cos, exp, ln, sin};

use super::call::{Call, Carry, DGTR, DR};
use super::coefficients::{PD, PDL, PDM, PMA, PS, PT, PTL, PTM};
use super::globe::globe7;
use super::globe_lower::glob7s;
use super::profile::{Bates, Nodes, ccor, ccor2, densu, dnet, scalh};

/// Mass of one atomic mass unit, g, as the reference rounds it.
pub(crate) const AMU_G: f64 = 1.66E-24;

/// What the thermospheric part hands on.
pub(crate) struct Upper {
    /// Number densities, cm⁻³, and at index 5 the mass density, g/cm³, in
    /// the reference's order: He, O, N2, O2, Ar, mass, H, N, anomalous O.
    pub(crate) d: [f64; 9],
    /// Exospheric temperature, K.
    pub(crate) tinf: f64,
    /// Temperature at the altitude, K.
    pub(crate) tz: f64,
    /// The fully mixed N2 density at the altitude, cm⁻³; `None` when the
    /// turbopause mixing was not evaluated.
    pub(crate) dm28: Option<f64>,
    /// The lower-thermosphere temperature nodes.
    pub(crate) nodes: Nodes<5>,
    /// The activity functions, for the lower atmosphere.
    pub(crate) carry: Carry,
}

/// The state every species' profile shares.
struct Column<'a> {
    call: &'a Call,
    carry: Carry,
    /// Altitude, km.
    z: f64,
    bates: Bates,
    nodes: Nodes<5>,
    /// Mean molecular mass of the mixed atmosphere.
    xmm: f64,
    /// Turbopause transition scale length, km.
    zhm28: f64,
    /// Mixed N2 density at its turbopause, which fixes every other
    /// species' ground mixing ratio.
    b28: f64,
    /// Whether departures from diffusive equilibrium are evaluated.
    mixing: bool,
}

impl Column<'_> {
    fn densu(&mut self, alt: f64, dlb: f64, xm: f64, alpha: f64) -> f64 {
        let gravity = &self.call.gravity;
        densu(alt, dlb, &self.bates, xm, alpha, &mut self.nodes, gravity).0
    }

    /// Diffusive density at the lower boundary of the species whose
    /// variations are parameter set `set` about the average `average`.
    fn boundary(&mut self, set: usize, average: f64) -> f64 {
        let sw = self.call.factors.sw;
        let g = sw[21] * globe7(&PD[set], self.call, &mut self.carry);
        average * exp(g) * PD[set][0]
    }

    /// The net of diffusive and mixed density at the altitude, and the
    /// mixed density at the species' turbopause `zh`.
    fn mixed(&mut self, diffusive: f64, db: f64, zh: f64, xm: f64, alpha: f64) -> (f64, f64) {
        let xmm = self.xmm;
        let b = self.densu(zh, db, xm - xmm, alpha - 1.0);
        let dm = self.densu(self.z, b, xmm, 0.0);
        (dnet(diffusive, dm, self.zhm28, xmm, xm), b)
    }

    /// The factor the solar flux puts on the O and O2 corrections.
    fn flux_factor(&self) -> f64 {
        1.0 + self.call.factors.sw[1] * PDL[0][23] * (self.call.f107a - 150.0)
    }

    /// N2, which also fixes the turbopause scale and mixing reference.
    fn molecular_nitrogen(&mut self, zhf: f64) -> (f64, Option<f64>) {
        let z = self.z;
        let db28 = self.boundary(2, PDM[2][0]);
        let mut d = self.densu(z, db28, 28.0, 0.0);
        let zh28 = PDM[2][2] * zhf;
        self.zhm28 = PDM[2][3] * PDL[1][5];
        let xmd = 28.0 - self.xmm;
        self.b28 = self.densu(zh28, db28, xmd, -1.0);
        let mut dm28 = None;
        if self.mixing && z <= 160.0 {
            let mixed = self.densu(z, self.b28, self.xmm, 0.0);
            d = dnet(d, mixed, self.zhm28, self.xmm, 28.0);
            dm28 = Some(mixed);
        }
        (d, dm28)
    }

    fn helium(&mut self) -> f64 {
        let z = self.z;
        let alpha = -0.38;
        let db04 = self.boundary(0, PDM[0][0]);
        let mut d = self.densu(z, db04, 4.0, alpha);
        // At the limit altitude itself the mixing applies, as the Fortran
        // original has it and as for every other species; the C
        // translation excludes exactly 200 km.
        if self.mixing && z <= 200.0 {
            let (net, b04) = self.mixed(d, db04, PDM[0][2], 4.0, alpha);
            let rl = ln(self.b28 * PDM[0][1] / b04);
            let zc04 = PDM[0][4] * PDL[1][0];
            let hc04 = PDM[0][5] * PDL[1][1];
            d = net * ccor(z, rl, hc04, zc04);
        }
        d
    }

    fn atomic_oxygen(&mut self) -> f64 {
        let z = self.z;
        let db16 = self.boundary(1, PDM[1][0]);
        let mut d = self.densu(z, db16, 16.0, 0.0);
        if self.mixing && z <= 300.0 {
            let (net, _) = self.mixed(d, db16, PDM[1][2], 16.0, 0.0);
            let rl = PDM[1][1] * PDL[1][16] * self.flux_factor();
            let hc16 = PDM[1][5] * PDL[1][3];
            let zc16 = PDM[1][4] * PDL[1][2];
            let hc216 = PDM[1][5] * PDL[1][4];
            d = net * ccor2(z, rl, hc16, zc16, hc216);
            let hcc16 = PDM[1][7] * PDL[1][13];
            let zcc16 = PDM[1][6] * PDL[1][12];
            let rc16 = PDM[1][3] * PDL[1][14];
            d *= ccor(z, rc16, hcc16, zcc16);
        }
        d
    }

    fn molecular_oxygen(&mut self) -> f64 {
        let z = self.z;
        let db32 = self.boundary(4, PDM[3][0]);
        let mut d = self.densu(z, db32, 32.0, 0.0);
        if self.mixing {
            if z <= 250.0 {
                let (net, b32) = self.mixed(d, db32, PDM[3][2], 32.0, 0.0);
                let rl = ln(self.b28 * PDM[3][1] / b32);
                let hc32 = PDM[3][5] * PDL[1][7];
                let zc32 = PDM[3][4] * PDL[1][6];
                d = net * ccor(z, rl, hc32, zc32);
            }
            // The departure from diffusive equilibrium above the lower
            // boundary applies at every altitude.
            let hcc32 = PDM[3][7] * PDL[1][22];
            let hcc232 = PDM[3][7] * PDL[0][22];
            let zcc32 = PDM[3][6] * PDL[1][21];
            let rc32 = PDM[3][3] * PDL[1][23] * self.flux_factor();
            d *= ccor2(z, rc32, hcc32, zcc32, hcc232);
        }
        d
    }

    fn argon(&mut self) -> f64 {
        let z = self.z;
        let alpha = 0.17;
        let db40 = self.boundary(5, PDM[4][0]);
        let mut d = self.densu(z, db40, 40.0, alpha);
        if self.mixing && z <= 240.0 {
            let (net, b40) = self.mixed(d, db40, PDM[4][2], 40.0, alpha);
            let rl = ln(self.b28 * PDM[4][1] / b40);
            let hc40 = PDM[4][5] * PDL[1][9];
            let zc40 = PDM[4][4] * PDL[1][8];
            d = net * ccor(z, rl, hc40, zc40);
        }
        d
    }

    fn atomic_hydrogen(&mut self) -> f64 {
        let z = self.z;
        let alpha = -0.38;
        let db01 = self.boundary(6, PDM[5][0]);
        let mut d = self.densu(z, db01, 1.0, alpha);
        if self.mixing && z <= 320.0 {
            let (net, b01) = self.mixed(d, db01, PDM[5][2], 1.0, alpha);
            let rl = ln(self.b28 * PDM[5][1] * abs(PDL[1][17]) / b01);
            let hc01 = PDM[5][5] * PDL[1][11];
            let zc01 = PDM[5][4] * PDL[1][10];
            d = net * ccor(z, rl, hc01, zc01);
            let hcc01 = PDM[5][7] * PDL[1][19];
            let zcc01 = PDM[5][6] * PDL[1][18];
            let rc01 = PDM[5][3] * PDL[1][20];
            d *= ccor(z, rc01, hcc01, zcc01);
        }
        d
    }

    fn atomic_nitrogen(&mut self) -> f64 {
        let z = self.z;
        let db14 = self.boundary(7, PDM[6][0]);
        let mut d = self.densu(z, db14, 14.0, 0.0);
        if self.mixing && z <= 450.0 {
            let (net, b14) = self.mixed(d, db14, PDM[6][2], 14.0, 0.0);
            let rl = ln(self.b28 * PDM[6][1] * abs(PDL[0][2]) / b14);
            let hc14 = PDM[6][5] * PDL[0][1];
            let zc14 = PDM[6][4] * PDL[0][0];
            d = net * ccor(z, rl, hc14, zc14);
            let hcc14 = PDM[6][7] * PDL[0][4];
            let zcc14 = PDM[6][6] * PDL[0][3];
            let rc14 = PDM[6][3] * PDL[0][5];
            d *= ccor(z, rc14, hcc14, zcc14);
        }
        d
    }

    /// Anomalous oxygen: an isothermal hot population with its own scale
    /// height above its reference altitude.
    fn anomalous_oxygen(&mut self) -> f64 {
        let z = self.z;
        let db16h = self.boundary(8, PDM[7][0]);
        let tho = PDM[7][9] * PDL[0][6];
        let hot = Bates {
            tinf: tho,
            tlb: tho,
            ..self.bates
        };
        let gravity = &self.call.gravity;
        let dd = densu(z, db16h, &hot, 16.0, 0.0, &mut self.nodes, gravity).0;
        let zsht = PDM[7][5];
        let zmho = PDM[7][4];
        let zsho = scalh(zmho, 16.0, tho, gravity);
        dd * exp(-zsht / zsho * (exp(-(z - zmho) / zsht) - 1.0))
    }
}

/// The lower-thermosphere temperature nodes below the joining altitude
/// `za`. Their variations matter to the density only below 300 km.
fn lower_thermosphere_nodes(call: &Call, carry: &Carry, za: f64, alt: f64) -> Nodes<5> {
    let sw = &call.factors.sw;
    let mut tn = [0.0; 5];
    let gradient = if alt < 300.0 {
        tn[1] = PTM[6] * PTL[0][0] / (1.0 - sw[18] * glob7s(&PTL[0], call, carry));
        tn[2] = PTM[2] * PTL[1][0] / (1.0 - sw[18] * glob7s(&PTL[1], call, carry));
        tn[3] = PTM[7] * PTL[2][0] / (1.0 - sw[18] * glob7s(&PTL[2], call, carry));
        tn[4] = PTM[4] * PTL[3][0] / (1.0 - sw[18] * sw[20] * glob7s(&PTL[3], call, carry));
        PTM[8] * PMA[8][0] * (1.0 + sw[18] * sw[20] * glob7s(&PMA[8], call, carry)) * tn[4] * tn[4]
            / ((PTM[4] * PTL[3][0]) * (PTM[4] * PTL[3][0]))
    } else {
        tn[1] = PTM[6] * PTL[0][0];
        tn[2] = PTM[2] * PTL[1][0];
        tn[3] = PTM[7] * PTL[2][0];
        tn[4] = PTM[4] * PTL[3][0];
        PTM[8] * PMA[8][0] * tn[4] * tn[4] / ((PTM[4] * PTL[3][0]) * (PTM[4] * PTL[3][0]))
    };
    Nodes {
        zn: [za, 110.0, 100.0, 90.0, 72.5],
        tn,
        tgn: [0.0, gradient],
    }
}

/// The model at `alt` km, which must not be below 72.5 km.
pub(crate) fn gts7(call: &Call, alt: f64) -> Upper {
    let sw = &call.factors.sw;
    let mut carry = Carry::default();
    let za = PDL[1][15];
    // Variations of the exospheric temperature and of the gradient do not
    // matter below the joining altitude and the lowest node respectively.
    let tinf = if alt > za {
        PTM[0] * PT[0] * (1.0 + sw[16] * globe7(&PT, call, &mut carry))
    } else {
        PTM[0] * PT[0]
    };
    let g0 = if alt > 72.5 {
        PTM[3] * PS[0] * (1.0 + sw[19] * globe7(&PS, call, &mut carry))
    } else {
        PTM[3] * PS[0]
    };
    let tlb = PTM[1] * (1.0 + sw[17] * globe7(&PD[3], call, &mut carry)) * PD[3][0];
    let bates = Bates {
        tinf,
        tlb,
        s2: g0 / (tinf - tlb),
        zlb: PTM[5],
    };
    let nodes = lower_thermosphere_nodes(call, &carry, za, alt);
    let mut column = Column {
        call,
        carry,
        z: alt,
        bates,
        nodes,
        xmm: PDM[2][4],
        zhm28: 0.0,
        b28: 0.0,
        mixing: sw[15] != 0.0,
    };
    // The turbopause height varies with latitude and season.
    let zhf = PDL[1][24]
        * (1.0 + sw[5] * PDL[0][24] * sin(DGTR * call.g_lat) * cos(DR * (call.doy - PT[13])));
    let mut d = [0.0; 9];
    let dm28;
    (d[2], dm28) = column.molecular_nitrogen(zhf);
    d[0] = column.helium();
    d[1] = column.atomic_oxygen();
    d[3] = column.molecular_oxygen();
    d[4] = column.argon();
    d[6] = column.atomic_hydrogen();
    d[7] = column.atomic_nitrogen();
    d[8] = column.anomalous_oxygen();
    d[5] = AMU_G
        * (4.0 * d[0] + 16.0 * d[1] + 28.0 * d[2] + 32.0 * d[3] + 40.0 * d[4] + d[6] + 14.0 * d[7]);
    let gravity = &call.gravity;
    let (_, tz) = densu(abs(alt), 1.0, &bates, 0.0, 0.0, &mut column.nodes, gravity);
    Upper {
        d,
        tinf,
        tz,
        dm28,
        nodes: column.nodes,
        carry: column.carry,
    }
}
