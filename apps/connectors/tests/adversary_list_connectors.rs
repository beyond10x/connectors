//! Adversary cases for story:cli-lists-available-connectors. The contract
//! (`contracts/cli/v1alpha1/semantics.md` §3, "Listing connectors") says a
//! file that exists "is refused as for every other command", and the doc
//! comment of `Config::listed_instances` says it is "admitted as
//! [`Config::read`] admits it". These cases hold `list` to that: a file every
//! other command refuses is refused by `list` too, and a configuration that
//! resolves is never silently dropped.
#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn private_file(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A copy of the CLI and, beside it, a stand-in catalog provider that records
/// every start.
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

    fn command(&self) -> Command {
        let mut command = Command::new(self.root.path().join("bin/connectors"));
        command.env_clear();
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .arg("--config")
            .arg(self.config())
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .output()
            .unwrap()
    }

    /// `setup init`, then the configuration replaced by `format` and `body`.
    fn configure(&self, format: &str, body: &str) {
        let initialized = self.run(&["setup", "init"]);
        assert!(initialized.status.success(), "{initialized:?}");
        private_file(
            &self.config(),
            format!(
                "format='{format}'\nowner_uid={}\n{body}",
                // SAFETY: getuid has no preconditions and cannot fail.
                unsafe { libc::getuid() }
            )
            .as_bytes(),
            0o600,
        );
    }
}

/// A configuration every other command refuses is refused by `list` as well,
/// for each way `Config::read` refuses that `listed_instances` does not check:
/// an unsupported format, a `[connectors]` table outside `connectors-local/5`,
/// a disabled connector with instances, and an instance name that is no
/// selector.
#[test]
fn list_refuses_every_configuration_instances_list_refuses() {
    let cases = [
        (
            "unsupported format",
            "connectors-local/99",
            "[connectors.gitlab]\nenabled=true\n",
        ),
        (
            "[connectors] outside connectors-local/5",
            "connectors-local/4",
            "[connectors.gitlab]\nenabled=true\n",
        ),
        (
            "disabled connector with an instance",
            "connectors-local/5",
            "[connectors.gitlab]\nenabled=false\n[connectors.gitlab.instances.work]\n",
        ),
        (
            "instance name that is no selector",
            "connectors-local/5",
            "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.\"Work Space\"]\n",
        ),
    ];
    let mut accepted = Vec::new();
    for (case, format, body) in cases {
        let installation = Installation::new();
        installation.configure(format, body);
        let every_other = installation.run(&["--output", "json", "instances", "list"]);
        assert_ne!(
            every_other.status.code(),
            Some(0),
            "{case}: instances list accepted it: {every_other:?}"
        );
        let listed = installation.run(&["--output", "json", "list"]);
        if listed.status.code() == Some(0) {
            accepted.push(format!(
                "{case}: instances list exit {:?}, list exit 0 with {}",
                every_other.status.code(),
                String::from_utf8_lossy(&listed.stdout).trim_end()
            ));
        }
    }
    assert!(accepted.is_empty(), "\n{}", accepted.join("\n"));
}

/// An instance name carrying a terminal control sequence, which every other
/// command refuses (it is no selector), is not written to the terminal.
#[test]
fn list_never_writes_a_control_character_from_the_configuration() {
    let installation = Installation::new();
    installation.configure(
        "connectors-local/5",
        "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.\"w\\u001b[2Jork\"]\n",
    );
    let human = installation.run(&["list"]);
    assert!(
        !human.stdout.contains(&0x1b),
        "exit {:?}, stdout {:?}",
        human.status.code(),
        String::from_utf8_lossy(&human.stdout)
    );
}

