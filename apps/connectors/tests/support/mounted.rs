//! The production CLI against the read-only `mounted_secret` binding, in a jail
//! (`connectors_custody::mounted::fixture`): its own user and mount
//! namespaces, the test's user as itself (not root), a root directory the test
//! owns, and the custody directory bound read-only from a source the test
//! updates the way a kubelet does.
//!
//! The jail holds copies of the `connectors` binary and of this test binary,
//! which is also the adapter child: the owner starts it filtered to
//! [`ADAPTER_FIXTURE`], and it serves [`Fixture`] on descriptor 3, recording
//! the digest of every credential it validates or invokes with under `/work`.
//! Every command runs with an empty environment and only `--config` and
//! `--state-dir`.
#![allow(dead_code)]
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::local::{registry, runtime};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Output, Stdio},
};

pub const CONFIG: &str = "/srv/connectors/config.toml";
pub const STATE: &str = "/srv/connectors/state";
/// The configured custody directory: bound read-only from [`SOURCE`].
pub const CUSTODY: &str = "/run/connectors/custody";
pub const SOURCE: &str = "/srv/secret";
pub const STAMP: &str = "..2026_10_13_00_00_00.000000001";
pub const ALIAS: &str = "forge";
pub const PROFILE: &str = "fixture.token";
pub const OAUTH_PROFILE: &str = "fixture.oauth";
pub const READ: &str = "fixture.whoami";
pub const REVISION: &str = "desc-1";
pub const SECRET: &[u8] = b"fictional-mounted-token-0001";

pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn input_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{}})
}
fn output_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"credential":{"type":"string"}},"required":["credential"]})
}

fn profile(id: &str, acquisition: Option<runtime::Acquisition>) -> runtime::Profile {
    runtime::Profile {
        id: id.into(),
        revision: "profile-1".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_bearer".into(),
        capability: "http-bearer".into(),
        minimum_scopes: BTreeSet::new(),
        evidence_lifetime_ms: 60_000,
        fields: vec![runtime::EntryField {
            name: "token".into(),
            label: "Token".into(),
            max_bytes: 1024,
        }],
        acquisition,
    }
}

/// The fixture adapter's description: one read under a static profile, and
/// an OAuth-acquired profile, the refresh-rotating kind.
pub fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: REVISION.into(),
        operations: vec![connectors_core::Operation {
            id: READ.into(),
            description: "names the credential it was invoked with".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: input_schema(),
            output_schema: output_schema(),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![
            profile(PROFILE, None),
            profile(
                OAUTH_PROFILE,
                Some(runtime::Acquisition {
                    authorize_url: "https://accounts.fixture.invalid/auth".into(),
                    token_url: "https://accounts.fixture.invalid/token".into(),
                    scopes: BTreeSet::from(["fixture.read".to_owned()]),
                }),
            ),
        ],
        requirements: vec![runtime::Requirement {
            operation: READ.into(),
            profile: PROFILE.into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

/// The adapter child: each validation and invocation appends the digest of
/// the credential it received to `/work/validations` or `/work/invocations`.
pub struct Fixture;

fn record(name: &str, secret: &[u8]) {
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new("/work").join(name))
        .unwrap();
    writeln!(log, "{}", digest(secret)).unwrap();
}

#[async_trait::async_trait]
impl runtime::Adapter for Fixture {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    async fn validate(
        &self,
        _: &str,
        document: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        record("validations", &document.0);
        if document.0 == b"refused" {
            return Err(runtime::Failure::InvalidCredential);
        }
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            identity: registry::ExternalIdentity {
                kind: "fixture.user".into(),
                subject: "fixture-subject-1".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 50_000,
        })
    }
    async fn invoke(
        &self,
        _: &str,
        _: &str,
        document: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        record("invocations", &document.0);
        Ok(json!({"credential": digest(&document.0)}))
    }
}

/// Serve [`Fixture`] when this binary is the owner's adapter child.
pub fn serve_when_child() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Fixture);
    }
}

/// One jail with the CLI, the adapter, a configuration and a custody source.
pub struct Container {
    pub jail: Jail,
    adapter_digest: String,
    adapter_args: Vec<String>,
}

impl Container {
    /// `test_binary_filter` is the fixture test that serves the adapter.
    pub fn new(adapter_fixture: &str) -> Self {
        // The child refuses a description outside the protocol's bounds and
        // closes its channel unanswered, with its stderr discarded: the CLI
        // then reports only `unavailable` at `readiness`. Hold the description
        // to the child's own check here, where the failure names itself.
        bootstrap()
            .validate_for(runtime::PrivateProtocol::V1)
            .expect("the fixture adapter's description passes the child's own readiness check");
        let mut jail = Jail::new();
        jail.directory("/srv", 0o755);
        jail.directory("/srv/connectors", 0o700);
        jail.directory(SOURCE, 0o755);
        jail.directory(CUSTODY, 0o755);
        jail.directory("/work", 0o700);
        jail.read_only(SOURCE, CUSTODY);
        jail.install(
            Path::new(env!("CARGO_BIN_EXE_connectors")),
            "/bin-test/connectors",
        );
        let adapter = jail.install(&std::env::current_exe().unwrap(), "/bin-test/adapter");
        let adapter_digest = digest(&fs::read(adapter).unwrap());
        Self {
            jail,
            adapter_digest,
            adapter_args: vec!["--exact".into(), adapter_fixture.into(), "--".into()],
        }
    }

    /// The host path of a path inside the jail.
    pub fn path(&self, inside: &str) -> PathBuf {
        self.jail.path(inside)
    }

