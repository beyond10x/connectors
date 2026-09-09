//! Repository-owned tool selection. Resolution never downloads or changes PATH.
use connectors_core::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub repository: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub ess: String,
    /// Absent only in historical, release-version-only records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourcePin>,
}

const RECEIPT_FORMAT: &str = "connectors.ess-build-receipt/v1";
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildReceipt {
    pub format: String,
    pub source: SourcePin,
    pub ess: String,
    pub executable_sha256: String,
    pub cargo_lock_sha256: String,
    pub rustc: String,
    pub cargo: String,
    pub build_arguments: Vec<String>,
}

/// The legacy `{ "ess": "x.y.z" }` format remains readable, but a source pin
/// always requires a matching receipt as well as the release version.
pub fn parse_pin(bytes: &[u8]) -> Result<Pin> {
    let pin: Pin = connectors_core::read_json(bytes)?;
    if pin.ess.split('.').count() != 3
        || pin
            .ess
            .split('.')
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(Error::invalid(
            "invalid ESS version in crates/connectors-spec/toolchain.json",
        ));
    }
    if let Some(source) = &pin.source
        && (source.repository.is_empty()
            || source.repository.chars().any(char::is_whitespace)
            || !is_hex(&source.commit, 40))
    {
        return Err(Error::invalid(
            "invalid exact ESS source pin in toolchain.json",
        ));
    }
    Ok(pin)
}
pub fn pin() -> Result<Pin> {
    parse_pin(include_bytes!("../toolchain.json"))
}
pub fn source() -> Result<Option<SourcePin>> {
    Ok(pin()?.source)
}
pub fn version() -> Result<String> {
    Ok(format!("ess {}", pin()?.ess))
}
pub fn pin_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("toolchain.json")
}
pub fn cache_path() -> Result<PathBuf> {
    let pin = pin()?;
    let identity = pin.source.as_ref().map_or(pin.ess.as_str(), |s| &s.commit);
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/toolchains/ess")
        .join(identity)
        .join("bin/ess"))
}
/// Receipts travel with the actual executable, including through PATH symlinks.
pub fn receipt_path(executable: &Path) -> PathBuf {
    let mut name = executable.as_os_str().to_owned();
    name.push(".receipt.json");
    PathBuf::from(name)
}
pub fn check(executable: &Path) -> Result<()> {
    let executable = executable
        .canonicalize()
        .map_err(|e| Error::invalid(format!("{}: {e}", executable.display())))?;
    check_with_pin(&executable, &pin()?).map_err(Error::invalid)
}
fn check_with_pin(executable: &Path, pin: &Pin) -> std::result::Result<(), String> {
    // An unproven executable must not run, even just to print its version.
    if let Some(source) = &pin.source {
        let path = receipt_path(executable);
        let bytes = fs::read(&path).map_err(|e| format!("receipt {}: {e}", path.display()))?;
        let receipt: BuildReceipt = connectors_core::read_json(&bytes)
            .map_err(|e| format!("receipt {}: {e}", path.display()))?;
        if receipt.format != RECEIPT_FORMAT
            || receipt.source != *source
            || receipt.ess != pin.ess
            || !is_hex(&receipt.executable_sha256, 64)
            || !is_hex(&receipt.cargo_lock_sha256, 64)
            || receipt.rustc.is_empty()
            || receipt.cargo.is_empty()
            || receipt.build_arguments != build_arguments()
        {
            return Err(format!(
                "receipt {} does not match pinned ESS source {}",
                path.display(),
                source.commit
            ));
        }
        let digest = file_digest(executable).map_err(|e| e.to_string())?;
        if digest != receipt.executable_sha256 {
            return Err(format!(
                "receipt {}: executable SHA-256 mismatch",
                path.display()
            ));
        }
    }
    let expected = format!("ess {}", pin.ess);
    let actual = probe(executable)?;
    if actual != expected {
        return Err(format!(
            "{} requires {expected}; {} reports {actual}",
            pin_path().display(),
            executable.display()
        ));
    }
    Ok(())
}
fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn file_digest(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).map_err(|e| Error::invalid(format!("{}: {e}", path.display())))?;
    let mut digest = Sha256::new();
    let mut bytes = [0; 64 * 1024];
    loop {
        let read = file
            .read(&mut bytes)
            .map_err(|e| Error::invalid(format!("{}: {e}", path.display())))?;
        if read == 0 {
            break;
        }
        digest.update(&bytes[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}
fn probe(path: &Path) -> std::result::Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{}: --version exited {}",
            path.display(),
            output.status
        ));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(|_| format!("{}: invalid version output", path.display()))
}
/// Flag > CONNECTORS_ESS > checkout cache > PATH. Explicit paths never fall back.
/// A bare executable name searches PATH for the exact pinned identity.
pub fn resolve(requested: Option<&Path>) -> Result<PathBuf> {
    resolve_in(
        requested,
        std::env::var_os("CONNECTORS_ESS").as_deref(),
        std::env::var_os("PATH").as_deref().unwrap_or_default(),
        &cache_path()?,
    )
}
fn resolve_in(
    requested: Option<&Path>,
    environment: Option<&OsStr>,
    search: &OsStr,
    cached: &Path,
) -> Result<PathBuf> {
    let pin = pin()?;
    let expected = match &pin.source {
        Some(source) => format!("ess {} at {}", pin.ess, source.commit),
        None => format!("ess {}", pin.ess),
    };
    let selected = requested.or_else(|| environment.map(Path::new));
    if selected.is_some_and(|p| p.as_os_str().is_empty()) {
        return Err(Error::invalid(format!(
            "{}: empty ESS override",
            pin_path().display()
        )));
    }
    let mut candidates = Vec::new();
    match selected {
        Some(path) if path.is_absolute() || path.components().count() > 1 => {
            candidates.push(path.to_owned())
        }
        Some(name) => candidates.extend(std::env::split_paths(search).map(|dir| dir.join(name))),
        None => {
            candidates.push(cached.to_owned());
            candidates.extend(std::env::split_paths(search).map(|dir| dir.join("ess")));
        }
    }
    let mut seen = BTreeSet::new();
    let mut observations = Vec::new();
    for path in candidates {
        let path = match path.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                observations.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        match check_with_pin(&path, &pin) {
            Ok(()) => return Ok(path),
            Err(error) => observations.push(format!("{}: {error}", path.display())),
        }
    }
    Err(Error::invalid(format!(
        "{} requires {expected}; no matching ESS executable. Searched: {}. Build the pinned source with connectors-build toolchain --source <checkout> (cache: {}), or install the historical pinned release when no source is selected; --ess and CONNECTORS_ESS select an explicit executable.",
        pin_path().display(),
        observations.join("; "),
        cached.display()
    )))
}

