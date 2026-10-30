//! story:connectors-do-short-invocation, decision D10: the security review of
//! `connections connect <connector>/<instance> --endpoint <url>`, which writes
//! the operator's owner-only configuration file before any credential is
//! asked for. Each case holds one invariant the write rests on:
//!
//! - the replacement never widens the file's mode and never writes through a
//!   link (`filesystem::publish_replace_keeping`);
//! - an endpoint carrying a credential (userinfo, a query or a fragment), or
//!   one that is not https, never reaches the configuration file, the state
//!   directory or the refusal;
//! - an endpoint written for one instance never lands in another;
//! - a changed endpoint is a changed binding: another provider authority, so a
//!   stored credential is never sent to the new host
//!   (`registry::revalidation::identity_changed`);
//! - an endpoint that moves an instance which already holds a connection to
//!   another provider authority is refused before the file is written, since
//!   another host is another instance and connect refuses that instance id
//!   afterwards (`registry::lifecycle::authentication_changed`).
//!
//! The production CLI is copied beside the real catalog provider built in the
//! same target directory (`cargo build -p connectors-catalog-provider`).
#![cfg(unix)]
use connectors_host::local::{filesystem, registry, runtime};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const GITLAB_DEFAULT: &str = "https://gitlab.com";
const OTHER_HOST: &str = "https://gitlab.example.test";
/// A marker standing for a credential an operator pastes into a URL.
const MARKER: &str = "s3cr3t-d218-token";

fn real_provider() -> PathBuf {
    let provider =
        Path::new(env!("CARGO_BIN_EXE_connectors")).with_file_name("connectors-catalog-provider");
    assert!(
        provider.exists(),
        "build it first: cargo build -p connectors-catalog-provider"
    );
    provider
}

fn install(source: &Path, target: &Path) {
    fs::copy(source, target).unwrap();
    fs::set_permissions(target, fs::Permissions::from_mode(0o700)).unwrap();
}

struct Installation {
    root: tempfile::TempDir,
}

impl Installation {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        install(
            Path::new(env!("CARGO_BIN_EXE_connectors")),
            &bin.join("connectors"),
        );
        install(&real_provider(), &bin.join("connectors-catalog-provider"));
        Self { root }
    }

    fn config(&self) -> PathBuf {
        self.root.path().join("config/config.toml")
    }

    fn state(&self) -> PathBuf {
        self.root.path().join("state")
    }

    fn run(&self, args: &[&str]) -> Output {
        let run = || {
            Command::new(self.root.path().join("bin/connectors"))
                .env_clear()
                .args(["--output", "json", "--config"])
                .arg(self.config())
                .arg("--state-dir")
                .arg(self.state())
                .args(args)
                .stdin(Stdio::null())
                .output()
        };
        for _ in 0..50 {
            match run() {
                Err(error) if error.raw_os_error() == Some(26) => {
                    std::thread::sleep(std::time::Duration::from_millis(100))
                }
                output => return output.unwrap(),
            }
        }
        run().unwrap()
    }

    fn configure(&self, body: &str) -> String {
        let initialized = self.run(&["setup", "init"]);
        assert!(initialized.status.success(), "{initialized:?}");
        let text = format!(
            "format='connectors-local/5'\nowner_uid={}\n{body}",
            filesystem::uid()
        );
        fs::write(self.config(), &text).unwrap();
        fs::set_permissions(self.config(), fs::Permissions::from_mode(0o600)).unwrap();
        text
    }

    fn text(&self) -> String {
        fs::read_to_string(self.config()).unwrap()
    }

    fn inode(&self) -> u64 {
        fs::symlink_metadata(self.config()).unwrap().ino()
    }

    /// Every name in the configuration's directory.
    fn beside(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.config().parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn connect(&self, target: &str, endpoint: &str) -> Output {
        self.run(&[
            "connections",
            "connect",
            target,
            "--profile",
            "gitlab.pat",
            "--endpoint",
            endpoint,
        ])
    }
}

