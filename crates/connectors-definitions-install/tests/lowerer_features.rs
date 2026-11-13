//! D247: install admits exactly the definition features the lowerer reads. The
//! installer depends on no connectors crate, so the two lists meet only here, in a
//! test target that links both: equal, and each one a member a released
//! definition carries into its lowered selection (`link_next`, which none uses
//! yet, through a released definition given a `link_next` paging style).
#![cfg(unix)]
use connectors_catalog::definition::{self, Request};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// connectors-definitions 0.5.2's `connectors/github.yaml` and its pinned source,
/// the copy `connectors-catalog` holds for its own tests.
fn released() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../connectors-catalog/tests/fixtures/definitions-0.5.2")
}

#[test]
fn install_features_are_the_features_the_lowerer_reads() {
    assert_eq!(
        connectors_definitions_install::FEATURES,
        definition::READ_FEATURES,
        "install admits exactly what the lowerer reads"
    );
    let temp = tempfile::tempdir().unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: &released().join("connectors/github.yaml"),
        root: &released(),
        out: &temp.path().join("out"),
        scratch: &scratch,
    })
    .expect("a definition using every listed feature lowers");
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let mut entries = selection["operations"].as_array().unwrap().clone();
    // No released definition uses `link_next` yet; GitHub's lists, which page
    // by the same `Link` header, carry it through a paging style here.
    let paged = std::fs::read_to_string(released().join("connectors/github.yaml"))
        .unwrap()
        .replacen(
            "\ndefaults:",
            "\npaging:\n  pages: {request: page, link_next: true}\ndefaults:",
            1,
        );
    let file = temp.path().join("github-link-next.yaml");
    std::fs::write(&file, paged).unwrap();
    let lowered = definition::lower(&Request {
        definition: &file,
        root: &released(),
        out: &temp.path().join("out-link-next"),
        scratch: &scratch,
    })
    .expect("GitHub's definition with a link_next paging style lowers");
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    entries.extend(selection["operations"].as_array().unwrap().iter().cloned());
    for feature in connectors_definitions_install::FEATURES {
        assert!(
            entries.iter().any(|entry| entry.get(*feature).is_some()),
            "no lowered operation carries `{feature}`"
        );
    }
}
