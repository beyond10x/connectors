//! Adversary pass 1 against story:drive-file-content-read: the Discovery
//! projector now projects `alt` on `drive.files.get`, and a definition is
//! lowered at install time with the projector of the installing binary. A
//! Drive definition written before this change (the committed fixture without
//! the `withhold: [alt]` this change added) must not turn the metadata read
//! `files.get` into one whose caller chooses `alt`. No network, no credential.
#![cfg(unix)]
use connectors_catalog::{bundle, definition, import};
use connectors_catalog_provider::{Effect, Engine, Selection};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn fixtures() -> PathBuf {
    repository().join("crates/connectors-catalog/tests/fixtures/definitions")
}

#[test]
fn adversary_a_drive_definition_written_before_files_get_media_keeps_alt_off_files_get() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let selection = repository().join("adapters/catalog/providers/google-drive/operations.json");
    import::import(&import::Request {
        connectors: &repository(),
        catalog: "google-drive",
        selection: Some(&selection),
        definitions: &root,
        name: Some("google-drive"),
    })
    .unwrap();
    std::fs::create_dir_all(root.join("auth")).unwrap();
    for entry in std::fs::read_dir(fixtures().join("auth")).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), root.join("auth").join(entry.file_name())).unwrap();
    }
    // The definition as published before this change: `files.get` withholds nothing.
    let current = std::fs::read_to_string(fixtures().join("connectors/google-drive.yaml")).unwrap();
    let published = current.replace("    withhold: [alt]\n", "");
    assert_ne!(
        published, current,
        "the fixture no longer withholds alt on files.get"
    );
    let file = root.join("connectors/google-drive.yaml");
    std::fs::write(&file, published).unwrap();
    let out = temp.path().join("out");
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&definition::Request {
        definition: &file,
        root: &root,
        out: &out,
        scratch: &scratch,
    })
    .unwrap();
    let operations: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let selections: Vec<Selection> =
        serde_json::from_value(operations["operations"].clone()).unwrap();
    let engine = Engine::new(
        &bundle::load(&out, "google-drive").unwrap(),
        "/drive/v3",
        &selections,
    )
    .unwrap();
    let declaration = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|d| d.id == "files.get")
        .unwrap();
    let properties = declaration.input_schema["properties"].as_object().unwrap();
    assert!(
        !properties.contains_key("alt"),
        "files.get of a definition written before this change now takes `alt` from the caller: {}",
        declaration.input_schema["properties"]["alt"]
    );
}
