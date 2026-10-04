//! Independent provider and peer fixture; all authority remains production code.
use connectors_host::local::{
    config::{self, Config, Paths},
    keyring::custody as credential_custody,
    operation_curation, owner, registry, runtime,
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
mod custody;

pub struct Provider {
    pub root: PathBuf,
}
#[async_trait::async_trait]
impl runtime::Adapter for Provider {
    fn bootstrap(&self) -> runtime::Bootstrap {
        serde_json::from_slice(&fs::read(self.root.join("bootstrap.json")).unwrap()).unwrap()
    }
    fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(self.bootstrap())
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(self.bootstrap())
    }
    async fn validate(&self, _: &str, _: Secret) -> runtime::Result<runtime::Baseline> {
        Err(runtime::Failure::Unsupported)
    }
    async fn invoke(&self, _: &str, _: &str, _: Secret, _: Value) -> runtime::Result<Value> {
        panic!("unbounded dispatch")
    }
    async fn invoke_bounded(
        &self,
        op: &str,
        _: &str,
        secret: Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> runtime::Result<Value> {
        budget.until()?;
        assert_eq!(op, "read");
        assert_eq!(secret.0, b"fictional-mcp-test-material");
        let mut calls = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("calls"))
            .unwrap();
        writeln!(calls, "{input}").unwrap();
        calls.sync_all().unwrap();
        if input["n"] == 999 {
            tokio::time::sleep(Duration::from_secs(3)).await;
            budget.until()?;
        }
        Ok(json!({"observed":input["n"]}))
    }
}
fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
pub struct Fixture {
    pub paths: Paths,
    pub connection: String,
    pub root: tempfile::TempDir,
    _custody: custody::Custody,
}
impl Fixture {
    pub fn new() -> Self {
        // SUN_LEN applies to Secret Service's actual pathname, not /proc aliases.
        let parent = PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache");
        let root = tempfile::Builder::new()
            .prefix("mcp-")
            .tempdir_in(parent)
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let custody = custody::Custody::new(root.path());
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut config = Config::load(&paths.config).unwrap();
        config.format = "connectors-local/2".into();
        config.secret_service_socket = Some(custody.socket.clone());
        config.approval_clock = Some(connectors_host::local::clock::Configuration {
            format: "roughtime-clock/1".into(),
            address: "127.0.0.1:1".into(),
            public_key: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                [7u8; 32],
            ),
            max_rate_error_ppm: 10_000,
        });
        let executable = std::env::current_exe().unwrap();
        let digest = ring::digest::digest(&ring::digest::SHA256, &fs::read(&executable).unwrap())
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let adapter = config::Adapter {
            instance_id: "local".into(),
            adapter_id: "fixture".into(),
            configuration_revision: "cfg-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V3),
            startup: Default::default(),
            restart: Default::default(),
            executable: config::Executable {
                path: executable,
                sha256: digest,
                args: vec![
                    "--exact".into(),
                    "native_provider_fixture".into(),
                    "--".into(),
                    format!("mcp-root={}", root.path().display()),
                ],
            },
            permissions: config::Permissions {
                profiles: BTreeSet::from(["token".into()]),
                operations: ["read", "disabled", "hidden", "guarded"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
        };
        config.adapters.insert("fixture".into(), adapter.clone());
        write(&paths.config, toml::to_string(&config).unwrap());
        let descriptor=connectors_core::Descriptor {version:"v1alpha1".into(),instance:adapter.instance_id.clone(),adapter:adapter.adapter_id.clone(),revision:"desc-1".into(),configuration_schema:json!({"type":"object"}),operations:["read","disabled","hidden","guarded"].into_iter().map(|id|connectors_core::Operation {id:id.into(),description:format!("Fixture {id}"),contract:"operations/v1alpha1".into(),profile:"resource".into(),input_schema:json!({"type":"object","properties":{"n":{"type":"number"}},"required":["n"],"additionalProperties":false}),output_schema:json!({"type":"object","properties":{"observed":{"type":"number"}},"required":["observed"],"additionalProperties":false})}).collect()};
        let bootstrap = runtime::Bootstrap {
            instance: adapter.instance_id.clone(),
            adapter: adapter.adapter_id.clone(),
            protocol: adapter.protocol.clone(),
            configuration_revision: adapter.configuration_revision.clone(),
            provider_authority: "https://fixture.invalid".into(),
            descriptor: serde_json::to_string(&descriptor).unwrap(),
            profiles: vec![runtime::Profile {
                id: "token".into(),
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
                acquisition: None,
            }],
            requirements: ["read", "disabled", "hidden", "guarded"]
                .into_iter()
                .map(|op| runtime::Requirement {
                    operation: op.into(),
                    profile: "token".into(),
                    scopes: BTreeSet::new(),
                    effect: runtime::Effect::Read,
                })
                .collect(),
        };
        write(
            &root.path().join("bootstrap.json"),
            serde_json::to_vec(&bootstrap).unwrap(),
        );
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap)
            .unwrap();
        let now = connectors_sdk::now_ms();
        let registry = registry::Registry::new(&paths.state);
        let acquired = registry
            .begin(&bootstrap.binding("token").unwrap(), now)
            .unwrap();
        let claim = registry.consume(acquired, now).unwrap();
        let secret = Secret(b"fictional-mcp-test-material".to_vec());
        let candidate = registry
            .prepare(
                &claim,
                registry::ValidatedBaseline {
                    identity: registry::ExternalIdentity {
                        kind: "fixture".into(),
                        subject: "owner".into(),
                    },
                    granted_scopes: Some(BTreeSet::new()),
                    credential_expires_at_ms: None,
                    collected_at_ms: now,
                    valid_until_ms: now + 60_000,
                },
                secret.0.len(),
                now,
            )
            .unwrap();
        let store =
            credential_custody::Store::open_at(candidate.version().scope(), Some(&custody.socket))
                .unwrap();
        let connection = registry
            .store_and_publish(candidate, &secret, &store, connectors_sdk::now_ms)
            .unwrap();
        let entries:Vec<_>=["read","disabled","hidden","guarded"].into_iter().map(|op|json!({"operation":op,"metadata":{"effects":["network"],"semantic_effects":[],"risk":"low","idempotency":{"kind":"none"},"approval":if op=="guarded" {"required"}else{"not_required"},"realization":"implemented","limits":{"request_bytes":65536,"result_bytes":4194304,"execution_ms":20000,"provider_ms":15000,"connect_ms":5000}}})).collect();
        write(&operation_curation::path(&paths),json!({"format":"connectors-operation-curation/1","adapters":[{"adapter_alias":"fixture","executable_selection":adapter.selection(),"bootstrap_sha256":connectors_core::digest(&serde_json::to_value(&bootstrap).unwrap()),"operations":entries}]}).to_string());
        let fixture = Self {
            paths,
            connection,
            root,
            _custody: custody,
        };
        fixture.exposures(true);
        fixture
    }
    pub fn exposures(&self, enabled: bool) {
        let exposures:Vec<_>=["read","disabled","guarded"].into_iter().map(|op|json!({"adapter_alias":"fixture","operation_ref":op,"connection_ref":self.connection,"families":["tools"],"enabled":op!="disabled"&&enabled})).collect();
        write(&self.companion(),json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,"concurrent_requests":1,"request_milliseconds":120000},"exposures":exposures}).to_string());
    }
    pub fn companion(&self) -> PathBuf {
        let mut p = self.paths.config.as_os_str().to_owned();
        p.push(".mcp-stdio.json");
        p.into()
    }
    pub fn calls(&self) -> usize {
        fs::read_to_string(self.root.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(client) = owner::Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            let _ = client.shutdown(&host);
        }
    }
}