    /// Write the configuration owner-only: `format`, the `[custody]` table
    /// `custody` (TOML, or empty) and the fixture adapter as `forge`.
    pub fn configure(&self, format: &str, custody: &str) {
        let uid = connectors_host::local::filesystem::uid();
        let text = format!(
            "format='{format}'\nowner_uid={uid}\n{custody}\n[adapters.{ALIAS}]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.{ALIAS}.permissions]\nprofiles=['{PROFILE}','{OAUTH_PROFILE}']\noperations=['{READ}']\n[adapters.{ALIAS}.executable]\npath='/bin-test/adapter'\nsha256='{}'\nargs={}\n",
            self.adapter_digest,
            serde_json::to_string(&self.adapter_args).unwrap()
        );
        let path = self.path(CONFIG);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// The `[custody]` table selecting the mounted directory.
    pub fn mounted() -> String {
        format!("[custody]\nbinding='mounted_secret'\ndirectory='{CUSTODY}'\n")
    }

    /// Lay the source out as a kubelet atomic writer does: the item in a
    /// timestamped directory, `..data` naming it and a top-level alias link.
    pub fn deliver(&self, profile: &str, bytes: &[u8]) {
        let source = self.path(SOURCE);
        let item = source.join(STAMP).join(ALIAS).join(profile);
        fs::create_dir_all(item.parent().unwrap()).unwrap();
        fs::write(&item, bytes).unwrap();
        fs::set_permissions(&item, fs::Permissions::from_mode(0o400)).unwrap();
        if source.join(ALIAS).symlink_metadata().is_err() {
            symlink(Path::new("..data").join(ALIAS), source.join(ALIAS)).unwrap();
        }
        if source.join("..data").symlink_metadata().is_err() {
            symlink(STAMP, source.join("..data")).unwrap();
        }
    }

    /// A kubelet update: the new bytes in a new timestamped directory, then
    /// `..data` renamed onto it.
    pub fn rotate(&self, stamp: &str, bytes: &[u8]) {
        let source = self.path(SOURCE);
        let item = source.join(stamp).join(ALIAS).join(PROFILE);
        fs::create_dir_all(item.parent().unwrap()).unwrap();
        fs::write(&item, bytes).unwrap();
        fs::set_permissions(&item, fs::Permissions::from_mode(0o400)).unwrap();
        symlink(stamp, source.join("..data_tmp")).unwrap();
        fs::rename(source.join("..data_tmp"), source.join("..data")).unwrap();
    }

    /// Run the CLI in the jail with an empty environment and only the two
    /// path globals.
    pub fn run(&self, args: &[&str]) -> Output {
        Jail::output(
            self.jail
                .command("/bin-test/connectors", Map::Own)
                .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
                .args(args)
                .env_clear()
                .stdin(Stdio::null()),
        )
    }

    /// The digests the adapter child recorded in one of its logs.
    pub fn seen(&self, log: &str) -> Vec<String> {
        fs::read_to_string(self.path("/work").join(log))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Every entry under `inside`: its relative path, mode, link target or
    /// bytes. Equal snapshots mean nothing under it changed.
    pub fn snapshot(&self, inside: &str) -> BTreeMap<PathBuf, (u32, Vec<u8>)> {
        let root = self.path(inside);
        let mut entries = BTreeMap::new();
        let mut pending = vec![root.clone()];
        while let Some(path) = pending.pop() {
            for entry in fs::read_dir(&path).unwrap() {
                let path = entry.unwrap().path();
                let meta = fs::symlink_metadata(&path).unwrap();
                let content = if meta.file_type().is_symlink() {
                    fs::read_link(&path)
                        .unwrap()
                        .into_os_string()
                        .into_encoded_bytes()
                } else if meta.is_dir() {
                    pending.push(path.clone());
                    Vec::new()
                } else if meta.file_type().is_socket() {
                    // The owner's live socket (`owner.sock`) cannot be
                    // opened and holds no stored bytes; its mode still
                    // compares.
                    Vec::new()
                } else {
                    fs::read(&path).unwrap()
                };
                entries.insert(
                    path.strip_prefix(&root).unwrap().to_owned(),
                    (meta.mode(), content),
                );
            }
        }
        entries
    }

    /// Whether any file under the state directory holds `bytes`.
    pub fn state_holds(&self, bytes: &[u8]) -> bool {
        self.snapshot(STATE)
            .values()
            .any(|(_, content)| content.windows(bytes.len()).any(|window| window == bytes))
    }

    pub fn schema(&self) -> String {
        connectors_host::local::owner::schema(&bootstrap(), READ).unwrap()
    }

    pub fn invoke(&self, connection: &str) -> Output {
        let schema = self.schema();
        self.run(&[
            "operations",
            "invoke",
            "--adapter",
            ALIAS,
            "--connection",
            connection,
            "--operation",
            READ,
            "--schema",
            &schema,
            "--revision",
            REVISION,
            "--input-json",
            "{}",
        ])
    }

    pub fn connect(&self, profile: &str) -> Output {
        self.run(&[
            "connections",
            "connect",
            "--adapter",
            ALIAS,
            "--profile",
            profile,
        ])
    }
}

/// The successful result, or a failure naming the output.
pub fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

/// The refusal's exit status and `error.data`.
pub fn refusal(output: &Output) -> (Option<i32>, Value) {
    assert!(!output.status.success(), "{output:?}");
    let value: Value =
        serde_json::from_slice(&output.stderr).unwrap_or_else(|_| panic!("{output:?}"));
    (output.status.code(), value["error"]["data"].clone())
}