/// A configuration whose path resolves is read even when no state directory
/// resolves: `list` needs no state, and a configured instance is not dropped
/// from its row while the command succeeds.
#[test]
fn an_unresolvable_state_directory_does_not_hide_the_configured_instances() {
    let installation = Installation::new();
    installation.configure(
        "connectors-local/5",
        "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.work]\n",
    );
    let output = installation
        .command()
        .arg("--config")
        .arg(installation.config())
        .args(["--state-dir", "relative/state", "--output", "json", "list"])
        .output()
        .unwrap();
    if output.status.code() == Some(0) {
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        let gitlab = envelope["result"]["connectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == "gitlab")
            .cloned()
            .unwrap();
        assert_eq!(
            gitlab["instances"],
            serde_json::json!(["work"]),
            "the configured instance was dropped: {envelope}"
        );
    }
}

// Round 2: the no-derive mode (`Derivation::Deferred`). A deferred instance
// has no entry in `adapters`, so every rule that looks an alias up there sees
// it as absent, and every rule that counts entries counts it although a real
// derivation that awaits a profile adds none.

/// The real catalog provider built beside the test's CLI
/// (`cargo build -p connectors-catalog-provider`).
fn real_provider() -> PathBuf {
    let provider =
        Path::new(env!("CARGO_BIN_EXE_connectors")).with_file_name("connectors-catalog-provider");
    assert!(
        provider.exists(),
        "build it first: cargo build -p connectors-catalog-provider"
    );
    provider
}

/// An installation whose catalog provider is the real one, so every other
/// command derives unpinned instances and accepts what derives.
fn with_real_provider() -> Installation {
    let installation = Installation::new();
    let beside = installation
        .root
        .path()
        .join("bin/connectors-catalog-provider");
    fs::copy(real_provider(), &beside).unwrap();
    fs::set_permissions(&beside, fs::Permissions::from_mode(0o700)).unwrap();
    installation
}

const GMAIL: &str = "[connectors.gmail]\nenabled=true\n[connectors.gmail.instances.private]\n";

/// A legacy entry under `alias` that loads without starting anything.
fn legacy(alias: &str) -> String {
    format!(
        "[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='forge'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/forge'\nsha256='{}'\nargs=[]\n",
        "0".repeat(64)
    )
}

/// Both commands' exits on the same file: `instances list` reads it as every
/// command does, `list` in the no-derive mode.
fn both(installation: &Installation) -> (Output, Output) {
    (
        installation.run(&["--output", "json", "instances", "list"]),
        installation.run(&["--output", "json", "list"]),
    )
}

/// A file every other command accepts is accepted by `list`, for each rule
/// that looks a derived instance's alias up in `adapters`: a consumer
/// permitted the derived instance's connections, and `default_adapter`
/// naming its alias.
#[test]
fn list_accepts_every_configuration_instances_list_accepts() {
    let consumer = format!(
        "[consumers.agent.executable]\npath='/not-installed/agent'\nsha256='{}'\nargs=[]\n[consumers.agent.permissions]\nconnections=['gmail.private']\n",
        "0".repeat(64)
    );
    let cases = [
        (
            "consumer permitted the derived alias",
            format!("{GMAIL}{consumer}"),
        ),
        (
            "default_adapter naming the derived alias",
            format!("default_adapter='gmail.private'\n{GMAIL}"),
        ),
    ];
    let mut refused = Vec::new();
    for (case, body) in cases {
        let installation = with_real_provider();
        installation.configure("connectors-local/5", &body);
        let (every_other, listed) = both(&installation);
        assert_eq!(
            every_other.status.code(),
            Some(0),
            "{case}: precondition, instances list refused it: {every_other:?}"
        );
        if listed.status.code() != Some(0) {
            refused.push(format!(
                "{case}: instances list exit 0, list exit {:?}: {}",
                listed.status.code(),
                String::from_utf8_lossy(&listed.stderr).trim_end()
            ));
        }
    }
    assert!(refused.is_empty(), "\n{}", refused.join("\n"));
}

/// The 64-entry limit counts entries. An instance of a connector with several
/// profiles that names none awaits a profile and has no entry, so 64 legacy
/// entries beside it are accepted by every other command; `list` counts the
/// deferred instance as a 65th entry.
#[test]
fn an_instance_awaiting_a_profile_is_not_counted_against_the_entry_limit() {
    let installation = with_real_provider();
    let entries: String = (0..64).map(|n| legacy(&format!("forge{n:02}"))).collect();
    installation.configure(
        "connectors-local/5",
        &format!(
            "{entries}[connectors.zendesk]\nenabled=true\n[connectors.zendesk.instances.support]\n"
        ),
    );
    let (every_other, listed) = both(&installation);
    assert_eq!(
        every_other.status.code(),
        Some(0),
        "precondition, instances list refused it: {every_other:?}"
    );
    assert_eq!(
        listed.status.code(),
        Some(0),
        "list refused a file instances list accepts: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
}
