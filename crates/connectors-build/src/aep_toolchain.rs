//! AEP selection by released version: the installed `aep` must report exactly the pinned release.

use super::{Result, run};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Pin {
    aep: String,
}

fn parse(bytes: &[u8]) -> Result<Pin> {
    let pin: Pin = serde_json::from_slice(bytes)?;
    if pin.aep.split('.').count() != 3
        || pin
            .aep
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("invalid AEP release pin in crates/connectors-build/aep-toolchain.json".into());
    }
    Ok(pin)
}

fn pin() -> Result<Pin> {
    parse(include_bytes!("../aep-toolchain.json"))
}

fn check(binary: &Path, pin: &Pin) -> Result<PathBuf> {
    let binary = binary.canonicalize()?;
    let actual = run(Command::new(&binary).arg("--version"))?;
    if String::from_utf8(actual.stdout)?.trim() != format!("aep {}", pin.aep) {
        return Err(format!("AEP executable is not the pinned release {}", pin.aep).into());
    }
    Ok(binary)
}

/// Flag, environment, then PATH; an explicit override never falls back.
pub fn resolve(requested: Option<&Path>) -> Result<PathBuf> {
    let pin = pin()?;
    let environment = std::env::var_os("CONNECTORS_AEP");
    let selected = requested.or_else(|| environment.as_deref().map(Path::new));
    if let Some(path) = selected {
        return check(path, &pin)
            .map_err(|error| format!("explicit AEP {}: {error}", path.display()).into());
    }
    let mut errors = Vec::new();
    for path in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|path| path.join("aep"))
    {
        match check(&path, &pin) {
            Ok(binary) => return Ok(binary),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    Err(format!(
        "AEP {} is required; install that release (`b10x upgrade`). Searched: {}",
        pin.aep,
        errors.join("; ")
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_is_one_exact_release() {
        assert_eq!(pin().unwrap().aep.split('.').count(), 3);
        assert!(parse(br#"{"aep":"0.65"}"#).is_err());
        assert!(parse(br#"{"aep":"0.65.x"}"#).is_err());
        assert!(parse(br#"{"aep":"0.65.0","patch_sha256":"00"}"#).is_err());
    }
}
