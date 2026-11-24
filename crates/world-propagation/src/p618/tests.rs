use super::*;
use crate::units::{Frequency, Polarisation};

fn sig(ghz: f64, pol: Polarisation) -> Signal {
    Signal {
        frequency: Frequency::from_ghz(ghz).unwrap(),
        polarisation: pol,
    }
}

fn pct(p: f64) -> Percentage {
    Percentage::new(p).unwrap()
}

struct MockGridProvider {
    bytes: &'static [u8],
    expected: Option<[u8; 32]>,
    sample: SiteAtmosphere,
}

impl AtmosphericGridProvider for MockGridProvider {
    fn expected_sha256(&self) -> Option<[u8; 32]> {
        self.expected
    }

    fn visit_grid_bytes(&self, visitor: &mut dyn FnMut(&[u8])) {
        visitor(self.bytes);
    }

    fn sample_site(
        &self,
        latitude_deg: f64,
        _longitude_deg: f64,
        _station_altitude_km: f64,
    ) -> Result<SiteAtmosphere, PropagationError> {
        if latitude_deg.abs() > 80.0 {
            return Err(PropagationError::GridDataUnavailable {
                latitude_deg,
                longitude_deg: 0.0,
            });
        }
        Ok(self.sample)
    }
}

#[test]
fn sha256_matches_nist_fips_180_4_vectors() {
    assert_eq!(
        integrity::sha256(b""),
        integrity::parse_sha256_hex(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
        .unwrap()
    );
    assert_eq!(
        integrity::sha256(b"abc"),
        integrity::parse_sha256_hex(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )
        .unwrap()
    );
    assert_eq!(
        integrity::sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        integrity::parse_sha256_hex(
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        )
        .unwrap()
    );
}

#[test]
fn verified_provider_accepts_exact_digest_and_rejects_tamper_or_missing() {
    let payload = b"ITU-P837-P839-GRID-V1";
    let digest = integrity::sha256(payload);
    let site = SiteAtmosphere::itu_reference(35.0, 4.0, 0.5, 45.0);

    let good = MockGridProvider {
        bytes: payload,
        expected: Some(digest),
        sample: site,
    };
    let model = P618WithProvider::new(&good, 1.2, 0.6).unwrap();
    assert_eq!(model.verified_sha256(), digest);
    let out = model
        .path_loss(
            &P618StationPath {
                latitude_deg: 40.0,
                longitude_deg: -75.0,
                station_altitude_km: 0.1,
                elevation_deg: 30.0,
                polarisation_tilt_deg: None,
                percentage: pct(0.01),
            },
            &sig(20.0, Polarisation::Horizontal),
        )
        .unwrap();
    assert!(out.loss_db() > 5.0);

    let mut bad_digest = digest;
    bad_digest[0] ^= 0xff;
    let tampered = MockGridProvider {
        bytes: payload,
        expected: Some(bad_digest),
        sample: site,
    };
    assert!(matches!(
        P618WithProvider::new(&tampered, 1.2, 0.6),
        Err(PropagationError::GridIntegrityMismatch)
    ));

    let missing_digest = MockGridProvider {
        bytes: payload,
        expected: None,
        sample: site,
    };
    assert!(matches!(
        P618WithProvider::new(&missing_digest, 1.2, 0.6),
        Err(PropagationError::GridIntegrityMismatch)
    ));
}

#[test]
fn rain_coefficients_match_p838_3_reference_values() {
    // At 10 GHz H/V: P.838-3 Table 5 reference values are
    // k_H = 0.01217, alpha_H = 1.2571, k_V = 0.01129, alpha_V = 1.2156.
    let (kh, ah) = rain::specific_rain_coefficients(10.0, 0.0, 0.0);
    let (kv, av) = rain::specific_rain_coefficients(10.0, 0.0, 90.0);
    assert!((kh - 0.01217).abs() < 2e-4, "kh = {kh}");
    assert!((ah - 1.2571).abs() < 2e-3, "ah = {ah}");
    assert!((kv - 0.01129).abs() < 2e-4, "kv = {kv}");
    assert!((av - 1.2156).abs() < 2e-3, "av = {av}");
}

#[test]
fn cloud_kl_matches_p840_rayleigh_double_debye_at_0_c() {
    // At 30 GHz and 273.15 K, K_l ≈ 0.771 (dB/km)/(g/m^3).
    let kl_30 = cloud::specific_cloud_attenuation_kl(30.0, 273.15);
    assert!((kl_30 - 0.771).abs() < 0.02, "kl_30 = {kl_30}");
    let kl_10 = cloud::specific_cloud_attenuation_kl(10.0, 273.15);
    assert!(kl_30 > kl_10);
}

#[test]
fn gaseous_exhibits_22_2_ghz_water_vapour_peak_and_oxygen_edge() {
    let dry = gaseous::gaseous_attenuation(12.0, 30.0, 1013.25, 288.15, 0.0);
    assert_eq!(dry.water_vapour_db, 0.0);
    assert!(dry.oxygen_db > 0.0);

    let g_15 = gaseous::gaseous_attenuation(15.0, 30.0, 1013.25, 288.15, 7.5);
    let g_22 = gaseous::gaseous_attenuation(22.235, 30.0, 1013.25, 288.15, 7.5);
    let g_30 = gaseous::gaseous_attenuation(30.0, 30.0, 1013.25, 288.15, 7.5);
    let g_54_5 = gaseous::gaseous_attenuation(54.5, 30.0, 1013.25, 288.15, 7.5);
    assert!(g_22.water_vapour_db > g_15.water_vapour_db);
    assert!(g_22.water_vapour_db > g_30.water_vapour_db);
    assert!(g_54_5.oxygen_db > g_30.oxygen_db);
}

#[test]
fn large_aperture_scintillation_does_not_produce_nan() {
    // Large 30 m dish at 50 GHz has x >> 7 where raw square-root argument turns negative.
    let s = scintillation::scintillation_attenuation_db(50.0, 60.0, 30.0, 0.65, 50.0, 0.01);
    assert!(s.is_finite() && s == 0.0);
    let small = scintillation::scintillation_attenuation_db(30.0, 10.0, 0.6, 0.6, 50.0, 0.01);
    assert!(small > 0.5 && small.is_finite());
}

#[test]
fn monotonicity_across_rain_elevation_percentage_and_frequency() {
    let site_light = SiteAtmosphere::itu_reference(10.0, 3.8, 0.4, 40.0);
    let site_heavy = SiteAtmosphere::itu_reference(60.0, 3.8, 0.4, 40.0);
    let light = P618::new(site_light, 40.0, 0.1, 1.0, 0.6).unwrap();
    let heavy = P618::new(site_heavy, 40.0, 0.1, 1.0, 0.6).unwrap();

    let ku = sig(14.25, Polarisation::Horizontal);
    let ka = sig(28.5, Polarisation::Horizontal);

    let l_light = light
        .path_loss(&P618Path::new(30.0, pct(0.01)), &ku)
        .unwrap();
    let l_heavy = heavy
        .path_loss(&P618Path::new(30.0, pct(0.01)), &ku)
        .unwrap();
    assert!(l_heavy.rain_db > l_light.rain_db);
    assert!(l_heavy.loss_db() > l_light.loss_db());

    let l_low_el = heavy
        .path_loss(&P618Path::new(10.0, pct(0.01)), &ku)
        .unwrap();
    let l_zenith = heavy
        .path_loss(&P618Path::new(90.0, pct(0.01)), &ku)
        .unwrap();
    assert!(l_low_el.loss_db() > l_heavy.loss_db());
    assert!(l_heavy.loss_db() > l_zenith.loss_db());
    assert!(l_zenith.loss_db().is_finite());

    let l_1pct = heavy
        .path_loss(&P618Path::new(30.0, pct(1.0)), &ku)
        .unwrap();
    assert!(l_heavy.loss_db() > l_1pct.loss_db());

    let l_ka = heavy
        .path_loss(&P618Path::new(30.0, pct(0.01)), &ka)
        .unwrap();
    assert!(l_ka.rain_db > l_heavy.rain_db);
    assert!(l_ka.loss_db() > l_heavy.loss_db());
    assert!(l_ka.cross_polar_discrimination_db.is_some());
    assert!(l_zenith.cross_polar_discrimination_db.is_none());
}

#[test]
fn rejects_out_of_range_frequency_elevation_percentage_and_site() {
    let site = SiteAtmosphere::itu_reference(30.0, 4.0, 0.5, 40.0);
    let model = P618::new(site, 40.0, 0.1, 1.0, 0.6).unwrap();
    let ok_path = P618Path::new(30.0, pct(0.1));

    assert!(matches!(
        model.path_loss(&ok_path, &sig(0.9, Polarisation::Vertical)),
        Err(PropagationError::FrequencyOutOfRange { .. })
    ));
    assert!(matches!(
        model.path_loss(&ok_path, &sig(56.0, Polarisation::Vertical)),
        Err(PropagationError::FrequencyOutOfRange { .. })
    ));
    assert!(
        model
            .path_loss(
                &P618Path::new(4.9, pct(0.1)),
                &sig(20.0, Polarisation::Horizontal)
            )
            .is_err()
    );
    assert!(
        model
            .path_loss(
                &P618Path::new(90.1, pct(0.1)),
                &sig(20.0, Polarisation::Horizontal)
            )
            .is_err()
    );
    assert!(
        model
            .path_loss(
                &P618Path::new(30.0, pct(50.1)),
                &sig(20.0, Polarisation::Horizontal)
            )
            .is_err()
    );
    assert!(
        model
            .path_loss(
                &P618Path::new(30.0, pct(0.0005)),
                &sig(20.0, Polarisation::Horizontal)
            )
            .is_err()
    );
    assert!(P618::new(site, 91.0, 0.0, 1.0, 0.6).is_err());
    assert!(P618::new(site, 40.0, 0.0, 0.0, 0.6).is_err());
    assert!(P618::new(site, 40.0, 0.0, 1.0, 1.2).is_err());
    let mut bad_site = site;
    bad_site.rain_rate_001_mm_h = -1.0;
    assert!(P618::new(bad_site, 40.0, 0.0, 1.0, 0.6).is_err());
}
