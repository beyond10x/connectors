//! Adversary pass on story:catalog-operation-declares-its-walk, the importer's stopgap. The
//! definition notation has no field for a read's `walk` yet (story:definition-lowerer-carries-walk).
//! The coordinator decided in correction round 1 that the importer's contract
//! (`src/import.rs`, module documentation) makes `walk` the one member reported as dropped
//! instead of refused: the import succeeds, writes no walk and names each walk it left out, by
//! operation, in `Imported::dropped`.
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
fn an_import_reports_by_operation_each_walk_the_definition_cannot_say() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let selection = repository().join("adapters/catalog/providers/zendesk/operations.json");
    let imported = import::import(&import::Request {
        connectors: &repository(),
        catalog: "zendesk",
        selection: Some(&selection),
        definitions: &root,
        name: Some("zendesk"),
    })
    .unwrap_or_else(|failure| panic!("the selection with five walks imports: {failure}"));
    let dropped: Vec<(&str, &str)> = imported
        .dropped
        .iter()
        .map(|d| (d.operation.as_str(), d.member.as_str()))
        .collect();
    assert_eq!(
        dropped,
        [
            ("tickets.incremental", "walk"),
            ("ticket.comments", "walk"),
            ("ticket.audits", "walk"),
            ("users.incremental", "walk"),
            ("organizations.incremental", "walk"),
        ]
    );
    for dropped in &imported.dropped {
        assert!(
            dropped
                .to_string()
                .contains("story:definition-lowerer-carries-walk"),
            "{dropped}"
        );
    }
    let written = std::fs::read_to_string(&imported.definition).unwrap();
    assert_eq!(written.matches("walk:").count(), 0, "{written}");
}

/// A selection without a walk drops nothing.
#[test]
fn an_import_of_a_selection_without_a_walk_drops_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let imported = import::import(&import::Request {
        connectors: &repository(),
        catalog: "gitlab",
        selection: None,
        definitions: &root,
        name: None,
    })
    .unwrap_or_else(|failure| panic!("gitlab imports: {failure}"));
    assert_eq!(imported.dropped, []);
}
