//! Adversary cases for story:invoke-definition-file
//! (`contracts/cli/v1alpha1/semantics.md` §3, "Invoking a definition file", and
//! the exit table: a user interruption is `interrupted`, exit 130).
//!
//! These cases start no real provider. The definition lowers for real (a
//! hand-made gzip of a small OpenAPI document); the catalog provider the CLI
//! derives the instance from is a stand-in, named by `CONNECTORS_INSTALLATION`,
//! that marks when it was started and then takes its time.
#![cfg(unix)]
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture forge","version":"1"},
"servers":[{"url":"https://gitlab.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getApiV4User","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getApiV4PersonalAccessTokensSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/projects/{id}":{"get":{"operationId":"getApiV4ProjectsId","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}}
}}"#;

fn definition(sha256: &str) -> String {
    format!(
        "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/forge/openapi.json.gz
  sha256: {sha256}
base_url: https://{{hostname}}/api/v4
config: {{hostname: {{type: string, default: gitlab.example.com}}}}
auth:
  forge.pat:
    scheme: header
    header: PRIVATE-TOKEN
    label: Fixture personal access token
    identity: {{path: user, kind: forge.user, subject: /id}}
    scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}
operations:
  project.get:
    op: getApiV4ProjectsId
    description: Read one project
"
    )
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A gzip member holding `data` in one stored deflate block.
fn gzip(data: &[u8]) -> Vec<u8> {
    assert!(data.len() < 65_536);
    let len = data.len() as u16;
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 0x01];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

fn private(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// An initialised configuration and state, a definitions tree whose
    /// definition lowers, and an installation directory whose catalog provider
    /// is `provider` (a shell script body).
    fn new(provider: &str) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        let init = fixture.command(&["setup", "init"]).output().unwrap();
        assert!(init.status.success(), "{init:?}");
        let defs = fixture.root.path().join("defs");
        fs::create_dir_all(defs.join("connectors")).unwrap();
        fs::create_dir_all(defs.join("upstream/forge")).unwrap();
        let sha256 = hex::encode(Sha256::digest(OPENAPI.as_bytes()));
        fs::write(defs.join("connectors/forge.yaml"), definition(&sha256)).unwrap();
        fs::write(
            defs.join("upstream/forge/openapi.json.gz"),
            gzip(OPENAPI.as_bytes()),
        )
        .unwrap();
        let bin = fixture.bin();
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        private(
            &bin.join("connectors-catalog-provider"),
            provider.as_bytes(),
            0o700,
        );
        fixture
    }

    fn bin(&self) -> PathBuf {
        self.root.path().join("bin")
    }

    fn marker(&self) -> PathBuf {
        self.root.path().join("provider-started")
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
            .env("CONNECTORS_INSTALLATION", self.bin())
            .args(["--output", "json", "--config"])
            .arg(self.root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
        command
    }
}

/// SIGINT (the operator's Ctrl-C) arrives while the command prepares the
/// definition: after the signal handlers `prepare` installs are in place
/// (`definition.rs` `run`, `protected::Signals::install`), while the derived
/// provider is being asked for its bootstrap. The command is interrupted:
/// exit 130 with an `interrupted` failure, and it records no instance for the
/// definition. A handler that only sets a flag nobody reads lets the command
/// run on: it derives, records, asks or invokes as if nothing had happened.
#[test]
fn adversary_an_interrupt_while_the_definition_is_prepared_ends_the_command() {
    let marker_placeholder = "@MARKER@";
    let script = format!("#!/bin/sh\n: > '{marker_placeholder}'\n/bin/sleep 4\nexit 1\n");
    // The marker path is only known once the fixture's root exists.
    let fixture = Fixture::new("#!/bin/sh\nexit 1\n");
    private(
        &fixture.bin().join("connectors-catalog-provider"),
        script
            .replace(marker_placeholder, fixture.marker().to_str().unwrap())
            .as_bytes(),
        0o700,
    );
    let definition = fixture.definition();
    let mut child = fixture
        .command(&[
            "operations",
            "invoke",
            "--definition",
            &definition,
            "--operation",
            "project.get",
            "--input-json",
            r#"{"id":"org/project"}"#,
        ])
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    while !fixture.marker().exists() {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            panic!("the provider was never started: {status:?} {output:?}");
        }
        assert!(Instant::now() < until, "the provider was never started");
        std::thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: a live child of this process; a scalar signal number.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "interrupted, exit 130: {output:?}"
    );
    let envelope: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"));
    assert_eq!(
        envelope["error"]["data"]["kind"], "interrupted",
        "{envelope}"
    );
    assert!(
        !fixture.state().join("definitions/instances.toml").exists(),
        "an interrupted command records no instance"
    );
}
