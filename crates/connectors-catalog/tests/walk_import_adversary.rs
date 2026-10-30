//! Adversary pass on story:catalog-operation-declares-its-walk, the importer's stopgap. The
//! importer's own contract (`src/import.rs`, module documentation): "a selection the definition
//! notation cannot say is refused by name ([`Failure::Unexpressible`]) rather than written
//! approximately". The definition notation has no field for a read's `walk` yet
//! (story:definition-lowerer-carries-walk), and the stopgap removes `walk` from the expected
//! selection before the round-trip comparison, so a selection with a walk is written as a
//! definition without it and the import reports success.
#![cfg(unix)]
use connectors_catalog::import;
use std::path::{Path, PathBuf};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn an_import_refuses_by_name_a_walk_the_definition_cannot_say() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let selection = repository().join("adapters/catalog/providers/zendesk/operations.json");
    let outcome = import::import(&import::Request {
        connectors: &repository(),
        catalog: "zendesk",
        selection: Some(&selection),
        definitions: &root,
        name: Some("zendesk"),
    });
    match outcome {
        Err(import::Failure::Unexpressible { field, .. }) => assert_eq!(field, "walk"),
        Err(other) => panic!("refused for another reason: {other}"),
        Ok(imported) => {
            let written = std::fs::read_to_string(&imported.definition).unwrap();
            panic!(
                "the selection's five walks were dropped and the import reported success; \
                 the written definition says `walk` {} times",
                written.matches("walk:").count()
            );
        }
    }
    assert!(!root.join("connectors/zendesk.yaml").exists());
}
