//! Adversarial checks of the documented exact-source admission boundary.
use connectors_spec::toolchain;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

// Keep script creation and execution together; concurrent forks can retain an
// open writable fixture descriptor briefly on Linux.
static FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fixture(root: &Path) -> (PathBuf, PathBuf, Value) {
    let executable = root.join("ess");
    let marker = root.join("executed");
    let version = toolchain::version().unwrap();
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf 'executed' > '{}'\nprintf '%s\\n' '{}'\n",
            marker.display(),
            version
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let pin = toolchain::pin().unwrap();
    let receipt = json!({
        "format": "connectors.ess-build-receipt/v1",
        "source": pin.source.unwrap(),
        "ess": pin.ess,
        "executable_sha256": hex::encode(Sha256::digest(fs::read(&executable).unwrap())),
        "cargo_lock_sha256": "1".repeat(64),
        "rustc": "fixture rustc",
        "cargo": "fixture cargo",
        "build_arguments": ["build", "--locked", "--package", "ess-cli", "--bin", "ess", "--jobs", "2", "--profile", "dev", "--target-dir", "target"]
    });
    (executable, marker, receipt)
}

fn write_receipt(executable: &Path, receipt: &Value) {
    fs::write(
        toolchain::receipt_path(executable),
        serde_json::to_vec(receipt).unwrap(),
    )
    .unwrap();
}

#[test]
fn malformed_or_wrong_identity_receipt_never_executes_matching_bytes() {
    let _guard = FIXTURES.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (executable, marker, valid) = fixture(temp.path());
    assert!(toolchain::check(&executable).is_err());
    assert!(!marker.exists(), "missing receipt ran --version");

    for (pointer, replacement) in [
        ("/source/repository", json!("https://example.invalid/other")),
        ("/source/commit", json!("0".repeat(40))),
        ("/ess", json!("99.0.0")),
        ("/format", json!("connectors.ess-build-receipt/v2")),
        ("/build_arguments", json!(["build", "--release"])),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        write_receipt(&executable, &invalid);
        assert!(toolchain::check(&executable).is_err(), "admitted {pointer}");
        assert!(!marker.exists(), "{pointer} ran before receipt refusal");
    }
    let mut extra = valid.clone();
    extra["source"]["unrecognized"] = json!(true);
    write_receipt(&executable, &extra);
    assert!(toolchain::check(&executable).is_err());
    assert!(!marker.exists(), "unknown nested field ran --version");

    write_receipt(&executable, &valid);
    toolchain::check(&executable).unwrap();
    assert!(marker.exists(), "valid control never executed");
}

#[test]
fn symlink_alias_cannot_supply_a_receipt_for_an_unproven_target() {
    let _guard = FIXTURES.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (executable, marker, valid) = fixture(temp.path());
    let alias = temp.path().join("ess-link");
    symlink(&executable, &alias).unwrap();
    write_receipt(&alias, &valid);
    assert!(toolchain::check(&alias).is_err());
    assert!(toolchain::resolve(Some(&alias)).is_err());
    assert!(!marker.exists(), "alias receipt admitted unproven target");

    fs::remove_file(toolchain::receipt_path(&alias)).unwrap();
    write_receipt(&executable, &valid);
    assert_eq!(
        toolchain::resolve(Some(&alias)).unwrap(),
        executable.canonicalize().unwrap()
    );
    assert!(marker.exists(), "canonical target receipt was not used");
}

#[test]
fn explicit_wrong_source_refuses_even_when_it_reports_the_pinned_release() {
    let _guard = FIXTURES.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (executable, marker, mut receipt) = fixture(temp.path());
    receipt["source"]["commit"] = json!("0".repeat(40));
    write_receipt(&executable, &receipt);
    let error = toolchain::resolve(Some(&executable)).unwrap_err();
    assert!(error.to_string().contains("pinned ESS source"));
    assert!(!marker.exists(), "wrong source was executed");
    assert!(toolchain::resolve(Some(Path::new(""))).is_err());
}

#[test]
fn source_pin_parser_preserves_legacy_and_rejects_incomplete_source_records() {
    assert!(
        toolchain::parse_pin(br#"{"ess":"0.20.0"}"#)
            .unwrap()
            .source
            .is_none()
    );
    let valid = serde_json::to_value(toolchain::pin().unwrap()).unwrap();
    for replacement in [
        json!({}),
        json!({"repository": "https://example.invalid/ess"}),
        json!({"commit": "0".repeat(40)}),
        json!({"repository": " ", "commit": "0".repeat(40)}),
        json!({"repository": "https://example.invalid/ess", "commit": "main"}),
        json!({"repository": "https://example.invalid/ess", "commit": "0".repeat(39)}),
        json!({"repository": "https://example.invalid/ess", "commit": "0".repeat(41)}),
    ] {
        let mut invalid = valid.clone();
        invalid["source"] = replacement;
        assert!(
            toolchain::parse_pin(&serde_json::to_vec(&invalid).unwrap()).is_err(),
            "admitted incomplete exact source: {invalid}"
        );
    }
}
