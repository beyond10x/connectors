use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest as _, Sha256};

#[derive(Deserialize)]
struct Bundle {
    files: Vec<BundleFile>,
}

#[derive(Deserialize)]
struct BundleFile {
    path: String,
    bytes: usize,
    sha256: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn check(relative: &str) {
    let root = root();
    let bundle: Bundle =
        serde_json::from_slice(&fs::read(root.join(relative)).expect("bundle is readable"))
            .expect("bundle parses");
    for entry in bundle.files {
        let bytes = fs::read(root.join(&entry.path))
            .unwrap_or_else(|error| panic!("{} is readable: {error}", entry.path));
        assert_eq!(bytes.len(), entry.bytes, "{} byte length", entry.path);
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            entry.sha256,
            "{} digest",
            entry.path
        );
    }
}

#[test]
fn owner_contract_bundle_is_immutable() {
    check("contracts/voice-session/v0alpha1/bundle.json");
}

#[test]
fn rtvbp_binding_bundle_is_immutable() {
    check("fixtures/rtvbp-voice-binding/v1/bundle.json");
}
