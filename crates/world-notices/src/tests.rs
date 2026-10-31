use world_core::{DecimalYear, GeodeticPosition, ValidityWindow};

use crate::{
    AirspaceKey, ClosureStatus, FacilityKey, LateralShape, NoticeAltitude, NoticeId, NoticesStore,
    RunwayKey, SuaActivationNotice, SuaKind, SurfaceClosureNotice, SurfaceElement, TfrKind,
    TfrNotice, VerticalBand,
};

fn window() -> ValidityWindow {
    ValidityWindow {
        from: DecimalYear(2026.2),
        until: DecimalYear(2026.8),
    }
}

#[test]
fn tfr_containment_and_cancellation_work() {
    let mut store = NoticesStore::default();
    let tfr_id = NoticeId::parse("FDC 6/9141").unwrap();
    let center = GeodeticPosition::wgs84(39.13, -75.47, 0.0);

    store
        .upsert_tfr(TfrNotice {
            id: tfr_id,
            kind: TfrKind::Vip,
            lateral: LateralShape::Circle {
                center,
                radius_m: 18_520.0,
            },
            vertical: VerticalBand {
                floor: NoticeAltitude::Surface,
                ceiling: NoticeAltitude::FlightLevel(180),
            },
            valid: window(),
            cancelled: false,
            provenance: None,
        })
        .unwrap();

    assert_eq!(
        store
            .tfrs_containing(
                GeodeticPosition::wgs84(39.15, -75.48, 900.0),
                900.0,
                DecimalYear(2026.5)
            )
            .len(),
        1
    );
    assert!(
        store
            .tfrs_containing(
                GeodeticPosition::wgs84(39.15, -75.48, 6_500.0),
                6_500.0,
                DecimalYear(2026.5)
            )
            .is_empty()
    );
    assert!(store.cancel(tfr_id));
    assert_eq!(store.prune_expired(DecimalYear(2026.5)), 1);
}

#[test]
fn runway_closures_and_sua_hot_windows_work() {
    let mut store = NoticesStore::default();
    let kdov = FacilityKey::parse("KDOV").unwrap();
    let rwy01 = RunwayKey::parse("01/19").unwrap();
    let rwy14 = RunwayKey::parse("14/32").unwrap();
    store
        .upsert_surface_closure(SurfaceClosureNotice {
            id: NoticeId::parse("DOV 10/012").unwrap(),
            facility: kdov,
            element: SurfaceElement::Runway(rwy01),
            status: ClosureStatus::Closed,
            valid: window(),
            cancelled: false,
        })
        .unwrap();
    assert!(store.is_runway_closed(kdov, rwy01, DecimalYear(2026.5)));
    assert!(!store.is_runway_closed(kdov, rwy14, DecimalYear(2026.5)));

    let r4006 = AirspaceKey::parse("R-4006").unwrap();
    store
        .upsert_sua_activation(SuaActivationNotice {
            id: NoticeId::parse("ZDC 10/4006").unwrap(),
            airspace: r4006,
            kind: SuaKind::Restricted,
            vertical: VerticalBand {
                floor: NoticeAltitude::GeometricMsl(100.0),
                ceiling: NoticeAltitude::FlightLevel(230),
            },
            valid: window(),
            cancelled: false,
        })
        .unwrap();
    assert!(store.is_airspace_active(r4006, Some(2_500.0), DecimalYear(2026.5)));
}
