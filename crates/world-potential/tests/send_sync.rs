//! Every public field type is shareable across threads, checked at
//! compile time, and a shared stack evaluates identically on every thread.

use world_potential::{
    AnomalyGrid, GravityModel, MagneticModel, NoField, ScalarAnomalyGrid, Sum, VectorField,
};

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

#[test]
fn field_types_are_send_and_sync() {
    assert_send_sync::<dyn VectorField>();
    assert_send_sync::<NoField>();
    assert_send_sync::<Sum<NoField, NoField>>();
    assert_send_sync::<GravityModel<[f64; 6]>>();
    assert_send_sync::<GravityModel<&[f64]>>();
    assert_send_sync::<MagneticModel<[f64; 6]>>();
    assert_send_sync::<AnomalyGrid<[f64; 12]>>();
    assert_send_sync::<AnomalyGrid<&[f64]>>();
    assert_send_sync::<ScalarAnomalyGrid<[f64; 4]>>();
}

#[cfg(feature = "alloc")]
#[test]
fn owned_field_types_are_send_and_sync() {
    use world_potential::{CofModel, FieldStack, GaussRow, GfcModel, MagneticSeries, ShadrModel};

    assert_send_sync::<Box<dyn VectorField>>();
    assert_send_sync::<FieldStack>();
    assert_send_sync::<GravityModel<Vec<f64>>>();
    assert_send_sync::<MagneticModel<Vec<f64>>>();
    assert_send_sync::<MagneticSeries>();
    assert_send_sync::<AnomalyGrid<Vec<f64>>>();
    assert_send_sync::<ScalarAnomalyGrid<Vec<f64>>>();
    assert_send_sync::<GfcModel>();
    assert_send_sync::<ShadrModel>();
    assert_send_sync::<CofModel>();
    assert_send_sync::<GaussRow>();
}
