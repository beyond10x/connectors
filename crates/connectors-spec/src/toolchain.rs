//! Repository-owned ESS selection by released version. Resolution never downloads or changes PATH.
use connectors_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub ess: String,
}

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
    Ok(pin)
}
pub fn pin() -> Result<Pin> {
    parse_pin(include_bytes!("../toolchain.json"))
}
pub fn version() -> Result<String> {
    Ok(format!("ess {}", pin()?.ess))
}
pub fn pin_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("toolchain.json")
}
pub fn check(executable: &Path) -> Result<()> {
    let executable = executable
        .canonicalize()
        .map_err(|e| Error::invalid(format!("{}: {e}", executable.display())))?;
    check_with_pin(&executable, &pin()?).map_err(Error::invalid)
}
fn check_with_pin(executable: &Path, pin: &Pin) -> std::result::Result<(), String> {
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
/// Flag > CONNECTORS_ESS > PATH. Explicit paths never fall back.
/// A bare executable name searches PATH for the exact pinned release.
pub fn resolve(requested: Option<&Path>) -> Result<PathBuf> {
    resolve_in(
        requested,
        std::env::var_os("CONNECTORS_ESS").as_deref(),
        std::env::var_os("PATH").as_deref().unwrap_or_default(),
    )
}
fn resolve_in(
    requested: Option<&Path>,
    environment: Option<&OsStr>,
    search: &OsStr,
) -> Result<PathBuf> {
    let pin = pin()?;
    let selected = requested.or_else(|| environment.map(Path::new));
    if selected.is_some_and(|p| p.as_os_str().is_empty()) {
        return Err(Error::invalid(format!(
            "{}: empty ESS override",
            pin_path().display()
        )));
    }
    let candidates: Vec<PathBuf> = match selected {
        Some(path) if path.is_absolute() || path.components().count() > 1 => vec![path.to_owned()],
        Some(name) => std::env::split_paths(search)
            .map(|dir| dir.join(name))
            .collect(),
        None => std::env::split_paths(search)
            .map(|dir| dir.join("ess"))
            .collect(),
    };
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
        "{} requires ess {}; no matching ESS executable. Searched: {}. Install that release (`b10x upgrade`); --ess and CONNECTORS_ESS select an explicit executable.",
        pin_path().display(),
        pin.ess,
        observations.join("; "),
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
    fn the_pin_is_one_exact_release() {
        assert!(parse_pin(br#"{"ess":"0.40"}"#).is_err());
        assert!(parse_pin(br#"{"ess":"0.40.x"}"#).is_err());
        assert!(
            parse_pin(br#"{"ess":"0.40.0","source":{"repository":"r","commit":"c"}}"#).is_err()
        );
        parse_pin(br#"{"ess":"0.40.0"}"#).unwrap();
    }
    /// The executable pin and the `ess-*` libraries the build links must be one release:
    /// every workspace `ess-*` git dependency names the pinned tag, and every locked package
    /// from the ESS repository resolves that tag.
    #[test]
    fn the_ess_libraries_follow_the_pin() {
        const REPOSITORY: &str = "https://github.com/beyond10x/ess";
        let pinned = pin().unwrap().ess;
        let manifest = include_str!("../../../Cargo.toml");
        let dependencies: Vec<&str> = manifest
            .lines()
            .filter(|line| line.starts_with("ess-") && line.contains(REPOSITORY))
            .collect();
        assert_eq!(dependencies.len(), 7, "{dependencies:?}");
        for line in dependencies {
            assert!(
                line.contains(&format!("tag = \"{pinned}\"")),
                "Cargo.toml `{line}` does not name ESS {pinned}"
            );
        }
        let lock = include_str!("../../../Cargo.lock");
        let sources: Vec<&str> = lock
            .lines()
            .filter(|line| line.starts_with(&format!("source = \"git+{REPOSITORY}?")))
            .collect();
        assert!(!sources.is_empty());
        for line in sources {
            assert!(
                line.contains(&format!("?tag={pinned}#")),
                "Cargo.lock `{line}` does not resolve ESS {pinned}"
            );
        }
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
                resolve_in(None, None, &path).unwrap(),
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
            resolve_in(Some(&matching), Some(wrong.as_os_str()), search).unwrap(),
            matching.canonicalize().unwrap()
        );
        assert!(resolve_in(Some(&wrong), None, search).is_err());
        assert!(resolve_in(None, Some(wrong.as_os_str()), search).is_err());
        assert!(resolve_in(Some(Path::new("")), None, search).is_err());
        assert_eq!(
            resolve_in(None, Some(matching.as_os_str()), OsStr::new("")).unwrap(),
            matching.canonicalize().unwrap()
        );
    }
    #[test]
    fn a_missing_release_names_the_record_and_every_searched_path() {
        let _guard = FIXTURE_PROCESSES.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let wrong = binary(temp.path(), "wrong", "99.0.0");
        let error = resolve_in(None, None, wrong.parent().unwrap().as_os_str())
            .unwrap_err()
            .to_string();
        assert!(error.contains("toolchain.json"));
        assert!(error.contains(wrong.to_str().unwrap()));
        assert!(error.contains(&version().unwrap()));
    }
}
