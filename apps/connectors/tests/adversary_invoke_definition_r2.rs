//! Adversary cases, round 2, for story:invoke-definition-file: the cache-hit
//! check `published()` (`apps/connectors/src/local/definition.rs`) against the
//! same state directory named by another spelling the CLI admits.
//!
//! `--state-dir` is admitted with interior `.` components and repeated
//! separators (`filesystem::validate_path` checks `Path::components`, which
//! normalises both away), and every other command treats `/x/./state` and
//! `/x/state` as one state directory. The definition lowers for real; the
//! catalog provider is a stand-in that refuses at once, so each run stops after
//! the cache step.
#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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
    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        let bin = fixture.root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        private(
            &bin.join("connectors-catalog-provider"),
            b"#!/bin/sh\nexit 1\n",
            0o700,
        );
        let state = fixture.root.path().join("state");
        let init = fixture.run(&state, &["setup", "init"]);
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
        fixture
    }

    /// The CLI with no terminal, the state directory spelled `state`.
    fn run(&self, state: &Path, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .env("CONNECTORS_INSTALLATION", self.root.path().join("bin"))
            .args(["--output", "json", "--config"])
            .arg(self.root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(state)
            .args(args)
            .stdin(Stdio::null());
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
        command.output().unwrap()
    }

    fn invoke(&self, state: &Path) -> Output {
        let definition = self.root.path().join("defs/connectors/forge.yaml");
        self.run(
            state,
            &[
                "operations",
                "invoke",
                "--definition",
                definition.to_str().unwrap(),
                "--operation",
                "project.get",
                "--input-json",
                r#"{"id":"org/project"}"#,
            ],
        )
    }

    /// Each cache entry: its digest, the inode of its native configuration
    /// (a freed inode may be reused) and that configuration's text.
    fn entries(&self) -> Vec<(String, u64, String)> {
        let mut entries: Vec<_> = fs::read_dir(self.root.path().join("state/definitions"))
            .unwrap()
            .map(Result::unwrap)
            .filter(|entry| entry.file_type().unwrap().is_dir())
            .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .map(|entry| {
                (
                    entry.file_name().into_string().unwrap(),
                    fs::metadata(entry.path().join("native.json"))
                        .unwrap()
                        .ino(),
                    fs::read_to_string(entry.path().join("native.json")).unwrap(),
                )
            })
            .collect();
        entries.sort();
        entries
    }
}

/// The same definition, unchanged, run against one state directory named
/// `<root>/state` and `<root>/./state`. A run with unchanged inputs reuses the
/// entry (`semantics.md` §3: "A run whose inputs are unchanged reuses the
/// entry"); `published()` compares the entry's path as text, so each change of
/// spelling removes the entry the other spelling published (under any owner
/// still serving from it) and lowers it again, run after run.
#[test]
fn adversary_a_cache_entry_is_reused_whichever_spelling_names_the_state_directory() {
    let fixture = Fixture::new();
    let plain: PathBuf = fixture.root.path().join("state");
    let dotted = PathBuf::from(format!("{}/./state", fixture.root.path().display()));
    // The stand-in provider refuses after the cache step, every time.
    let first = fixture.invoke(&plain);
    assert!(!first.status.success(), "{first:?}");
    let lowered = fixture.entries();
    assert_eq!(lowered.len(), 1, "{lowered:?} {first:?}");
    let second = fixture.invoke(&dotted);
    assert!(!second.status.success(), "{second:?}");
    assert_eq!(fixture.entries(), lowered, "the other spelling re-lowered");
    let third = fixture.invoke(&plain);
    assert!(!third.status.success(), "{third:?}");
    assert_eq!(fixture.entries(), lowered, "the first spelling re-lowered");
}
