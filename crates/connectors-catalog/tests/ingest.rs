use connectors_catalog::{Dialect, Refusal, SOURCE_LIMIT, ingest, ingest_file};

fn document(version: &str) -> Vec<u8> {
    format!(
        r#"{{"openapi":"{version}","info":{{"title":"Fixture","version":"7.3.1","license":{{"name":"MIT"}}}},"paths":{{}}}}"#
    )
    .into_bytes()
}

#[test]
fn the_same_bytes_ingest_to_the_same_record() {
    let bytes = document("3.1.0");
    let first = ingest("fixture.json", &bytes).expect("3.1 is supported");
    let second = ingest("fixture.json", &bytes).expect("3.1 is supported");
    assert_eq!(first, second);
    // The digest is over the file bytes, so it is reproducible outside this crate.
    let expected = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(&bytes))
    };
    assert_eq!(first.source_sha256, expected);
    assert_eq!(first.source_bytes, bytes.len());
}

#[test]
fn both_supported_dialects_report_their_own_version() {
    let thirty = ingest("a.json", &document("3.0.3")).expect("3.0 is supported");
    assert_eq!(thirty.dialect, Dialect::V30);
    assert_eq!(thirty.openapi, "3.0.3");
    let thirty_one = ingest("b.json", &document("3.1.1")).expect("3.1 is supported");
    assert_eq!(thirty_one.dialect, Dialect::V31);
    assert_eq!(thirty_one.openapi, "3.1.1");
    assert_eq!(thirty_one.info_version.as_deref(), Some("7.3.1"));
    assert_eq!(thirty_one.license.as_deref(), Some("MIT"));
}

#[test]
fn each_unsupported_shape_is_refused_by_its_own_reason() {
    let cases: [(&[u8], Refusal); 5] = [
        (br#"{"swagger":"2.0","paths":{}}"#, Refusal::VersionAbsent),
        (br#"{"openapi":"2.0","paths":{}}"#, Refusal::VersionUnsupported),
        (br#"{"openapi":4,"paths":{}}"#, Refusal::VersionAbsent),
        (br#"[1,2,3]"#, Refusal::NotADocument),
        (br#"{"openapi":"3.1","#, Refusal::Malformed),
    ];
    for (bytes, expected) in cases {
        let refusal = ingest("x.json", bytes).expect_err("must refuse");
        assert_eq!(refusal, expected, "for {}", String::from_utf8_lossy(bytes));
        assert!(!refusal.reason().is_empty());
    }
}

#[test]
fn a_document_over_the_limit_is_refused_without_parsing() {
    let oversized = vec![b'{'; SOURCE_LIMIT + 1];
    assert_eq!(
        ingest("big.json", &oversized).expect_err("must refuse"),
        Refusal::TooLarge
    );
}

#[test]
fn a_file_records_its_own_name_and_not_its_path() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("gitlab.openapi.json");
    std::fs::write(&path, document("3.0.0")).expect("write fixture");
    let record = ingest_file(&path).expect("readable 3.0 document");
    assert_eq!(record.file_name, "gitlab.openapi.json");
    assert!(!record.source_sha256.is_empty());
}

#[test]
fn a_missing_file_is_refused_rather_than_panicking() {
    let directory = tempfile::tempdir().expect("temporary directory");
    assert!(ingest_file(&directory.path().join("absent.json")).is_err());
}
