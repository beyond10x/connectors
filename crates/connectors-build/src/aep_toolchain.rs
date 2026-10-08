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
    Err(format!("{} Searched: {}", install_hint(&pin.aep), errors.join("; ")).into())
}

/// The exact steps that install the pinned release; `b10x upgrade` installs the newest one,
/// which is not the pin when a newer AEP has shipped.
fn install_hint(version: &str) -> String {
    let archive = format!("aep-{version}-x86_64-unknown-linux-gnu.tar.gz");
    let base = format!("https://github.com/beyond10x/aep/releases/download/{version}");
    format!(
        "AEP {version} is required. Install that release: \
         curl -fsSLO {base}/{archive} && curl -fsSLO {base}/SHA256SUMS && \
         grep ' {archive}$' SHA256SUMS | sha256sum -c - && tar -xzf {archive}, \
         then set CONNECTORS_AEP=$PWD/aep-{version}-x86_64-unknown-linux-gnu/aep \
         or put that directory first on PATH."
    )
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

    #[test]
    fn the_refusal_names_the_pinned_release_and_how_to_install_it() {
        let hint = install_hint("0.69.0");
        assert!(hint.starts_with("AEP 0.69.0 is required."));
        assert!(hint.contains(
            "https://github.com/beyond10x/aep/releases/download/0.69.0/aep-0.69.0-x86_64-unknown-linux-gnu.tar.gz"
        ));
        assert!(hint.contains("sha256sum -c -"));
        assert!(hint.contains("CONNECTORS_AEP="));
        assert!(!hint.contains("b10x upgrade"));
    }
}
