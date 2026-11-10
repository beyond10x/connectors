//! Adversary pass 2 for story:bundle-index-refuses-ambiguous-rows. The
//! correction for pass 1 F1 checks `symlink_metadata` of the bundle file and
//! then reads it by path, and leaves `index.json` read through any link. These
//! cases probe both: the outcome is that `load` never reads a file outside the
//! bundle directory.

#![cfg(unix)]

use connectors_catalog::bundle::{Bundle, INDEX_FILE, load, write};
use connectors_catalog::{ingest, inventory::extract};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

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

/// `index.json` is a symbolic link to an index outside the directory. The rows
/// pass every check, but the index itself was read from outside.
#[test]
fn an_index_that_links_outside_the_directory_is_not_followed() {
    let root = tempfile::tempdir().expect("temporary directory");
    let directory = root.path().join("bundles");
    let outside = root.path().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("outside directory");
    write(&directory, &bundle("fixture"), false).expect("first write");
    let inside = directory.join(INDEX_FILE);
    let target = outside.join(INDEX_FILE);
    std::fs::rename(&inside, &target).expect("move the index outside");
    std::os::unix::fs::symlink(&target, &inside).expect("link to the outside index");

    let loaded = load(&directory, "fixture");
    assert!(
        loaded.is_err(),
        "load read the index {} through a link out of {}",
        target.display(),
        directory.display()
    );
}

/// The bundle file's name alternates, by atomic rename, between a regular file
/// whose bytes do not match the index and a link to the matching bundle outside
/// the directory. Neither state alone loads: the regular file fails the digest
/// and the link fails the regular-file check. An `Ok` therefore means `load`
/// passed the check on the regular file and then read the outside bundle through
/// the link, because the check and the read resolve the path twice.
#[test]
fn a_bundle_file_swapped_for_a_link_after_the_check_is_not_followed() {
    let root = tempfile::tempdir().expect("temporary directory");
    let directory = root.path().join("bundles");
    let outside = root.path().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("outside directory");
    let entry = write(&directory, &bundle("fixture"), false).expect("first write");
    let inside = directory.join(&entry.file_name);
    let target = outside.join(&entry.file_name);
    std::fs::rename(&inside, &target).expect("move the bundle outside");
    std::fs::write(&inside, b"{}").expect("decoy regular file");

    let stop = Arc::new(AtomicBool::new(false));
    let swapper = {
        let stop = Arc::clone(&stop);
        let inside = inside.clone();
        let target = target.clone();
        let staging = root.path().join("staging");
        std::fs::create_dir_all(&staging).expect("staging directory");
        std::thread::spawn(move || {
            let regular = staging.join("regular");
            let link = staging.join("link");
            while !stop.load(Ordering::Relaxed) {
                let _ = std::fs::write(&regular, b"{}");
                let _ = std::fs::rename(&regular, &inside);
                let _ = std::os::unix::fs::symlink(&target, &link);
                let _ = std::fs::rename(&link, &inside);
            }
        })
    };

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut attempts = 0u64;
    let mut followed = 0u64;
    while Instant::now() < deadline && followed == 0 {
        attempts += 1;
        if load(&directory, "fixture").is_ok() {
            followed += 1;
        }
    }
    stop.store(true, Ordering::Relaxed);
    swapper.join().expect("swapper");
    assert_eq!(
        followed,
        0,
        "load read {} through a link out of {} after checking a regular file ({attempts} attempts)",
        target.display(),
        directory.display()
    );
}
