//! Explicit AEP source builds and receipt-checked selection, isolated from ambient installations.

use super::{Result, run};
use connectors_spec::{toolchain::SourcePin, v2::hash};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Pin {
    aep: String,
    source: SourcePin,
    patch_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format: String,
    pin: Pin,
    executable_sha256: String,
    cargo_lock_sha256: String,
    rustc: String,
    cargo: String,
    build_arguments: Vec<String>,
}

const PATCH: &[u8] = include_bytes!("../aep-findings.patch");
const FORMAT: &str = "connectors.aep-build-receipt/v1";

fn pin() -> Result<Pin> {
    let pin: Pin = serde_json::from_slice(include_bytes!("../aep-toolchain.json"))?;
    if pin.source.repository != "https://github.com/beyond10x/aep"
        || !hexadecimal(&pin.source.commit, 40)
        || pin.patch_sha256 != hash(PATCH)
        || pin.aep.split('.').count() != 3
        || pin
            .aep
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("invalid AEP source/version pin or patch digest".into());
    }
    Ok(pin)
}

fn hexadecimal(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn cache(root: &Path, pin: &Pin) -> PathBuf {
    root.join(".local/toolchains/aep")
        .join(format!("{}-{}", pin.source.commit, pin.patch_sha256))
        .join("bin/aep")
}

fn receipt_path(binary: &Path) -> PathBuf {
    let mut path = binary.as_os_str().to_owned();
    path.push(".receipt.json");
    path.into()
}

fn arguments() -> Vec<String> {
    [
        "build",
        "--locked",
        "--package",
        "aep-cli",
        "--bin",
        "aep",
        "--jobs",
        "2",
        "--profile",
        "dev",
        "--target-dir",
        "target",
    ]
    .map(str::to_owned)
    .to_vec()
}

fn receipt_matches(receipt: &Receipt, pin: &Pin, bytes: &[u8]) -> Result<()> {
    if receipt.format != FORMAT
        || &receipt.pin != pin
        || receipt.build_arguments != arguments()
        || !hexadecimal(&receipt.cargo_lock_sha256, 64)
        || receipt.rustc.is_empty()
        || receipt.cargo.is_empty()
        || receipt.executable_sha256 != hash(bytes)
    {
        return Err(
            "AEP build receipt does not match the pinned source, patch, build or executable digest"
                .into(),
        );
    }
    Ok(())
}

fn check(binary: &Path, pin: &Pin) -> Result<PathBuf> {
    let binary = binary.canonicalize()?;
    let receipt: Receipt = serde_json::from_slice(&fs::read(receipt_path(&binary))?)?;
    // Check the receipt and bytes before executing even --version.
    receipt_matches(&receipt, pin, &fs::read(&binary)?)?;
    let actual = run(Command::new(&binary).arg("--version"))?;
    if String::from_utf8(actual.stdout)?.trim() != format!("protocol {}", pin.aep) {
        return Err("AEP executable version disagrees with the source pin".into());
    }
    Ok(binary)
}

/// Flag, environment, local cache, then PATH; an explicit override never falls back.
pub fn resolve(root: &Path, requested: Option<&Path>) -> Result<PathBuf> {
    let pin = pin()?;
    let environment = std::env::var_os("CONNECTORS_AEP");
    let selected = requested.or_else(|| environment.as_deref().map(Path::new));
    if let Some(path) = selected {
        return check(path, &pin)
            .map_err(|error| format!("explicit AEP {}: {error}", path.display()).into());
    }
    let mut candidates = vec![cache(root, &pin)];
    candidates.extend(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|path| path.join("aep")),
    );
    let mut errors = Vec::new();
    for path in candidates {
        match check(&path, &pin) {
            Ok(binary) => return Ok(binary),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    Err(format!("AEP {} at {} with patch {} is required; build with connectors-build aep-toolchain --source <local-aep-repository>. Searched: {}",
        pin.aep, pin.source.commit, pin.patch_sha256, errors.join("; ")).into())
}

fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.args(["--no-replace-objects", "-C"]).arg(root);
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(key);
    }
    command
}

