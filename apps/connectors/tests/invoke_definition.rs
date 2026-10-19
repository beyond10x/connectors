//! story:invoke-definition-file: `operations invoke --definition <file>` is a
//! third selector, exclusive with `--adapter` and with `--connector` and
//! `--instance`; a definition whose instance the configuration already holds
//! is refused by name; a definition the lowerer refuses is refused before any
//! owner, provider or credential is touched; and the operator's configuration
//! file is never written (`contracts/cli/v1alpha1/semantics.md` §3, "Invoking a
//! definition file"). These cases start no provider: the journey through a
//! disposable HTTPS provider is `adapters/catalog/tests/local_runtime/invoke_definition.rs`.
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// A directory holding no executable (`cli_surface_minor.rs`).
const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};

/// A hand-written token-profile definition of the connector `forge`. Its
/// source is named but never read by these cases unless one says so.
const DEFINITION: &str = "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/forge/openapi.json.gz
  sha256: 01fcc470fbbd1a2c691d5a17c35a00b086ca5e62fdd08747f92cb350e0fc6fa1
base_url: https://{hostname}/api/v4
config: {hostname: {type: string, default: gitlab.example.com}}
auth:
  forge.pat:
    scheme: header
    header: PRIVATE-TOKEN
    label: Fixture personal access token
    identity: {path: user, kind: forge.user, subject: /id}
    scopes: {path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}
operations:
  project.get:
    op: getApiV4ProjectsId
    description: Read one project
";

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// An initialised configuration and state, with `extra` appended to the
    /// configuration, and the definition `connectors/forge.yaml` under a
    /// definitions root whose source file is `source`.
    fn new(extra: &str, source: &[u8]) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        let init = fixture.run(&["setup", "init"]);
        assert!(init.status.success(), "{init:?}");
        if !extra.is_empty() {
            let initialized = fs::read_to_string(fixture.config()).unwrap();
            let owner = initialized
                .lines()
                .find(|line| line.starts_with("owner_uid"))
                .unwrap();
            private(
                &fixture.config(),
                &format!("format = \"connectors-local/5\"\n{owner}\n{extra}"),
            );
        }
        let defs = fixture.root.path().join("defs");
        fs::create_dir_all(defs.join("connectors")).unwrap();
        fs::create_dir_all(defs.join("upstream/forge")).unwrap();
        fs::write(defs.join("connectors/forge.yaml"), DEFINITION).unwrap();
        fs::write(defs.join("upstream/forge/openapi.json.gz"), source).unwrap();
        fixture
    }

    fn config(&self) -> PathBuf {
        self.root.path().join("config/config.toml")
    }

    fn state(&self) -> PathBuf {
        self.root.path().join("state")
    }

    fn definition(&self) -> String {
        self.root
            .path()
            .join("defs/connectors/forge.yaml")
            .to_str()
            .unwrap()
            .to_owned()
    }

    /// The CLI with no terminal: its own session, standard input closed.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .args(["--output", "json", "--config"])
            .arg(self.config())
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null());
        detach(&mut command);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    /// [`Fixture::run`], or `None` when the command has not exited within
    /// 10 s; it is then killed.
    #[cfg(unix)]
    fn run_within_10s(&self, args: &[&str]) -> Option<Output> {
        let mut child = self
            .command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= until {
                child.kill().unwrap();
                child.wait().unwrap();
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Some(child.wait_with_output().unwrap())
    }

    fn args<'a>(definition: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec![
            "operations",
            "invoke",
            "--definition",
            definition,
            "--operation",
            "project.get",
            "--input-json",
            r#"{"id":"org/project"}"#,
        ];
        args.extend_from_slice(extra);
        args
    }

    fn invoke(&self, extra: &[&str]) -> Output {
        let definition = self.definition();
        self.run(&Self::args(&definition, extra))
    }
}

/// A named pipe at `path`, with no writer.
#[cfg(unix)]
fn named_pipe(path: &Path) {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: a NUL-terminated path; a scalar mode.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
}

/// No controlling terminal: a command that would ask must refuse instead.
#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: setsid in the child between fork and exec; no allocation.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}
#[cfg(not(unix))]
fn detach(_: &mut Command) {}

fn private(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[track_caller]
fn refused(output: &Output, exit: i32) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"));
    assert_eq!(envelope["ok"], false, "{envelope}");
    envelope["error"]["data"].clone()
}

#[test]
fn definition_is_a_third_selector_exclusive_with_the_other_two() {
    let fixture = Fixture::new("", b"not read");
    let before = fs::read(fixture.config()).unwrap();
    for extra in [
        &["--adapter", "forge"][..],
        &["--connector", "forge", "--instance", "definition"],
        &["--connector", "forge"],
        &["--instance", "definition"],
    ] {
        let data = refused(&fixture.invoke(extra), 2);
        assert_eq!(data["code"], "invalid_input", "{extra:?} {data}");
        assert_eq!(data["stage"], "arguments", "{extra:?} {data}");
        assert_eq!(data["next_action"], "none", "{extra:?} {data}");
    }
    assert_eq!(fs::read(fixture.config()).unwrap(), before);
    assert!(!fixture.state().join("definitions").exists());
}

