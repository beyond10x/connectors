//! story:connector-instance-derived-fields: a configuration whose only Gmail
//! lines are `[connectors.gmail]` and an empty
//! `[connectors.gmail.instances.private]` is derived by the production CLI from
//! the catalog provider installed beside it. A copy of the CLI is placed next
//! to a stand-in provider that records its arguments and prints a bootstrap.
//! The stand-in, a script, cannot be started from a sealed snapshot, so the
//! derivation stops at the child handshake and the file is refused; what is
//! observed is which provider the CLI asked and with which arguments. The
//! whole path, with the real provider, is the catalog journey
//! `local_runtime::derived_instance`.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

const GMAIL: &str = "[connectors.gmail]\nenabled=true\n[connectors.gmail.instances.private]\n";

fn private_file(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// An installation: a copy of the production CLI and, beside it, a stand-in
/// catalog provider.
struct Installation {
    root: tempfile::TempDir,
}

impl Installation {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        let cli = bin.join("connectors");
        fs::copy(env!("CARGO_BIN_EXE_connectors"), &cli).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        let printed = root.path().join("bootstrap.json");
        private_file(
            &printed,
            json!({"instance": "gmail.private", "adapter": "catalog"})
                .to_string()
                .as_bytes(),
            0o600,
        );
        private_file(
            &bin.join("connectors-catalog-provider"),
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\ncat '{}'\n",
                root.path().join("args").display(),
                printed.display()
            )
            .as_bytes(),
            0o700,
        );
        Self { root }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(self.root.path().join("bin/connectors"))
            .env_clear()
            .args(["--output", "json", "--config"])
            .arg(self.root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .output()
            .unwrap()
    }

    fn configure(&self, body: &str) {
        let initialized = self.run(&["setup", "init"]);
        assert!(initialized.status.success(), "{initialized:?}");
        private_file(
            &self.root.path().join("config/config.toml"),
            format!(
                "format='connectors-local/5'\nowner_uid={}\n{body}",
                // SAFETY: getuid has no preconditions and cannot fail.
                unsafe { libc::getuid() }
            )
            .as_bytes(),
            0o600,
        );
    }

    /// The arguments of every start of the provider, one per line each.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.root.path().join("args"))
            .unwrap_or_default()
            .split_inclusive("--print-local-bootstrap\n")
            .map(str::to_owned)
            .collect()
    }
}

#[track_caller]
fn invalid_configuration(output: &Output) {
    assert!(!output.status.success(), "{output:?}");
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        refused["error"]["data"]["code"], "invalid_configuration",
        "{refused}"
    );
}

#[test]
fn the_cli_derives_an_instance_named_alone_from_the_provider_beside_it() {
    let installation = Installation::new();
    installation.configure(GMAIL);
    invalid_configuration(&installation.run(&["adapters", "list"]));
    let calls = installation.calls();
    assert!(!calls.is_empty());
    for call in calls {
        assert_eq!(
            call,
            "--connector\ngmail\n--instance\nprivate\n--print-local-bootstrap\n"
        );
    }
    // An endpoint is the provider's to digest.
    let installation = Installation::new();
    installation.configure(&GMAIL.replace(
        "instances.private]\n",
        "instances.private]\nendpoint='https://mail.example.test'\n",
    ));
    invalid_configuration(&installation.run(&["adapters", "list"]));
    let calls = installation.calls();
    assert!(!calls.is_empty());
    for call in calls {
        assert_eq!(
            call,
            "--connector\ngmail\n--instance\nprivate\n--endpoint\nhttps://mail.example.test\n--print-local-bootstrap\n"
        );
    }
}

#[test]
fn an_instance_named_alone_without_an_installed_provider_is_an_invalid_configuration() {
    let installation = Installation::new();
    fs::remove_file(
        installation
            .root
            .path()
            .join("bin/connectors-catalog-provider"),
    )
    .unwrap();
    installation.configure(GMAIL);
    invalid_configuration(&installation.run(&["adapters", "list"]));
}
