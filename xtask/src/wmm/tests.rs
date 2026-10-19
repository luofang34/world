use super::*;

fn committed() -> (Vec<u8>, String, String) {
    let root = crate::repo_root();
    (
        read(&root, COF).expect("WMM.COF"),
        read_text(&root, TABLE).expect("table"),
        read_text(&root, PROVENANCE).expect("provenance"),
    )
}

#[test]
fn the_committed_table_reproduces_from_the_recorded_file() {
    let (cof, table, provenance) = committed();
    verify(&cof, &table, &provenance).expect("the committed files agree");
}

#[test]
fn an_edited_table_is_refused() {
    let (cof, table, provenance) = committed();
    let edited = table.replacen("(1, 0, -29351.8,", "(1, 0, -29351.9,", 1);
    assert_ne!(edited, table, "the fixture edits a real row");
    assert!(matches!(
        verify(&cof, &edited, &provenance),
        Err(XtaskError::TableDiffers { .. })
    ));
}

#[test]
fn a_different_coefficient_file_is_refused() {
    let (mut cof, table, provenance) = committed();
    cof.push(b'\n');
    assert!(matches!(
        verify(&cof, &table, &provenance),
        Err(XtaskError::ChecksumMismatch { .. })
    ));
}

#[test]
fn a_provenance_without_a_checksum_is_refused() {
    let (cof, table, _) = committed();
    assert!(matches!(
        verify(&cof, &table, "no checksum here"),
        Err(XtaskError::NoRecordedChecksum { .. })
    ));
}
