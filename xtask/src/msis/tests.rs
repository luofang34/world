use super::*;

fn committed() -> (Vec<u8>, String) {
    let root = crate::repo_root();
    (
        read(&root, DATA).expect("data file"),
        read_text(&root, PROVENANCE).expect("provenance"),
    )
}

fn committed_table(path: &'static str) -> Result<String, XtaskError> {
    read_text(&crate::repo_root(), path)
}

#[test]
fn the_committed_tables_reproduce_from_the_recorded_file() {
    let (data, provenance) = committed();
    verify(&data, &provenance, &committed_table).expect("the committed files agree");
}

#[test]
fn every_array_has_the_size_the_model_indexes() {
    let (data, _) = committed();
    let text = without_comments(std::str::from_utf8(&data).expect("text"));
    let found = arrays(&text).expect("arrays");
    let size = |name: &str| {
        let array = found.iter().find(|a| a.name == name).expect(name);
        (array.values.len(), array.columns)
    };
    assert_eq!(size("pt"), (150, None));
    assert_eq!(size("pd"), (9 * 150, Some(150)));
    assert_eq!(size("ps"), (150, None));
    assert_eq!(size("pdl"), (2 * 25, Some(25)));
    assert_eq!(size("ptm"), (10, None));
    assert_eq!(size("pdm"), (8 * 10, Some(10)));
    assert_eq!(size("ptl"), (4 * 100, Some(100)));
    assert_eq!(size("pma"), (10 * 100, Some(100)));
    assert_eq!(size("pavgm"), (10, None));
}

#[test]
fn an_edited_table_is_refused() {
    let (data, provenance) = committed();
    let edited = |path: &'static str| {
        committed_table(path).map(|table| table.replacen("9.86573E-01", "9.86574E-01", 1))
    };
    assert!(matches!(
        verify(&data, &provenance, &edited),
        Err(XtaskError::TableDiffers { .. })
    ));
}

#[test]
fn a_different_data_file_is_refused() {
    let (mut data, provenance) = committed();
    data.push(b'\n');
    assert!(matches!(
        verify(&data, &provenance, &committed_table),
        Err(XtaskError::ChecksumMismatch { .. })
    ));
}