fn build_arguments() -> Vec<String> {
    [
        "build",
        "--locked",
        "--package",
        "ess-cli",
        "--bin",
        "ess",
        "--jobs",
        "2",
        "--profile",
        "dev",
        "--target-dir",
        "target",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
fn git_command(root: &Path) -> Command {
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
fn captured(command: &mut Command, label: &str) -> Result<Vec<u8>> {
    let output = command
        .output()
        .map_err(|e| Error::invalid(format!("{label}: {e}")))?;
    if !output.status.success() {
        return Err(Error::invalid(format!(
            "{label} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}
fn output_text(command: &mut Command, label: &str) -> Result<String> {
    String::from_utf8(captured(command, label)?)
        .map(|s| s.trim().to_owned())
        .map_err(|_| Error::invalid(format!("{label}: non-UTF-8 output")))
}
fn verify_checkout(root: &Path, source: &SourcePin) -> Result<()> {
    let top = output_text(
        git_command(root).args(["rev-parse", "--show-toplevel"]),
        "locate ESS source root",
    )?;
    if Path::new(&top)
        .canonicalize()
        .map_err(|_| Error::invalid("invalid ESS source root"))?
        != root
    {
        return Err(Error::invalid("ESS source must name the repository root"));
    }
    let head = output_text(
        git_command(root).args(["rev-parse", "HEAD"]),
        "read ESS source HEAD",
    )?;
    if head != source.commit {
        return Err(Error::invalid(format!(
            "ESS source HEAD {head} does not match pin {}",
            source.commit
        )));
    }
    let status = captured(
        git_command(root).args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ]),
        "inspect ESS source",
    )?;
    if !status.is_empty() {
        return Err(Error::invalid(
            "ESS source has tracked changes or nonignored untracked files; commit or preserve them before building",
        ));
    }
    let entries = captured(
        git_command(root).args(["ls-tree", "-r", "HEAD"]),
        "inspect ESS source tree",
    )?;
    if entries
        .split(|b| *b == b'\n')
        .any(|line| line.starts_with(b"160000 "))
    {
        return Err(Error::invalid(
            "ESS source with submodules requires an explicit recursive provenance policy",
        ));
    }
    Ok(())
}

/// Explicit local source build, never called during resolution or ordinary builds.
///
/// The committed source is copied into an independent temporary Git clone; ignored
/// files in the supplied checkout cannot become build inputs. The receipt is a
/// local build observation, not a signed upstream attestation or a reproducibility
/// claim. Source and Cargo dependencies must already be trusted for execution.
pub fn build_from_source(source_checkout: &Path) -> Result<PathBuf> {
    let pin = pin()?;
    let source = pin.source.as_ref().ok_or_else(|| {
        Error::invalid("source build requires an exact source pin in toolchain.json")
    })?;
    let source_checkout = source_checkout
        .canonicalize()
        .map_err(|e| Error::invalid(format!("ESS source: {e}")))?;
    verify_checkout(&source_checkout, source)?;
    let executable = cache_path()?;
    // Never replace an existing installation or manufacture a receipt for it.
    if executable
        .try_exists()
        .map_err(|e| Error::invalid(format!("ESS cache: {e}")))?
    {
        check(&executable)?;
        return executable
            .canonicalize()
            .map_err(|e| Error::invalid(format!("ESS cache: {e}")));
    }
    let destination = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(Error::internal)?;
    if destination
        .try_exists()
        .map_err(|e| Error::invalid(format!("ESS cache: {e}")))?
    {
        return Err(Error::invalid(format!(
            "ESS cache {} already exists without a usable executable; preserve and inspect it before retrying",
            destination.display()
        )));
    }
    let parent = destination.parent().ok_or_else(Error::internal)?;
    fs::create_dir_all(parent).map_err(|e| Error::invalid(format!("create ESS cache: {e}")))?;
    let work = tempfile::Builder::new()
        .prefix(".build-")
        .tempdir_in(parent)
        .map_err(|e| Error::invalid(format!("create ESS build workspace: {e}")))?;
    let snapshot = work.path().join("source");
    captured(
        git_command(work.path())
            .args([
                "clone",
                "--no-local",
                "--no-checkout",
                "--no-hardlinks",
                "--",
            ])
            .arg(&source_checkout)
            .arg(&snapshot),
        "clone committed ESS source",
    )?;
    // The cache path is derived relative to this crate and can contain `..`.
    // Compare the cloned root by the same canonical identity as the input root.
    let snapshot = snapshot
        .canonicalize()
        .map_err(|e| Error::invalid(format!("locate cloned ESS source: {e}")))?;
    captured(
        git_command(&snapshot)
            .args(["checkout", "--detach"])
            .arg(&source.commit),
        "checkout pinned ESS source",
    )?;
    verify_checkout(&snapshot, source)?;
    let lock_digest = file_digest(&snapshot.join("Cargo.lock"))?;
    let rustc = output_text(
        Command::new("rustc")
            .current_dir(&snapshot)
            .arg("--version"),
        "identify rustc",
    )?;
    let cargo = output_text(
        Command::new("cargo")
            .current_dir(&snapshot)
            .arg("--version"),
        "identify cargo",
    )?;
    let log_path = work.path().join("build.log");
    let log = fs::File::create(&log_path)
        .map_err(|e| Error::invalid(format!("create ESS build log: {e}")))?;
    let status = Command::new("cargo")
        .current_dir(&snapshot)
        .args(build_arguments())
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET")
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "2")
        .stdout(log.try_clone().map_err(|_| Error::internal())?)
        .stderr(log)
        .status()
        .map_err(|e| Error::invalid(format!("build pinned ESS: {e}")))?;
    if !status.success() {
        let retained = work.keep();
        return Err(Error::invalid(format!(
            "build pinned ESS exited {status}; retained workspace and log: {}",
            retained.display()
        )));
    }
    verify_checkout(&source_checkout, source)?;
    verify_checkout(&snapshot, source)?;
    if file_digest(&snapshot.join("Cargo.lock"))? != lock_digest {
        return Err(Error::invalid("ESS source build changed Cargo.lock"));
    }
    let built = snapshot.join("target/debug/ess");
    let actual = probe(&built).map_err(Error::invalid)?;
    if actual != format!("ess {}", pin.ess) {
        return Err(Error::invalid(format!(
            "built ESS reports {actual}, expected ess {}",
            pin.ess
        )));
    }
    let receipt = BuildReceipt {
        format: RECEIPT_FORMAT.into(),
        source: source.clone(),
        ess: pin.ess.clone(),
        executable_sha256: file_digest(&built)?,
        cargo_lock_sha256: lock_digest,
        rustc,
        cargo,
        build_arguments: build_arguments(),
    };
    let installation = work.path().join("installation");
    fs::create_dir_all(installation.join("bin"))
        .map_err(|e| Error::invalid(format!("stage ESS installation: {e}")))?;
    let staged = installation.join("bin/ess");
    fs::copy(&built, &staged).map_err(|e| Error::invalid(format!("stage ESS executable: {e}")))?;
    fs::write(
        receipt_path(&staged),
        serde_json::to_vec_pretty(&receipt).map_err(|_| Error::internal())?,
    )
    .map_err(|e| Error::invalid(format!("stage ESS receipt: {e}")))?;
    fs::copy(&log_path, installation.join("build.log"))
        .map_err(|e| Error::invalid(format!("retain ESS build log: {e}")))?;
    check_with_pin(&staged, &pin).map_err(Error::invalid)?;
    // Rename a complete directory on one filesystem; readers never see half an install.
    fs::rename(&installation, destination)
        .map_err(|e| Error::invalid(format!("install ESS cache: {e}")))?;
    executable
        .canonicalize()
        .map_err(|e| Error::invalid(format!("resolve installed ESS: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    // A sibling fork can inherit a fixture's writable fd until exec closes it,
    // making another test's exec fail with ETXTBSY. Cover both writes and probes.
    static FIXTURE_PROCESSES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn binary(root: &Path, name: &str, version: &str) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ess");
        std::fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' 'ess {version}'\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        receipt(&path);
        path
    }
    fn receipt(path: &Path) {
        let pin = pin().unwrap();
        fs::write(
            receipt_path(path),
            serde_json::to_vec(&BuildReceipt {
                format: RECEIPT_FORMAT.into(),
                source: pin.source.unwrap(),
                ess: pin.ess,
                executable_sha256: file_digest(path).unwrap(),
                cargo_lock_sha256: "a".repeat(64),
                rustc: "fixture rustc".into(),
                cargo: "fixture cargo".into(),
                build_arguments: build_arguments(),
            })
            .unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn exact_source_pin_refuses_same_version_without_receipt() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let executable = binary(temp.path(), "unproven", &pin().unwrap().ess);
        fs::remove_file(receipt_path(&executable)).unwrap();
        let error = check(&executable).expect_err("version alone cannot prove an exact source pin");
        assert!(error.to_string().contains("receipt"));
    }
    #[test]
    fn exact_source_pin_rejects_wrong_source_with_the_same_version() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let executable = binary(temp.path(), "wrong-source", &pin().unwrap().ess);
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(receipt_path(&executable)).unwrap()).unwrap();
        record["source"]["commit"] = serde_json::json!("0".repeat(40));
        fs::write(
            receipt_path(&executable),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        assert!(
            check(&executable)
                .unwrap_err()
                .to_string()
                .contains("pinned ESS source")
        );
        let matching = binary(temp.path(), "matching", &pin().unwrap().ess);
        assert!(
            resolve_in(
                Some(&executable),
                None,
                matching.parent().unwrap().as_os_str(),
                &matching
            )
            .is_err()
        );
        let search =
            std::env::join_paths([executable.parent().unwrap(), matching.parent().unwrap()])
                .unwrap();
        assert_eq!(
            resolve_in(None, None, &search, &temp.path().join("absent")).unwrap(),
            matching.canonicalize().unwrap()
        );
    }
    #[test]
    fn altered_binary_is_refused_before_it_can_execute() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let executable = binary(temp.path(), "altered", &pin().unwrap().ess);
        let marker = temp.path().join("was-executed");
        fs::write(
            &executable,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        assert!(
            check(&executable)
                .unwrap_err()
                .to_string()
                .contains("SHA-256 mismatch")
        );
        assert!(!marker.exists());
    }
    #[test]
    fn invalid_receipts_are_closed_and_cannot_admit_a_tool() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let executable = binary(temp.path(), "receipt-cases", &pin().unwrap().ess);
        let valid: serde_json::Value =
            serde_json::from_slice(&fs::read(receipt_path(&executable)).unwrap()).unwrap();
        for field in [
            "format",
            "ess",
            "executable_sha256",
            "cargo_lock_sha256",
            "rustc",
            "cargo",
            "build_arguments",
            "unknown",
        ] {
            let mut record = valid.clone();
            record[field] = serde_json::json!("");
            fs::write(
                receipt_path(&executable),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
            assert!(check(&executable).is_err(), "admitted {field}");
        }
        receipt(&executable);
        check(&executable).unwrap();
    }
    #[test]
    fn legacy_release_records_remain_explicit_and_source_records_are_strict() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let executable = binary(temp.path(), "legacy", "0.20.0");
        fs::remove_file(receipt_path(&executable)).unwrap();
        let legacy = parse_pin(br#"{"ess":"0.20.0"}"#).unwrap();
        assert!(legacy.source.is_none());
        check_with_pin(&executable, &legacy).unwrap();
        for bytes in [br#"{"ess":"0.20"}"#.as_slice(), br#"{"ess":"0.20.0","unknown":true}"#, br#"{"ess":"0.20.0","source":{"repository":"x","commit":"main"}}"#, br#"{"ess":"0.20.0","source":{"repository":"x","commit":"0000000000000000000000000000000000000000","unknown":true}}"#] {
            assert!(parse_pin(bytes).is_err());
        }
    }
    fn fixture_repository(root: &Path) -> SourcePin {
        use std::io::Write;
        use std::process::Stdio;
        captured(git_command(root).arg("init"), "fixture init").unwrap();
        let mut tree = git_command(root)
            .args(["hash-object", "-t", "tree", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        drop(tree.stdin.take());
        let tree = String::from_utf8(tree.wait_with_output().unwrap().stdout).unwrap();
        let object = format!(
            "tree {}\nauthor Fixture <fixture@example.invalid> 0 +0000\ncommitter Fixture <fixture@example.invalid> 0 +0000\n\nsource admission fixture\n",
            tree.trim()
        );
        let mut commit = git_command(root)
            .args(["hash-object", "-t", "commit", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        commit
            .stdin
            .take()
            .unwrap()
            .write_all(object.as_bytes())
            .unwrap();
        let commit = String::from_utf8(commit.wait_with_output().unwrap().stdout)
            .unwrap()
            .trim()
            .to_owned();
        captured(
            git_command(root).args(["update-ref", "HEAD", &commit]),
            "fixture ref",
        )
        .unwrap();
        SourcePin {
            repository: "https://example.invalid/fixture".into(),
            commit,
        }
    }
    #[test]
    fn source_admission_requires_the_exact_clean_root_and_keeps_ignored_scratch_separate() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let source = fixture_repository(&root);
        verify_checkout(&root, &source).unwrap();
        let mut wrong = source.clone();
        wrong.commit = "0".repeat(40);
        assert!(
            verify_checkout(&root, &wrong)
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );
        fs::write(root.join("untracked.rs"), "source").unwrap();
        assert!(
            verify_checkout(&root, &source)
                .unwrap_err()
                .to_string()
                .contains("untracked")
        );
        captured(
            git_command(&root).args(["add", "untracked.rs"]),
            "stage fixture source",
        )
        .unwrap();
        assert!(verify_checkout(&root, &source).is_err());
        captured(
            git_command(&root).args(["reset", "--hard", "HEAD"]),
            "reset fixture source",
        )
        .unwrap();
        fs::write(root.join(".git/info/exclude"), ".local/\n").unwrap();
        fs::create_dir(root.join(".local")).unwrap();
        fs::write(root.join(".local/source.rs"), "ignored scratch").unwrap();
        verify_checkout(&root, &source).unwrap();
        assert!(
            verify_checkout(&root.join(".local"), &source)
                .unwrap_err()
                .to_string()
                .contains("repository root")
        );
    }
    #[test]
    fn default_search_is_independent_of_path_version_order() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let matching = binary(temp.path(), "matching", &pin().unwrap().ess);
        let wrong = binary(temp.path(), "wrong", "99.0.0");
        for dirs in [
            [wrong.parent().unwrap(), matching.parent().unwrap()],
            [matching.parent().unwrap(), wrong.parent().unwrap()],
        ] {
            let path = std::env::join_paths(dirs).unwrap();
            assert_eq!(
                resolve_in(None, None, &path, &temp.path().join("absent")).unwrap(),
                matching.canonicalize().unwrap()
            );
        }
    }
    #[test]
    fn explicit_overrides_win_and_wrong_paths_do_not_fall_back() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let matching = binary(temp.path(), "matching", &pin().unwrap().ess);
        let wrong = binary(temp.path(), "wrong", "99.0.0");
        let search = matching.parent().unwrap().as_os_str();
        assert_eq!(
            resolve_in(Some(&matching), Some(wrong.as_os_str()), search, &matching).unwrap(),
            matching.canonicalize().unwrap()
        );
        assert!(resolve_in(Some(&wrong), None, search, &matching).is_err());
        assert!(resolve_in(None, Some(wrong.as_os_str()), search, &matching).is_err());
        assert_eq!(
            resolve_in(None, Some(matching.as_os_str()), OsStr::new(""), &wrong).unwrap(),
            matching.canonicalize().unwrap()
        );
    }
    #[test]
    fn missing_pin_reports_record_and_search_locations_and_cache_is_supported() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let wrong = binary(temp.path(), "wrong", "99.0.0");
        let cache = temp.path().join("cache/ess");
        let error = resolve_in(None, None, wrong.parent().unwrap().as_os_str(), &cache)
            .unwrap_err()
            .to_string();
        assert!(error.contains("toolchain.json"));
        assert!(error.contains(wrong.to_str().unwrap()));
        assert!(error.contains(cache.to_str().unwrap()));
        assert!(error.contains(&version().unwrap()));
        let matching = binary(temp.path(), "cache", &pin().unwrap().ess);
        assert_eq!(
            resolve_in(None, None, wrong.parent().unwrap().as_os_str(), &matching).unwrap(),
            matching.canonicalize().unwrap()
        );
    }
    #[test]
    fn committed_manifest_uses_the_repository_pin() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../adapters/gitlab/generated/manifest.json"
        ))
        .unwrap();
        assert_eq!(manifest["ess"], version().unwrap());
        assert_eq!(
            manifest["ess_source"],
            serde_json::to_value(source().unwrap()).unwrap()
        );
    }
}
