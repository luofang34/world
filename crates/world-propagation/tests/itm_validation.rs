//! ITM point-to-point mode against NTIA's own test vectors.
//!
//! `data/ntia_itm_p2p.csv` and `data/ntia_itm_pfls.csv` are `p2p.csv` and
//! `pfls.csv` from github.com/NTIA/itm at commit
//! 62168e686e8af4a234cf3f8067d80b543a5d52ef, unmodified (SHA-256
//! 49d58a3d…1341 and a149390…fa10); like the code they are US-government
//! work, provided by NTIA "as is". Each `pfls.csv` row is a profile in
//! ITM's `pfl` layout (intervals, spacing in m, heights in m) for the
//! matching `p2p.csv` row, whose `A__db` is printed to 0.01 dB.
//!
//! The full-precision values are what the reference C++ prints for the
//! same inputs when built from that commit (Apple clang 21, `-O0
//! -ffp-contract=off`, macOS libm): basic transmission loss, reference
//! attenuation, Δh, horizon angles and distances, effective heights and
//! surface refractivity.

use world_propagation::itm::{Climate, Itm, ItmPath, PropagationMode, VariabilityMode};
use world_propagation::{
    Frequency, Ground, PathLoss, Percentage, Polarisation, ProfilePoint, Signal,
};

const P2P: &str = include_str!("data/ntia_itm_p2p.csv");
const PFLS: &str = include_str!("data/ntia_itm_pfls.csv");

/// Reference C++ output per row: A, A_ref, Δh, θ_tx, θ_rx, d_hzn,tx,
/// d_hzn,rx, h_e,tx, h_e,rx, N_s, mode.
const ORACLE: [([f64; 10], PropagationMode); 5] = [
    (
        [
            207.65478878914726,
            82.141635709919385,
            96.969710638224683,
            -0.0044787649638571136,
            -0.00048624806065802125,
            54288.081149999765,
            4099.1000500155951,
            43.750429746750513,
            1.0,
            298.85962362781544,
        ],
        PropagationMode::Troposcatter,
    ),
    (
        [
            157.10373515002374,
            59.261077872672246,
            815.22265364027692,
            0.11256922540757956,
            0.063721356436157908,
            2991.2697599999992,
            598.25395200000719,
            23.121078977149637,
            15.578091482456102,
            277.33062199093638,
        ],
        PropagationMode::LineOfSight,
    ),
    (
        [
            178.52929329705754,
            58.325898371200722,
            379.99446443633457,
            0.020313478707428795,
            0.014519544961272049,
            15393.888047999955,
            9696.1502640001381,
            37.736170915599331,
            12.53157296802587,
            295.69073811820937,
        ],
        PropagationMode::LineOfSight,
    ),
    (
        [
            183.25558085765567,
            35.659034948484631,
            33.324732681472724,
            0.0028975829946907925,
            -0.0013069890957668807,
            23705.265429000083,
            4901.0886329998411,
            9.9258587328964438,
            14.434655571167013,
            299.19643553159955,
        ],
        PropagationMode::Diffraction,
    ),
    (
        [
            218.90767034154055,
            85.926624645022031,
            109.2797875338585,
            0.022754716052385045,
            0.11383799806151743,
            99.865241999999995,
            99.865242000032765,
            1.5,
            10.0,
            298.59912720329794,
        ],
        PropagationMode::Diffraction,
    ),
];

#[cfg(test)]
fn numbers(line: &str) -> Vec<f64> {
    line.split(',').map(|v| v.trim().parse().unwrap()).collect()
}

/// A `pfl` row as equally spaced profile points.
#[cfg(test)]
fn profile(pfl: &[f64]) -> Vec<ProfilePoint> {
    let xi = pfl[1];
    pfl[2..]
        .iter()
        .enumerate()
        .map(|(i, &h)| ProfilePoint {
            distance_m: i as f64 * xi,
            height_m: h,
        })
        .collect()
}

#[cfg(test)]
fn climate(code: f64) -> Climate {
    [
        Climate::Equatorial,
        Climate::ContinentalSubtropical,
        Climate::MaritimeSubtropical,
        Climate::Desert,
        Climate::ContinentalTemperate,
        Climate::MaritimeTemperateOverLand,
        Climate::MaritimeTemperateOverSea,
    ][code as usize - 1]
}

/// `mdvar` as mode plus the +10 / +20 eliminations.
#[cfg(test)]
fn model(row: &[f64]) -> Itm {
    let mdvar = row[11] as u32;
    let mode = [
        VariabilityMode::SingleMessage,
        VariabilityMode::Accidental,
        VariabilityMode::Mobile,
        VariabilityMode::Broadcast,
    ][(mdvar % 10) as usize];
    let ground = Ground {
        relative_permittivity: row[2],
        conductivity_s_m: row[3],
    };
    let mut itm = Itm::new(climate(row[7]), row[4], ground, mode).unwrap();
    if mdvar % 20 >= 10 {
        itm = itm.without_location_variability();
    }
    if mdvar >= 20 {
        itm = itm.without_situation_variability();
    }
    itm
}

