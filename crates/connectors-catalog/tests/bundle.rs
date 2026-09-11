use connectors_catalog::bundle::{Bundle, load, read_index, write};
use connectors_catalog::{ingest, inventory::extract};
use serde_json::json;

fn source_bytes() -> Vec<u8> {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture", "version": "2"},
        "paths": {"/things": {"get": {"operationId": "listThings", "responses": {"200": {}}}}}
    })
    .to_string()
    .into_bytes()
}

fn bundle(provider: &str) -> Bundle {
    let bytes = source_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: provider.to_owned(),
        source: ingest("fixture.json", &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

#[test]
fn a_written_bundle_loads_back_unchanged() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let original = bundle("fixture");
    let entry = write(directory.path(), &original, false).expect("first write");
    assert_eq!(entry.operations, 1);
    assert_eq!(entry.unsupported, 0);
    assert_eq!(entry.source_sha256, original.source.source_sha256);
    let loaded = load(directory.path(), "fixture").expect("load");
    assert_eq!(loaded, original);
}

#[test]
fn one_altered_byte_makes_the_load_refuse() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let entry = write(directory.path(), &bundle("fixture"), false).expect("first write");
    let path = directory.path().join(&entry.file_name);
    let mut bytes = std::fs::read(&path).expect("read bundle");
    let last = bytes.len() - 2;
    bytes[last] = if bytes[last] == b' ' { b'\t' } else { b' ' };
    std::fs::write(&path, bytes).expect("rewrite bundle");
    let error = load(directory.path(), "fixture").expect_err("must refuse");
    assert!(
        error.message.contains("digest the index recorded"),
        "unexpected message: {}",
        error.message
    );
}

#[test]
fn an_unknown_provider_and_a_missing_file_refuse_differently() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let entry = write(directory.path(), &bundle("fixture"), false).expect("first write");
    let unknown = load(directory.path(), "absent").expect_err("must refuse");
    assert!(unknown.message.contains("no such provider"));
    std::fs::remove_file(directory.path().join(&entry.file_name)).expect("remove bundle");
    let missing = load(directory.path(), "fixture").expect_err("must refuse");
    assert!(missing.message.contains("indexed bundle file is absent"));
}

#[test]
fn replacing_an_indexed_provider_needs_asking() {
    let directory = tempfile::tempdir().expect("temporary directory");
    write(directory.path(), &bundle("fixture"), false).expect("first write");
    let refused = write(directory.path(), &bundle("fixture"), false).expect_err("must refuse");
    assert!(refused.message.contains("already indexed"));
    write(directory.path(), &bundle("fixture"), true).expect("explicit replacement");
    assert_eq!(read_index(directory.path()).expect("index").entries.len(), 1);
}

#[test]
fn the_index_lists_providers_in_a_stable_order() {
    let directory = tempfile::tempdir().expect("temporary directory");
    for provider in ["gitlab", "acme", "fixture"] {
        write(directory.path(), &bundle(provider), false).expect("write");
    }
    let index = read_index(directory.path()).expect("index");
    assert_eq!(index.providers(), vec!["acme", "fixture", "gitlab"]);
}

#[test]
fn an_empty_directory_reads_as_an_empty_index_rather_than_an_error() {
    let directory = tempfile::tempdir().expect("temporary directory");
    assert!(read_index(directory.path()).expect("index").entries.is_empty());
}