#[test]
fn definitions_root_and_credential_file_are_refused_without_a_definition() {
    let fixture = Fixture::new("", b"not read");
    for extra in [
        &["--definitions-root", "/fixture"][..],
        &["--credential-file", "/fixture/client.json"],
    ] {
        let mut args = vec![
            "operations",
            "invoke",
            "--adapter",
            "forge",
            "--connection",
            "c1",
            "--operation",
            "project.get",
            "--schema",
            "s1",
            "--revision",
            "r1",
            "--input-json",
            "{}",
        ];
        args.extend_from_slice(extra);
        let data = refused(&fixture.run(&args), 2);
        assert_eq!(data["code"], "invalid_input", "{extra:?} {data}");
        assert_eq!(data["stage"], "arguments", "{extra:?} {data}");
    }
}

#[test]
fn without_a_definition_schema_and_revision_stay_required() {
    let fixture = Fixture::new("", b"not read");
    let output = fixture.run(&[
        "operations",
        "invoke",
        "--adapter",
        "forge",
        "--connection",
        "c1",
        "--operation",
        "project.get",
        "--input-json",
        "{}",
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["error"]["code"], "cli_parse", "{envelope}");
}

#[test]
fn a_definition_whose_instance_is_configured_is_refused_by_name() {
    let table = "connectors.forge.instances.definition";
    let configured = format!(
        "[connectors.forge]\nenabled=true\n[{table}]\nalias='forge-pinned'\ninstance_id='forge-pinned'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['forge.pat']\noperations=['project.get']\n",
        "a".repeat(64)
    );
    // The instance id `forge.definition` is the definition's too.
    let by_id = format!(
        "[adapters.other]\ninstance_id='forge.definition'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.other.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
        "a".repeat(64)
    );
    for extra in [configured, by_id] {
        let fixture = Fixture::new(&extra, b"not read");
        let before = fs::read(fixture.config()).unwrap();
        let data = refused(&fixture.invoke(&[]), 2);
        assert_eq!(data["code"], "invalid_configuration", "{data}");
        assert_eq!(data["stage"], "configuration", "{data}");
        assert_eq!(
            data["configuration_rule"], "definition_instance_configured",
            "{data}"
        );
        assert_eq!(data["next_action"], "check_configuration", "{data}");
        assert_eq!(fs::read(fixture.config()).unwrap(), before);
        assert!(!fixture.state().join("definitions").exists());
    }
}

#[test]
fn a_definition_the_lowerer_refuses_is_invalid_input_and_nothing_is_kept() {
    // Not gzip: the lowerer refuses it before it writes anything.
    let fixture = Fixture::new("", b"not gzip");
    let before = fs::read(fixture.config()).unwrap();
    let data = refused(&fixture.invoke(&[]), 2);
    assert_eq!(data["code"], "invalid_input", "{data}");
    assert_eq!(data["stage"], "arguments", "{data}");
    assert_eq!(fs::read(fixture.config()).unwrap(), before);
    // No cache entry and no instance entry for a definition that did not lower.
    let definitions = fixture.state().join("definitions");
    let kept: Vec<_> = fs::read_dir(&definitions)
        .map(|entries| entries.map(|entry| entry.unwrap().file_name()).collect())
        .unwrap_or_default();
    assert!(kept.is_empty(), "{kept:?}");
}

#[test]
fn a_definition_that_cannot_be_read_is_invalid_input() {
    let fixture = Fixture::new("", b"not read");
    let missing = fixture.root.path().join("defs/connectors/absent.yaml");
    let data = refused(
        &fixture.run(&[
            "operations",
            "invoke",
            "--definition",
            missing.to_str().unwrap(),
            "--operation",
            "project.get",
            "--input-json",
            "{}",
        ]),
        2,
    );
    assert_eq!(data["code"], "invalid_input", "{data}");
    assert_eq!(data["stage"], "arguments", "{data}");
}

/// A definition path that is a named pipe is one that cannot be read, refused
/// without waiting for a writer: an `open(2)` of a pipe with no writer blocks,
/// and an interrupt does not end that wait.
#[cfg(unix)]
#[test]
fn a_definition_that_is_a_named_pipe_is_refused_without_waiting_for_a_writer() {
    let fixture = Fixture::new("", b"not read");
    let pipe = fixture.root.path().join("defs/connectors/pipe.yaml");
    named_pipe(&pipe);
    let output = fixture
        .run_within_10s(&Fixture::args(pipe.to_str().unwrap(), &[]))
        .expect("the command still waits on the named pipe after 10 s");
    let data = refused(&output, 2);
    assert_eq!(data["code"], "invalid_input", "{data}");
    assert_eq!(data["stage"], "arguments", "{data}");
}

/// A vendored source the definition names that is a named pipe is refused the
/// same way: every file the definition names is read as the definition is.
#[cfg(unix)]
#[test]
fn a_vendored_source_that_is_a_named_pipe_is_refused_without_waiting_for_a_writer() {
    let fixture = Fixture::new("", b"replaced by a named pipe");
    let source = fixture
        .root
        .path()
        .join("defs/upstream/forge/openapi.json.gz");
    fs::remove_file(&source).unwrap();
    named_pipe(&source);
    let definition = fixture.definition();
    let output = fixture
        .run_within_10s(&Fixture::args(&definition, &[]))
        .expect("the command still waits on the named pipe after 10 s");
    let data = refused(&output, 2);
    assert_eq!(data["code"], "invalid_input", "{data}");
    assert_eq!(data["stage"], "arguments", "{data}");
}