#[test]
fn reproduces_ntia_p2p_vectors() {
    let rows: Vec<Vec<f64>> = P2P.lines().skip(1).map(numbers).collect();
    let profiles: Vec<Vec<f64>> = PFLS.lines().map(numbers).collect();
    assert_eq!(rows.len(), 5);
    for (index, ((row, pfl), (oracle, mode))) in rows.iter().zip(&profiles).zip(ORACLE).enumerate()
    {
        assert_eq!(pfl[0] as usize + 3, pfl.len(), "row {index}: pfl layout");
        let terrain = profile(pfl);
        let path = ItmPath {
            profile: terrain.as_slice(),
            tx_height_m: row[0],
            rx_height_m: row[1],
            time: Percentage::new(row[8]).unwrap(),
            location: Percentage::new(row[9]).unwrap(),
            situation: Percentage::new(row[10]).unwrap(),
        };
        let signal = Signal {
            frequency: Frequency::from_mhz(row[5]).unwrap(),
            polarisation: if row[6] == 0.0 {
                Polarisation::Horizontal
            } else {
                Polarisation::Vertical
            },
        };
        let loss = model(row).path_loss(&path, &signal).unwrap();
        let published = row[12];
        assert!(
            (loss.basic_transmission_loss_db - published).abs() <= 0.005 + 1e-9,
            "row {index}: {} dB, NTIA prints {published} dB",
            loss.basic_transmission_loss_db
        );
        let got = [
            loss.basic_transmission_loss_db,
            loss.reference_attenuation_db,
            loss.terrain_irregularity_m,
            loss.horizon_angle_rad[0],
            loss.horizon_angle_rad[1],
            loss.horizon_distance_m[0],
            loss.horizon_distance_m[1],
            loss.effective_height_m[0],
            loss.effective_height_m[1],
            loss.surface_refractivity_n,
        ];
        for (k, (g, o)) in got.iter().zip(oracle).enumerate() {
            let tolerance = 1e-9 * o.abs().max(1.0);
            assert!(
                (g - o).abs() <= tolerance,
                "row {index} value {k}: {g} vs reference {o}"
            );
        }
        assert_eq!(loss.mode, mode, "row {index}");
    }
}

/// 240 random cases across the model's input ranges, run through the
/// reference C++ built as above: every `mdvar`, climate, polarisation,
/// frequencies 20 MHz to 20 GHz, heights 0.5 to 1000 m, percentages
/// into both tails, and each NTIA profile cut to a random number of
/// intervals (columns `profile`, `intervals`). Generated by a seeded
/// script (seed 20261124) that writes each cut profile in `pfl` layout,
/// calls `ITM_P2P_TLS_Ex` and prints `%.17g`.
const SWEEP: &str = include_str!("data/ntia_itm_sweep.csv");

#[test]
fn reproduces_reference_code_across_input_ranges() {
    let profiles: Vec<Vec<f64>> = PFLS.lines().map(numbers).collect();
    let mut checked = 0;
    for (index, line) in SWEEP.lines().skip(1).enumerate() {
        let c = numbers(line);
        let full = &profiles[c[0] as usize];
        let intervals = c[1] as usize;
        let mut pfl = vec![intervals as f64, full[1]];
        pfl.extend_from_slice(&full[2..2 + intervals + 1]);
        let terrain = profile(&pfl);
        // Same column order as p2p.csv from h_tx onwards, re-laid.
        let row = [
            c[2], c[3], c[8], c[9], c[5], c[6], c[7], c[4], c[11], c[12], c[13], c[10],
        ];
        let path = ItmPath {
            profile: terrain.as_slice(),
            tx_height_m: row[0],
            rx_height_m: row[1],
            time: Percentage::new(row[8]).unwrap(),
            location: Percentage::new(row[9]).unwrap(),
            situation: Percentage::new(row[10]).unwrap(),
        };
        let signal = Signal {
            frequency: Frequency::from_mhz(row[5]).unwrap(),
            polarisation: if row[6] == 0.0 {
                Polarisation::Horizontal
            } else {
                Polarisation::Vertical
            },
        };
        let loss = model(&row).path_loss(&path, &signal).unwrap();
        let (expected_a, expected_ref) = (c[16], c[17]);
        let tolerance = |v: f64| 1e-9 * v.abs().max(1.0);
        assert!(
            (loss.basic_transmission_loss_db - expected_a).abs() <= tolerance(expected_a),
            "case {index}: A {} vs reference {expected_a}",
            loss.basic_transmission_loss_db
        );
        assert!(
            (loss.reference_attenuation_db - expected_ref).abs() <= tolerance(expected_ref),
            "case {index}: A_ref {} vs reference {expected_ref}",
            loss.reference_attenuation_db
        );
        assert_eq!(
            u32::from(loss.warnings.bits()),
            c[15] as u32,
            "case {index}: warnings"
        );
        let mode = match c[18] as u32 {
            1 => PropagationMode::LineOfSight,
            2 => PropagationMode::Diffraction,
            _ => PropagationMode::Troposcatter,
        };
        assert_eq!(loss.mode, mode, "case {index}");
        checked += 1;
    }
    assert_eq!(checked, 240);
}
