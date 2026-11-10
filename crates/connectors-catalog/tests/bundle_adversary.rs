//! Adversary cases for story:bundle-index-refuses-ambiguous-rows. The story's
//! outcome is that `load` never reads a file outside the bundle directory; the
//! row checks cover the index's `file_name`, and these cases probe the file the
//! row names.

#![cfg(unix)]

use connectors_catalog::bundle::{Bundle, load, read_index, write};
use connectors_catalog::{ingest, inventory::extract};
use serde_json::json;

fn bundle(provider: &str) -> Bundle {
    let bytes = json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture", "version": "2"},
        "paths": {"/things": {"get": {"operationId": "listThings", "responses": {"200": {}}}}}
    })
    .to_string()
    .into_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: provider.to_owned(),
        source: ingest("fixture.json", &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

/// The index row is exactly what `write` produces and passes every row check,
/// but `<provider>.bundle.json` is a symbolic link to a bundle outside the
/// directory. `load` must not read it.
#[test]
fn a_bundle_file_that_links_outside_the_directory_is_not_followed() {
    let root = tempfile::tempdir().expect("temporary directory");
    let directory = root.path().join("bundles");
    let outside = root.path().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("outside directory");
    let entry = write(&directory, &bundle("fixture"), false).expect("first write");
    let inside = directory.join(&entry.file_name);
    let target = outside.join(&entry.file_name);
    std::fs::rename(&inside, &target).expect("move the bundle outside");
    std::os::unix::fs::symlink(&target, &inside).expect("link to the outside bundle");

    read_index(&directory).expect("the index rows are well formed");
    let loaded = load(&directory, "fixture");
    assert!(
        loaded.is_err(),
        "load read {} through a link out of {}",
        target.display(),
        directory.display()
    );
}