/// Whether any file under `root` holds `needle`.
fn tree_holds(root: &Path, needle: &[u8]) -> bool {
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file()
                && fs::read(&path)
                    .map(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
                    .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

fn refusal(output: &Output) -> Value {
    assert!(!output.status.success(), "{output:?}");
    let value: Value =
        serde_json::from_slice(&output.stderr).unwrap_or_else(|_| panic!("{output:?}"));
    value["error"]["data"].clone()
}

/// The bootstrap the real provider prints for `gitlab/ref-1`, at `endpoint`
/// or at the declaration's default endpoint.
fn bootstrap(endpoint: Option<&str>) -> runtime::Bootstrap {
    let mut command = Command::new(real_provider());
    command.args(["--connector", "gitlab", "--instance", "ref-1"]);
    if let Some(endpoint) = endpoint {
        command.args(["--endpoint", endpoint]);
    }
    let output = command
        .arg("--print-local-bootstrap")
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

/// story:connectors-do-short-invocation: the replacement the endpoint write
/// uses keeps an owner-only mode exactly, refuses any mode with a group,
/// other or special bit, leaves no temporary file, and replaces a link's name
/// without writing what the link points at.
#[test]
fn the_replacement_never_widens_the_mode_or_writes_through_a_link() {
    let root = tempfile::tempdir().unwrap();
    let dir_path = root.path().join("config");
    fs::create_dir(&dir_path).unwrap();
    fs::set_permissions(&dir_path, fs::Permissions::from_mode(0o700)).unwrap();
    let target = dir_path.join("config.toml");
    let names = || {
        let mut names: Vec<String> = fs::read_dir(&dir_path)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let name = std::ffi::OsStr::new("config.toml");
    for mode in [0o600, 0o400, 0o700] {
        let _ = fs::remove_file(&target);
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        let directory = filesystem::directory(&dir_path, false, false).unwrap();
        filesystem::publish_replace_keeping(&directory, name, b"new", mode).unwrap();
        let meta = fs::symlink_metadata(&target).unwrap();
        assert_eq!(meta.mode() & 0o7777, mode, "{mode:o}");
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(names(), ["config.toml"], "{mode:o}");
    }
    for mode in [0o640, 0o604, 0o660, 0o644, 0o4600, 0o2600, 0o1600, 0o10600] {
        let _ = fs::remove_file(&target);
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        let directory = filesystem::directory(&dir_path, false, false).unwrap();
        assert!(
            filesystem::publish_replace_keeping(&directory, name, b"new", mode).is_err(),
            "{mode:o}"
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "old", "{mode:o}");
        assert_eq!(names(), ["config.toml"], "{mode:o}");
    }
    // A link at the name: what it points at is never written.
    let outside = root.path().join("outside.toml");
    fs::write(&outside, "outside").unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(&target).unwrap();
    symlink(&outside, &target).unwrap();
    let directory = filesystem::directory(&dir_path, false, false).unwrap();
    let _ = filesystem::publish_replace_keeping(&directory, name, b"new", 0o600);
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside");
    assert_eq!(
        fs::symlink_metadata(&outside).unwrap().mode() & 0o7777,
        0o600
    );
}

/// story:connectors-do-short-invocation: an endpoint that carries a
/// credential (userinfo, a query, a fragment) or is not https is refused, and
/// the credential reaches neither the configuration file, nor any file of the
/// state directory, nor the refusal. The file keeps its bytes and its inode,
/// and no temporary file is left beside it.
#[test]
fn an_endpoint_carrying_a_credential_is_never_written_or_echoed() {
    let installation = Installation::new();
    let before = installation.configure("");
    let inode = installation.inode();
    let beside = installation.beside();
    for endpoint in [
        format!("https://user:{MARKER}@gitlab.example.test"),
        format!("https://{MARKER}@gitlab.example.test"),
        format!("https://:{MARKER}@gitlab.example.test"),
        format!("https://gitlab.example.test/?private_token={MARKER}"),
        format!("https://gitlab.example.test/#{MARKER}"),
        format!("http://gitlab.example.test/{MARKER}"),
        format!("http://user:{MARKER}@gitlab.example.test"),
    ] {
        let output = installation.connect("gitlab/ref-1", &endpoint);
        assert!(!output.status.success(), "{endpoint}: {output:?}");
        let printed = [output.stdout.as_slice(), output.stderr.as_slice()].concat();
        assert!(
            !printed
                .windows(MARKER.len())
                .any(|w| w == MARKER.as_bytes()),
            "{endpoint}: the refusal echoes the credential: {output:?}"
        );
        assert_eq!(installation.text(), before, "{endpoint}");
        assert_eq!(installation.inode(), inode, "{endpoint}");
        assert_eq!(installation.beside(), beside, "{endpoint}");
        assert!(
            !tree_holds(&installation.state(), MARKER.as_bytes()),
            "{endpoint}: the state directory holds the credential"
        );
    }
}

/// story:connectors-do-short-invocation: the endpoint lands in the named
/// instance's table only. Another instance of the same connector keeps its
/// endpoint, and a target whose names TOML would read as a path into another
/// table is refused without writing.
#[test]
fn an_endpoint_for_one_instance_never_lands_in_another() {
    let installation = Installation::new();
    installation.configure(&format!(
        "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.ref-1]\nendpoint='{OTHER_HOST}'\n[connectors.gitlab.instances.ref-2]\ndescription='second'\n"
    ));
    let elsewhere = "https://elsewhere.example.test";
    let data = refusal(&installation.connect("gitlab/ref-2", elsewhere));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    let written: toml::Table = installation.text().parse().unwrap();
    let instances = &written["connectors"]["gitlab"]["instances"];
    assert_eq!(instances["ref-1"]["endpoint"].as_str(), Some(OTHER_HOST));
    assert_eq!(instances["ref-2"]["endpoint"].as_str(), Some(elsewhere));
    let before = installation.text();
    for target in [
        "gitlab/ref-1.endpoint",
        "gitlab.instances.ref-1/x",
        "gitlab/ref-1\"",
        "gitlab/ref 1",
        "gitlab/ref-1]",
    ] {
        let output = installation.connect(target, "https://third.example.test");
        assert!(!output.status.success(), "{target}: {output:?}");
        assert_eq!(installation.text(), before, "{target}");
    }
}

/// story:connectors-do-short-invocation: a changed endpoint is a changed
/// binding. The real provider reports another provider authority, another
/// configuration revision and another credential-side revision for
/// `gitlab/ref-1` at another host than its default, so a connection admitted
/// under the default is `AuthenticationChanged` under the new configuration
/// and its stored credential is never sent there
/// (`registry::revalidation::identity_changed`).
#[test]
fn a_changed_endpoint_is_another_provider_authority() {
    let default = bootstrap(None);
    let explicit_default = bootstrap(Some(GITLAB_DEFAULT));
    let other = bootstrap(Some(OTHER_HOST));
    assert_eq!(
        default.provider_authority,
        explicit_default.provider_authority
    );
    assert_ne!(default.provider_authority, other.provider_authority);
    assert_ne!(default.configuration_revision, other.configuration_revision);
    assert_ne!(default.credential_revision, other.credential_revision);
    assert!(other.credential_revision.is_some());
    let (before, after) = (
        default.binding("gitlab.pat").unwrap(),
        other.binding("gitlab.pat").unwrap(),
    );
    assert_eq!(before.instance_id, after.instance_id);
    assert_ne!(before.provider_authority, after.provider_authority);
}

/// story:connectors-do-short-invocation: "another host is another instance".
/// `gitlab/ref-1` is listed without an endpoint, so it runs at the
/// declaration's default host, and it already holds a connection begun there.
/// `--endpoint` at another host moves that instance id to another provider
/// authority: its connection is stranded (`AuthenticationChanged`) and every
/// later connect of the instance id is refused
/// (`registry::lifecycle::authentication_changed`). The command is refused
/// before anything is written, as a different endpoint already in the table
/// is: `lifecycle_conflict` naming `endpoint`, the file unchanged.
#[test]
fn an_endpoint_that_moves_a_connected_instance_to_another_host_is_refused() {
    let installation = Installation::new();
    let before = installation.configure(
        "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.ref-1]\ndescription='team forge'\n",
    );
    // The connection an earlier `connections connect gitlab/ref-1` began under
    // the default host.
    let binding = bootstrap(None).binding("gitlab.pat").unwrap();
    registry::Registry::with_system_clock(&installation.state())
        .begin(&binding, connectors_sdk::now_ms())
        .unwrap();
    let inode = installation.inode();
    let output = installation.connect("gitlab/ref-1", OTHER_HOST);
    let data = refusal(&output);
    let after = installation.text();
    assert_eq!(
        (
            &data["code"],
            &data["field"],
            after == before,
            installation.inode() == inode
        ),
        (
            &Value::from("lifecycle_conflict"),
            &Value::from("endpoint"),
            true,
            true
        ),
        "the instance's host was rewritten under its connection: {data}\n--- before\n{before}\n--- after\n{after}"
    );
}

/// story:connectors-do-short-invocation: what the write above leaves. Once
/// `gitlab/ref-1` holds a connection begun at its default host, a connection
/// under the binding the new endpoint derives is refused
/// `AuthenticationChanged`: the instance admits no connection at the new host,
/// so the endpoint written before the credential is asked for strands the
/// instance until the operator edits the file back.
#[test]
fn a_connected_instance_moved_to_another_host_admits_no_new_connection() {
    let installation = Installation::new();
    installation.configure("");
    let registry = registry::Registry::with_system_clock(&installation.state());
    let default = bootstrap(None).binding("gitlab.pat").unwrap();
    registry.begin(&default, connectors_sdk::now_ms()).unwrap();
    let moved = bootstrap(Some(OTHER_HOST)).binding("gitlab.pat").unwrap();
    assert_eq!(
        registry.begin(&moved, connectors_sdk::now_ms()).err(),
        Some(registry::Failure::AuthenticationChanged)
    );
}