/// Build only the exact committed Git objects plus the digest-pinned patch, never working bytes.
/// The local receipt records this build; it is not an upstream signature or a reproducibility proof.
pub fn build(root: &Path, source: &Path) -> Result<PathBuf> {
    let pin = pin()?;
    let source = source.canonicalize()?;
    let binary = cache(root, &pin);
    if binary.exists() {
        return check(&binary, &pin);
    }
    let destination = binary
        .parent()
        .and_then(Path::parent)
        .ok_or("invalid AEP cache")?;
    if destination.exists() {
        return Err("incomplete AEP cache exists; inspect and preserve it before retrying".into());
    }
    let parent = destination.parent().ok_or("invalid AEP cache parent")?;
    fs::create_dir_all(parent)?;
    let temp = tempfile::Builder::new()
        .prefix(".build-")
        .tempdir_in(parent)?;
    let snapshot = temp.path().join("source");
    run(git(temp.path())
        .args([
            "clone",
            "--no-local",
            "--no-hardlinks",
            "--no-checkout",
            "--",
        ])
        .arg(source)
        .arg(&snapshot))?;
    run(git(&snapshot)
        .args(["checkout", "--detach"])
        .arg(&pin.source.commit))?;
    let tree = run(git(&snapshot).args(["ls-tree", "-r", "HEAD"]))?;
    if tree
        .stdout
        .split(|b| *b == b'\n')
        .any(|line| line.starts_with(b"160000 "))
    {
        return Err("AEP submodules require an explicit recursive source policy".into());
    }
    let patch = temp.path().join("findings.patch");
    fs::write(&patch, PATCH)?;
    run(git(&snapshot).args(["apply", "--check"]).arg(&patch))?;
    run(git(&snapshot).arg("apply").arg(&patch))?;
    let lock = hash(&fs::read(snapshot.join("Cargo.lock"))?);
    let rustc = String::from_utf8(
        run(Command::new("rustc")
            .current_dir(&snapshot)
            .arg("--version"))?
        .stdout,
    )?;
    let cargo = String::from_utf8(
        run(Command::new("cargo")
            .current_dir(&snapshot)
            .arg("--version"))?
        .stdout,
    )?;
    let log = fs::File::create(temp.path().join("build.log"))?;
    let status = Command::new("cargo")
        .current_dir(&snapshot)
        .args(arguments())
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET")
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "2")
        .stdout(log.try_clone()?)
        .stderr(log)
        .status()?;
    if !status.success() {
        let retained = temp.keep();
        return Err(format!(
            "AEP build failed: {status}; retained {}",
            retained.display()
        )
        .into());
    }
    if lock != hash(&fs::read(snapshot.join("Cargo.lock"))?) {
        return Err("AEP build changed Cargo.lock".into());
    }
    let built = snapshot.join("target/debug/aep");
    let receipt = Receipt {
        format: FORMAT.into(),
        pin: pin.clone(),
        executable_sha256: hash(&fs::read(&built)?),
        cargo_lock_sha256: lock,
        rustc: rustc.trim().into(),
        cargo: cargo.trim().into(),
        build_arguments: arguments(),
    };
    let installation = temp.path().join("installation");
    fs::create_dir_all(installation.join("bin"))?;
    let staged = installation.join("bin/aep");
    fs::copy(built, &staged)?;
    fs::write(receipt_path(&staged), serde_json::to_vec_pretty(&receipt)?)?;
    check(&staged, &pin)?;
    fs::rename(installation, destination)?;
    check(&binary, &pin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_patch_and_binary_must_all_match_before_execution() {
        let pin = pin().unwrap();
        let bytes = b"not an executable; receipt verification must not execute it";
        let mut receipt = Receipt {
            format: FORMAT.into(),
            pin: pin.clone(),
            executable_sha256: hash(bytes),
            cargo_lock_sha256: "a".repeat(64),
            rustc: "rustc test".into(),
            cargo: "cargo test".into(),
            build_arguments: arguments(),
        };
        receipt_matches(&receipt, &pin, bytes).unwrap();
        assert!(
            receipt_matches(&receipt, &pin, b"changed")
                .unwrap_err()
                .to_string()
                .contains("executable digest")
        );
        receipt.pin.patch_sha256 = "0".repeat(64);
        assert!(
            receipt_matches(&receipt, &pin, bytes)
                .unwrap_err()
                .to_string()
                .contains("pinned source")
        );
        receipt.pin = pin.clone();
        receipt.pin.source.commit = "0".repeat(40);
        assert!(
            receipt_matches(&receipt, &pin, bytes)
                .unwrap_err()
                .to_string()
                .contains("pinned source")
        );
    }
}
