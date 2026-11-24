//! Public API integration tests for ITU-R P.618-14 Earth-space attenuation.

use world_propagation::p618::{
    AtmosphericGridProvider, P618, P618Path, P618StationPath, P618WithProvider, SiteAtmosphere,
    sha256,
};
use world_propagation::{
    Frequency, Loss, PathLoss, Percentage, Polarisation, PropagationError, Signal,
};

struct SyntheticGrid {
    raw_table: &'static [u8],
    digest: [u8; 32],
}

impl AtmosphericGridProvider for SyntheticGrid {
    fn expected_sha256(&self) -> Option<[u8; 32]> {
        Some(self.digest)
    }

    fn visit_grid_bytes(&self, visitor: &mut dyn FnMut(&[u8])) {
        visitor(self.raw_table);
    }

    fn sample_site(
        &self,
        latitude_deg: f64,
        _longitude_deg: f64,
        station_altitude_km: f64,
    ) -> Result<SiteAtmosphere, PropagationError> {
        let lat_factor = 1.0 - (latitude_deg.abs() / 90.0) * 0.7;
        Ok(SiteAtmosphere {
            rain_rate_001_mm_h: 55.0 * lat_factor,
            zero_isotherm_height_km: 4.5 - 3.0 * (latitude_deg.abs() / 90.0),
            surface_temperature_k: 292.0 - 25.0 * (latitude_deg.abs() / 90.0),
            surface_pressure_hpa: 1013.25 * libm::exp(-station_altitude_km / 8.4),
            water_vapour_density_g_m3: 12.0 * lat_factor,
            integrated_liquid_water_kg_m2: 0.6 * lat_factor,
            wet_refractivity_n: 60.0 * lat_factor,
        })
    }
}

#[test]
fn hash_verified_provider_feeds_p618_end_to_end() {
    let bytes = b"synthetic-itu-grid-v1:lat-banded-test-table";
    let grid = SyntheticGrid {
        raw_table: bytes,
        digest: sha256(bytes),
    };
    let model = P618WithProvider::new(&grid, 1.8, 0.65).unwrap();
    let ka_h = Signal {
        frequency: Frequency::from_ghz(29.5).unwrap(),
        polarisation: Polarisation::Horizontal,
    };
    let ka_v = Signal {
        frequency: Frequency::from_ghz(29.5).unwrap(),
        polarisation: Polarisation::Vertical,
    };
    let path_h = P618StationPath {
        latitude_deg: 35.0,
        longitude_deg: -120.0,
        station_altitude_km: 0.2,
        elevation_deg: 25.0,
        polarisation_tilt_deg: None,
        percentage: Percentage::new(0.01).unwrap(),
    };

    let out_h = model.path_loss(&path_h, &ka_h).unwrap();
    let out_v = model.path_loss(&path_h, &ka_v).unwrap();

    // Horizontal rain attenuation slightly exceeds vertical because raindrops flatten horizontally.
    assert!(out_h.rain_db > out_v.rain_db);
    assert!(out_h.gaseous_db > 0.0);
    assert!(out_h.cloud_db > 0.0);
    assert!(out_h.scintillation_db > 0.0);
    assert!(out_h.effective_rain_path_km > 0.0);
    assert!(out_h.specific_rain_attenuation_db_km > 0.0);
    assert!(out_h.cross_polar_discrimination_db.is_some());

    // Exact P.618-14 §2.5 combination identity: A_T = A_G + sqrt((A_R + A_C)^2 + A_S^2).
    let expected_total =
        out_h.gaseous_db + libm::hypot(out_h.rain_db + out_h.cloud_db, out_h.scintillation_db);
    assert!((out_h.loss_db() - expected_total).abs() < 1e-12);
}

#[test]
fn circular_polarisation_tilt_45_lies_between_h_and_v_and_respects_above_5_percent() {
    let site = SiteAtmosphere::itu_reference(42.0, 4.2, 0.5, 45.0);
    let model = P618::new(site, 38.0, 0.05, 1.2, 0.6).unwrap();

    let p01 = Percentage::new(0.01).unwrap();
    let sig_h = Signal {
        frequency: Frequency::from_ghz(19.7).unwrap(),
        polarisation: Polarisation::Horizontal,
    };
    let sig_v = Signal {
        frequency: Frequency::from_ghz(19.7).unwrap(),
        polarisation: Polarisation::Vertical,
    };
    let lh = model.path_loss(&P618Path::new(30.0, p01), &sig_h).unwrap();
    let lv = model.path_loss(&P618Path::new(30.0, p01), &sig_v).unwrap();
    let lc = model
        .path_loss(
            &P618Path::new(30.0, p01).with_polarisation_tilt_deg(45.0),
            &sig_h,
        )
        .unwrap();

    assert!(lh.rain_db > lc.rain_db && lc.rain_db > lv.rain_db);
    // Linear XPD exceeds circular XPD because C_tau > 0 for tau = 0/90 vs 0 for tau = 45.
    assert!(lh.cross_polar_discrimination_db.unwrap() > lc.cross_polar_discrimination_db.unwrap());

    // Above 5 % exceedance, rain attenuation drops to 0 per §2.5 while gas+cloud+scint remain.
    let p10 = Percentage::new(10.0).unwrap();
    let l10 = model.path_loss(&P618Path::new(30.0, p10), &sig_h).unwrap();
    assert_eq!(l10.rain_db, 0.0);
    assert!(l10.loss_db() > l10.gaseous_db);
    assert!(l10.cross_polar_discrimination_db.is_none());
}
