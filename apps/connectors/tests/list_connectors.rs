//! story:cli-lists-available-connectors: `connectors list` prints every
//! connector this build ships a declaration for, one row each, with its auth
//! profiles and the instances the configuration names. It needs no
//! configuration file and starts no process, and no row names an
//! implementation (`contracts/cli/v1alpha1/semantics.md` §3, "Listing
//! connectors").
#![cfg(unix)]
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn private_file(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// An installation: a copy of the production CLI and, beside it, a stand-in
/// catalog provider that records every start in `started`. A derived
/// instance read as every other command reads it would start it.
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
        private_file(
            &bin.join("connectors-catalog-provider"),
            format!(
                "#!/bin/sh\nprintf 'started\\n' >> '{}'\nexit 1\n",
                root.path().join("started").display()
            )
            .as_bytes(),
            0o700,
        );
        Self { root }
    }

    fn config(&self) -> PathBuf {
        self.root.path().join("config/config.toml")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(self.root.path().join("bin/connectors"))
            .env_clear()
            .arg("--config")
            .arg(self.config())
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .output()
            .unwrap()
    }

    /// A `connectors-local/5` configuration holding `body`.
    fn configure(&self, body: &str) {
        let initialized = self.run(&["setup", "init"]);
        assert!(initialized.status.success(), "{initialized:?}");
        private_file(
            &self.config(),
            format!(
                "format='connectors-local/5'\nowner_uid={}\n{body}",
                // SAFETY: getuid has no preconditions and cannot fail.
                unsafe { libc::getuid() }
            )
            .as_bytes(),
            0o600,
        );
    }

    fn started(&self) -> bool {
        self.root.path().join("started").exists()
    }
}

/// The rows of a `--output json` success, by connector id: exactly one
/// envelope on stdout, nothing on standard error.
fn rows(output: &Output) -> BTreeMap<String, Value> {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let text = String::from_utf8(output.stdout.clone()).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let envelope: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(envelope["ok"], true, "{envelope}");
    envelope["result"]["connectors"]
        .as_array()
        .unwrap_or_else(|| panic!("{envelope}"))
        .iter()
        .map(|row| (row["id"].as_str().unwrap().to_owned(), row.clone()))
        .collect()
}

/// The human table's rows by their first column, header excluded.
fn table(output: &Output) -> BTreeMap<String, Vec<String>> {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let text = String::from_utf8(output.stdout.clone()).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(header, ["CONNECTOR", "PROFILES", "INSTANCES"], "{text}");
    lines
        .map(|line| {
            let columns: Vec<String> = line
                .split("  ")
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_owned)
                .collect();
            (columns[0].clone(), columns[1..].to_vec())
        })
        .collect()
}

fn says_catalog(output: &Output) -> bool {
    String::from_utf8_lossy(&output.stdout)
        .to_lowercase()
        .contains("catalog")
        || String::from_utf8_lossy(&output.stderr)
            .to_lowercase()
            .contains("catalog")
}

#[test]
fn with_no_configuration_every_shipped_connector_has_a_row_and_none_says_catalog() {
    let installation = Installation::new();
    let json = installation.run(&["--output", "json", "list"]);
    let listed = rows(&json);
    for connector in ["gitlab", "gmail", "zendesk"] {
        let row = &listed[connector];
        assert_eq!(row["instances"], serde_json::json!([]), "{row}");
        assert!(!row["profiles"].as_array().unwrap().is_empty(), "{row}");
    }
    assert_eq!(
        listed["gitlab"]["profiles"],
        serde_json::json!(["gitlab.pat"])
    );
    assert_eq!(
        listed["zendesk"]["profiles"],
        serde_json::json!(["zendesk.oauth", "zendesk.basic"])
    );
    assert!(!says_catalog(&json), "{json:?}");

    let human = installation.run(&["list"]);
    let shown = table(&human);
    for connector in ["gitlab", "gmail", "zendesk"] {
        assert_eq!(
            shown[connector].last().map(String::as_str),
            Some("-"),
            "{shown:?}"
        );
    }
    assert_eq!(shown.len(), listed.len(), "{shown:?}");
    assert!(!says_catalog(&human), "{human:?}");
    assert!(!installation.config().exists());
    assert!(!installation.started());
}

/// No resolvable configuration path at all is no configuration.
#[test]
fn without_a_configuration_path_the_list_still_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
        .env_clear()
        .args(["--output", "json", "list"])
        .output()
        .unwrap();
    let listed = rows(&output);
    assert!(listed.contains_key("gitlab"), "{listed:?}");
}

#[test]
fn a_configured_instance_is_named_on_its_connectors_row_and_nothing_is_started() {
    let installation = Installation::new();
    installation
        .configure("[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.work]\n");
    let json = installation.run(&["--output", "json", "list"]);
    let listed = rows(&json);
    assert_eq!(listed["gitlab"]["instances"], serde_json::json!(["work"]));
    assert_eq!(listed["gmail"]["instances"], serde_json::json!([]));
    assert!(!says_catalog(&json), "{json:?}");

    let human = installation.run(&["list"]);
    let shown = table(&human);
    assert_eq!(shown["gitlab"], ["gitlab.pat", "work"], "{shown:?}");
    assert!(!says_catalog(&human), "{human:?}");
    assert!(!installation.started(), "the catalog provider was started");
}

/// A file that exists is admitted as for every other command: one that is
/// not owner-only is refused naming its rule.
#[test]
fn a_configuration_that_is_not_owner_only_is_refused() {
    let installation = Installation::new();
    installation.configure("");
    fs::set_permissions(installation.config(), fs::Permissions::from_mode(0o644)).unwrap();
    let output = installation.run(&["--output", "json", "list"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        refused["error"]["data"]["configuration_rule"], "config_not_private",
        "{refused}"
    );
}
