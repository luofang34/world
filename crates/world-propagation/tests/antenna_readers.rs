//! Public API integration tests for MSI Planet and NSMA WG16.99 pattern readers.
#![cfg(feature = "alloc")]

use std::fmt::Write as _;
use world_propagation::antenna::DBD_TO_DBI;
use world_propagation::{
    AntennaPattern, CutPlane, PatternParseErrorKind, PatternPolarisation, PropagationError,
    TabulatedAntennaPattern,
};

#[test]
fn msi_planet_full_360_pattern_lookup_and_downtilt() {
    let mut text = String::from(
        "NAME Sector_1800_Tilt\nMAKE AntennaLab\nFREQUENCY 1850\nH_WIDTH 65.0\nV_WIDTH 8.0\nFRONT_TO_BACK 28.0\nGAIN 16.0 dBd\nTILT 4\nPOLARIZATION Vertical\nCOMMENT Integration fixture\nHORIZONTAL 360\n",
    );
    for az in 0..360 {
        let diff = az.min(360 - az) as f64;
        let att = (0.01 * diff * diff).min(28.0);
        writeln!(text, "{az} {att:.3}").unwrap();
    }
    text.push_str("VERTICAL 360\n");
    // Peak beam sits at 4 deg downtilt (`VERTICAL` index 4 -> elevation -4 deg).
    for v in 0..360 {
        let el = if v <= 180 {
            -(v as f64)
        } else {
            (360 - v) as f64
        };
        let att = ((el - -4.0).abs() * 0.75).min(25.0);
        writeln!(text, "{v} {att:.3}").unwrap();
    }

    let pat = TabulatedAntennaPattern::from_msi(&text).unwrap();
    assert_eq!(pat.azimuth_cut().len(), 360);
    assert_eq!(pat.tilt_deg(), Some(4.0));
    let expected_peak = 16.0 + DBD_TO_DBI;
    assert!((pat.gain_max_dbi() - expected_peak).abs() < 1e-12);
    // At boresight azimuth 0, downtilt elevation -4 deg, gain equals peak.
    assert!((pat.gain_at_az_el_dbi(0.0, -4.0).unwrap() - expected_peak).abs() < 1e-12);
    // At horizon (0, 0), vertical attenuation is 4 * 0.75 = 3.0 dB.
    assert!((pat.gain_dbi(0.0).unwrap() - (expected_peak - 3.0)).abs() < 1e-12);
    // At rear (180, -4), horizontal attenuation equals 28.0 dB F/B.
    assert!((pat.gain_at_az_el_dbi(180.0, -4.0).unwrap() - (expected_peak - 28.0)).abs() < 1e-12);
    // Elevation out of [-90, 90] is refused.
    assert!(matches!(
        pat.gain_at_az_el_dbi(0.0, 95.0),
        Err(PropagationError::AngleOutsidePattern { .. })
    ));
}

#[test]
fn nsma_wg16_99_supports_dbd_linear_and_hh_vv_hv_vh_cuts() {
    let adf = "\
REVNUM:,NSMA WG16.99.050
ANTMAN:,PrecisionMicrowave
MODNUM:,DISH-6G
LOWFRQ:,5925
HGHFRQ:,6425
GUNITS:,DBD/DBR
MDGAIN:,38.0
AZWIDT:,1.8
ELWIDT:,1.8
FRTOBA:,65.0
NUMCUT:,4
PATCUT:,AZ
POLARI:,H/H
NUPOIN:,5
-180,-65.0
-45,-32.0
0,0.0
45,-32.0
180,-65.0
PATCUT:,AZ
POLARI:,V/V
NUPOIN:,5
-180,-64.0
-45,-31.0
0,0.0
45,-31.0
180,-64.0
PATCUT:,AZ
POLARI:,H/V
NUPOIN:,3
-180,-70.0
0,-30.0
180,-70.0
PATCUT:,EL
POLARI:,V/H
NUPOIN:,3
-90,-50.0
0,-30.0
90,-50.0
ENDFIL:,EOF
";
    let pat = TabulatedAntennaPattern::from_nsma(adf).unwrap();
    let expected_peak = 38.0 + DBD_TO_DBI;
    assert!((pat.gain_max_dbi() - expected_peak).abs() < 1e-12);
    assert!((pat.frequency_mhz().unwrap() - 6175.0).abs() < 1e-12);
    assert!(
        pat.cut(CutPlane::Azimuth, PatternPolarisation::Horizontal)
            .is_some()
    );
    assert!(
        pat.cut(CutPlane::Azimuth, PatternPolarisation::CrossHv)
            .is_some()
    );
    assert!(
        pat.cut(CutPlane::Elevation, PatternPolarisation::CrossVh)
            .is_some()
    );

    // Midpoint interpolation between 0° (0 dB) and 45° (-32 dB) at 22.5° -> -16 dB.
    assert!((pat.gain_dbi(22.5).unwrap() - (expected_peak - 16.0)).abs() < 1e-12);
    // Wrapping negative azimuth -22.5° gives same midpoint.
    assert!((pat.gain_at_az_el_dbi(-22.5, 0.0).unwrap() - (expected_peak - 16.0)).abs() < 1e-12);
    // Cross-polar V/H elevation cut lookup.
    assert!(
        (pat.gain_for_polarisation_dbi(0.0, 0.0, PatternPolarisation::CrossVh)
            .unwrap()
            - (expected_peak - 30.0))
            .abs()
            < 1e-12
    );
}

#[test]
fn malformed_pattern_files_return_structured_errors() {
    assert!(matches!(
        TabulatedAntennaPattern::from_msi("NAME OnlyHeader\n"),
        Err(PropagationError::InvalidPatternFormat {
            kind: PatternParseErrorKind::MissingHeader("GAIN"),
            ..
        })
    ));
    assert!(matches!(
        TabulatedAntennaPattern::from_msi(
            "NAME BadVal\nGAIN 12 dBi\nHORIZONTAL 2\n0 0.0\n1 NaN\nVERTICAL 2\n0 0.0\n1 1.0\n"
        ),
        Err(PropagationError::InvalidPatternFormat {
            kind: PatternParseErrorKind::NonFiniteValue { index: 1 },
            ..
        })
    ));
    assert!(matches!(
        TabulatedAntennaPattern::from_msi(
            "NAME BadText\nGAIN 12 dBi\nHORIZONTAL 2\n0 0.0\n1 abc\nVERTICAL 2\n0 0.0\n1 1.0\n"
        ),
        Err(PropagationError::InvalidPatternFormat {
            kind: PatternParseErrorKind::BadNumber("attenuation"),
            ..
        })
    ));
    assert!(matches!(
        TabulatedAntennaPattern::from_nsma(
            "REVNUM:,NSMA WG16.99.050\nGUNITS:,DBI/XYZ\nMDGAIN:,10\nENDFIL:,EOF\n"
        ),
        Err(PropagationError::InvalidPatternFormat {
            kind: PatternParseErrorKind::UnsupportedUnits,
            ..
        })
    ));
}
