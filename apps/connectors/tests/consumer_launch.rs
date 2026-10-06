//! story:launch-consumer-with-connection-credential, ordinary lane. A launch
//! the configuration refuses is refused by name through the production CLI,
//! before any owner, keyring or consumer starts. The delivery itself, lapsed
//! evidence and a locked collection need the disposable Secret Service and are
//! the ignored host journey `disposable_consumer_launch_delivers_fd3_and_refuses_by_code`.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

struct Cli {
    config: PathBuf,
    state: PathBuf,
}

impl Cli {
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .env_clear()
            .args(["--output", "json", "--config"])
            .arg(&self.config)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
}

fn sha256(path: &Path) -> String {
    ring::digest::digest(&ring::digest::SHA256, &fs::read(path).unwrap())
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn uid() -> String {
    let status = fs::read_to_string("/proc/self/status").unwrap();
    status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|ids| ids.split_whitespace().next())
        .unwrap()
        .to_owned()
}

/// A `connectors-local/3` configuration with consumers `probe` (listing
/// `fixture`), `stranger` (listing nothing) and `tampered` (wrong digest).
fn configured(root: &Path) -> Cli {
    let cli = Cli {
        config: root.join("config/config.toml"),
        state: root.join("state"),
    };
    assert!(cli.run(&["setup", "init"]).status.success());
    let image = Path::new("/usr/bin/true");
    let consumer =
        |sha256: &str| format!("path='{}'\nsha256='{sha256}'\nargs=[]\n", image.display());
    fs::write(
        &cli.config,
        format!(
            "format='connectors-local/3'\nowner_uid={}\n[adapters.fixture]\ninstance_id='fixture-instance'\nadapter_id='fixture-adapter'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.fixture.permissions]\nprofiles=['fixture-token']\n[adapters.fixture.executable]\npath='/not-installed/must-not-start'\nsha256='{}'\nargs=[]\n[consumers.probe.executable]\n{}[consumers.probe.permissions]\nconnections=['fixture']\n[consumers.stranger.executable]\n{}[consumers.tampered.executable]\n{}[consumers.tampered.permissions]\nconnections=['fixture']\n",
            uid(),
            "a".repeat(64),
            consumer(&sha256(image)),
            consumer(&sha256(image)),
            consumer(&"b".repeat(64)),
        ),
    )
    .unwrap();
    cli
}

fn refused(output: &Output, exit: i32, code: &str, stage: &str, case: &str) {
    assert_eq!(output.status.code(), Some(exit), "{case}: {output:?}");
    assert!(output.stdout.is_empty(), "{case}: {output:?}");
    let refusal: Value = serde_json::from_slice(&output.stderr).unwrap();
    let data = &refusal["error"]["data"];
    assert_eq!(
        (data["code"].as_str(), data["stage"].as_str()),
        (Some(code), Some(stage)),
        "{case}: {refusal}"
    );
}

#[test]
fn a_launch_the_configuration_refuses_is_refused_by_name_before_anything_starts() {
    let root = tempfile::tempdir().unwrap();
    let cli = configured(root.path());
    for (consumer, code, stage, exit) in [
        ("absent", "not_found", "admission", 1),
        ("stranger", "forbidden", "admission", 1),
        ("tampered", "invalid_configuration", "configuration", 2),
        // An admitted pair reaches the connection authority: no such connection.
        ("probe", "not_found", "admission", 1),
    ] {
        let output = cli.run(&[
            "connections",
            "launch",
            "--adapter",
            "fixture",
            "--connection",
            "connection-1",
            "--consumer",
            consumer,
        ]);
        refused(&output, exit, code, stage, consumer);
    }
    assert!(!cli.state.join("owner.sock").exists());
}

#[test]
fn a_malformed_args_value_is_refused_as_invalid_input_before_admission() {
    let root = tempfile::tempdir().unwrap();
    let cli = configured(root.path());
    let launch = |consumer: &str, args: &str| {
        cli.run(&[
            "connections",
            "launch",
            "--adapter",
            "fixture",
            "--connection",
            "connection-1",
            "--consumer",
            consumer,
            "--args",
            args,
        ])
    };
    // Each is refused as usage, even for a consumer admission would refuse.
    for malformed in [
        "not json",
        "\"head\"",
        "{\"head\":1}",
        "[\"head\",1]",
        "[[\"head\"]]",
        "[\"he\\u0000ad\"]",
    ] {
        refused(
            &launch("stranger", malformed),
            2,
            "invalid_input",
            "arguments",
            malformed,
        );
    }
    // A JSON array of strings, including an empty one, reaches admission.
    for accepted in ["[]", "[\"head\",\"--json\",\"\"]"] {
        refused(
            &launch("stranger", accepted),
            1,
            "forbidden",
            "admission",
            accepted,
        );
    }
    assert!(!cli.state.join("owner.sock").exists());
}
