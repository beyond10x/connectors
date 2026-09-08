//! Repository-owned tool selection. Resolution never downloads or changes PATH.
use connectors_core::{Error, Result};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub ess: String,
}
pub fn pin() -> Result<Pin> {
    let pin: Pin = connectors_core::read_json(include_bytes!("../toolchain.json"))?;
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
    Ok(pin)
}
pub fn version() -> Result<String> {
    Ok(format!("ess {}", pin()?.ess))
}
pub fn pin_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("toolchain.json")
}
pub fn cache_path() -> Result<PathBuf> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/toolchains/ess")
        .join(pin()?.ess)
        .join("bin/ess"))
}
pub fn check(executable: &Path) -> Result<()> {
    let expected = version()?;
    let actual = probe(executable).map_err(Error::invalid)?;
    if actual != expected {
        return Err(Error::invalid(format!(
            "{} requires {expected}; {} reports {actual}",
            pin_path().display(),
            executable.display()
        )));
    }
    Ok(())
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
/// A bare executable name searches PATH for the exact pinned version.
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
    let expected = version()?;
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
        match probe(&path) {
            Ok(actual) if actual == expected => return Ok(path),
            Ok(actual) => observations.push(format!("{}: found {actual}", path.display())),
            Err(error) => observations.push(error),
        }
    }
    Err(Error::invalid(format!(
        "{} requires {expected}; no matching ESS executable. Searched: {}. Install the pinned release in {} or on PATH; --ess and CONNECTORS_ESS select an explicit executable.",
        pin_path().display(),
        observations.join("; "),
        cached.display()
    )))
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
        path
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
    }
}