pub struct Peer {
    pub child: std::process::Child,
    receiver: mpsc::Receiver<Value>,
    reader: Option<std::thread::JoinHandle<()>>,
    primary: bool,
}
impl Peer {
    pub fn start(f: &Fixture, primary: bool) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_connectors"))
            .env_clear()
            .arg("--config")
            .arg(&f.paths.config)
            .arg("--state-dir")
            .arg(&f.paths.state)
            .args(["server", "--transport", "stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let value = serde_json::from_str(&line.unwrap()).expect("protocol stdout polluted");
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        let mut peer = Self {
            child,
            receiver,
            reader: Some(reader),
            primary,
        };
        if !primary {
            peer.send(json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"independent-peer","version":"1"}}}));
            let init = peer.recv();
            assert_eq!(init["result"]["protocolVersion"], "2025-11-25", "{init}");
            peer.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        }
        peer
    }
    pub fn send(&mut self, value: Value) {
        self.raw(format!("{value}\n").as_bytes());
    }
    pub fn raw(&mut self, bytes: &[u8]) {
        self.child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
    }
    pub fn recv(&self) -> Value {
        self.receiver
            .recv_timeout(Duration::from_secs(15))
            .expect("production server did not reply")
    }
    pub fn request(&mut self, id: i32, method: &str, mut params: Value) -> Value {
        if self.primary {
            if !params["_meta"].is_object() {
                params["_meta"] = json!({});
            }
            params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
            params["_meta"]["io.modelcontextprotocol/clientCapabilities"] = json!({});
        }
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let result = self.recv();
        assert_eq!(result["id"], id, "{result}");
        result
    }
    pub fn finish(self) {
        self.exit(true);
    }
    pub fn exit(mut self, success: bool) {
        drop(self.child.stdin.take());
        let until = Instant::now() + Duration::from_secs(6);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.success(), success, "{status}");
                break;
            }
            assert!(Instant::now() < until, "stdio process failed to exit");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            self.receiver
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "unsolicited output"
        );
        self.reader.take().unwrap().join().unwrap();
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
