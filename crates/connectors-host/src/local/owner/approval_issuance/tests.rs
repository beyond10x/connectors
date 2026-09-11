use super::*;
use crate::local::clock::fixture as clock_fixture;
use crate::local::{clock, keyring::custody, metadata::Metadata};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{collections::BTreeSet, os::unix::fs::PermissionsExt, time::Duration};

const NOW: u64 = 1_788_998_400_000;
const INPUT: &str = r#"{"name":"item","at":"2026-09-11T00:00:00Z"}"#;
fn until() -> Instant {
    Instant::now() + Duration::from_secs(20)
}
struct Target {
    paths: Paths,
    adapter: Adapter,
    connection: String,
    schema: String,
}
impl Target {
    fn request(&self) -> Request<'_> {
        Request {
            connection: &self.connection,
            operation: "item.write",
            schema: &self.schema,
            revision: "desc-1",
            input: INPUT,
        }
    }
    fn keys(&self) -> approval_keys::Store {
        let config = Config::load(&self.paths.config).unwrap();
        key_store(&self.paths, &config, &self.adapter).unwrap()
    }
    fn policy(&self) -> approval_policy::View {
        policy_set(
            &self.paths,
            "fixture",
            br#"{"operations":["item.write"]}"#,
            None,
            until(),
        )
        .unwrap()
    }
    fn fake_issuer(&self) {
        // Metadata-only tests assert no secret qualification. Real issuance
        // below initializes the issuer through its qualified custody owner.
        let metadata = Metadata::update_approval_keys(&self.paths.state).unwrap();
        metadata
            .connection
            .execute(
                "INSERT INTO local_approval_issuers VALUES (?1,'fixture-instance',?2,?3)",
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    uuid::Uuid::new_v4().to_string(),
                    uuid::Uuid::new_v4().to_string()
                ],
            )
            .unwrap();
    }
}
fn setup(root: &Path, socket: Option<&Path>, clock_address: &str) -> Target {
    let paths = Paths {
        config: root.join("config/config.toml"),
        state: root.join("state"),
    };
    Config::initialize(&paths).unwrap();
    let mut config = Config::load(&paths.config).unwrap();
    config.format = "connectors-local/2".into();
    config.secret_service_socket = Some(
        socket
            .map(Path::to_owned)
            .unwrap_or_else(|| root.join("absent-bus")),
    );
    config.approval_clock = Some(clock::Configuration {
        format: "roughtime-clock/1".into(),
        address: clock_address.into(),
        public_key: STANDARD.encode(clock_fixture::root_key()),
        max_rate_error_ppm: 100,
    });
    let adapter = Adapter {
        instance_id: "fixture-instance".into(),
        adapter_id: "fixture-adapter".into(),
        configuration_revision: "cfg-1".into(),
        protocol: "v1alpha1".into(),
        private_protocol: Some(runtime::PrivateProtocol::V2),
        startup: Default::default(),
        restart: Default::default(),
        executable: crate::local::config::Executable {
            path: root.join("absent-adapter"),
            sha256: "a".repeat(64),
            args: vec![],
        },
        permissions: crate::local::config::Permissions {
            profiles: BTreeSet::from(["token".into()]),
            operations: BTreeSet::from(["item.write".into(), "item.read".into()]),
        },
    };
    config.adapters.insert("fixture".into(), adapter.clone());
    std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
    let operations = ["item.write","item.read"].into_iter().map(|id| connectors_core::Operation {
        id:id.into(), description:"fixture operation".into(),contract:"operations/v1alpha1".into(),profile:"resource".into(),
        input_schema:json!({"type":"object","required":["name","at"],"additionalProperties":false,
            "properties":{"name":{"type":"string"},"at":{"type":"string","format":"date-time"}}}),
        output_schema:json!({"type":"object","additionalProperties":false}),
    }).collect();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: adapter.instance_id.clone(),
        adapter: adapter.adapter_id.clone(),
        revision: "desc-1".into(),
        operations,
        configuration_schema: json!({"type":"object"}),
    };
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
            minimum_scopes: BTreeSet::from(["write".into()]),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
        }],
        requirements: vec![
            runtime::Requirement {
                operation: "item.write".into(),
                profile: "token".into(),
                scopes: BTreeSet::from(["write".into()]),
                effect: runtime::Effect::Write,
            },
            runtime::Requirement {
                operation: "item.read".into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            },
        ],
    };
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    let registry = registry::Registry::new(&paths.state);
    let acquisition = registry
        .begin(&bootstrap.binding("token").unwrap(), NOW)
        .unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let candidate = registry
        .prepare(
            &claim,
            registry::ValidatedBaseline {
                identity: registry::ExternalIdentity {
                    kind: "fixture".into(),
                    subject: "owner".into(),
                },
                granted_scopes: Some(BTreeSet::from(["write".into()])),
                credential_expires_at_ms: None,
                collected_at_ms: NOW,
                valid_until_ms: NOW + 60_000,
            },
            12,
            NOW,
        )
        .unwrap();
    let receipt = custody::WrittenVersion::fixture(candidate.version());
    let stored = registry.acknowledge(candidate, receipt, NOW).unwrap();
    let connection = registry.publish(stored, NOW).unwrap();
    let schema = super::super::schema(&bootstrap, "item.write").unwrap();
    Target {
        paths,
        adapter,
        connection,
        schema,
    }
}
fn root() -> tempfile::TempDir {
    let root = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    root
}

#[test]
fn prepare_is_passive_and_digests_the_strict_input_and_current_policy() {
    let root = root();
    let t = setup(root.path(), None, "127.0.0.1:9");
    t.fake_issuer();
    let policy = t.policy();
    let metadata = Metadata::inspect(&t.paths.state).unwrap();
    let path = metadata.connection.path().unwrap().to_owned();
    drop(metadata);
    let observer =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let data_version: i64 = observer
        .query_row("PRAGMA data_version", [], |r| r.get(0))
        .unwrap();
    let prepared = prepare(&t.paths, "fixture", &t.request(), until()).unwrap();
    assert_eq!(prepared.subject.authority.scope.caller, policy.principal());
    assert_eq!(
        prepared.subject.authority.current_authority,
        Some(policy.snapshot().unwrap())
    );
    assert_eq!(
        prepared.subject.input_sha256,
        connectors_core::digest(&serde_json::from_str::<Value>(INPUT).unwrap())
    );
    assert_eq!(
        prepared.subject_sha256,
        connectors_core::digest(&serde_json::to_value(&prepared.subject).unwrap())
    );
    assert!(prepared.subject.route.is_none());
    assert!(prepared.subject.authority.scope.tenant.is_none());
    assert!(
        !serde_json::to_string(&prepared)
            .unwrap()
            .contains("https://")
    );
    assert_eq!(
        observer
            .query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        data_version
    );
    assert!(!t.paths.state.join("owner.sock").exists());
    assert!(!root.path().join("absent-bus").exists());
    let mut request = t.request();
    request.input = r#"{ "at":"2026-09-11T00:00:00Z", "name":"item" }"#;
    assert_eq!(
        prepare(&t.paths, "fixture", &request, until())
            .unwrap()
            .subject,
        prepared.subject
    );
}

#[test]
fn publication_refusal_and_unknown_acknowledgement_preserve_the_protected_file() {
    let root = root();
    let parent = filesystem::directory(root.path(), false, true).unwrap();
    let name = std::ffi::OsStr::new("proof.json");
    let bytes = Secret(br#"{"reference":"fixture","evidence":"protected-fixture"}"#.to_vec());
    assert_eq!(
        publish_checked(&parent, name, &bytes, || Err(Code::Forbidden.into()))
            .unwrap_err()
            .code,
        Code::Forbidden
    );
    assert!(!root.path().join(name).exists());
    let mut checked = 0;
    let result = publish_checked(&parent, name, &bytes, || {
        checked += 1;
        if checked == 2 {
            Err(Code::Unavailable.into())
        } else {
            Ok(())
        }
    });
    assert_eq!(result.unwrap_err().code, Code::OutcomeUnknown);
    assert_eq!(checked, 2);
    assert!(
        filesystem::read_bounded(filesystem::private_file_at(&parent, name).unwrap(), 1024)
            .unwrap()
            == bytes.0
    );
    assert!(publish_checked(&parent, name, &Secret(b"replacement".to_vec()), || Ok(())).is_err());
    assert!(std::fs::read(root.path().join(name)).unwrap() == bytes.0);
}

#[test]
fn policy_and_prepare_refuse_wrong_permissions_pins_input_and_revision() {
    let root = root();
    let t = setup(root.path(), None, "127.0.0.1:9");
    assert!(policy_status(&t.paths, "fixture").unwrap().is_none());
    t.fake_issuer();
    for document in [
        br#"{"operations":["item.read"]}"#.as_slice(),
        br#"{"operations":["missing"]}"#,
        br#"{"operations":[],"issuer":"spoofed"}"#,
        br#"{"operations":[],"operations":["item.write"]}"#,
    ] {
        assert!(policy_set(&t.paths, "fixture", document, None, until()).is_err());
        assert!(policy_status(&t.paths, "fixture").unwrap().is_none());
    }
    let original = t.policy();
    assert_eq!(t.policy_error(None), Code::LifecycleConflict);
    assert_eq!(t.policy_error(Some(2)), Code::LifecycleConflict);
    let mut request = t.request();
    request.revision = "stale";
    assert_eq!(
        prepare(&t.paths, "fixture", &request, until())
            .unwrap_err()
            .code,
        Code::StaleDescription
    );
    request = t.request();
    request.schema = "b";
    assert_eq!(
        prepare(&t.paths, "fixture", &request, until())
            .unwrap_err()
            .code,
        Code::StaleDescription
    );
    for document in [
        r#"{"name":"item","at":"bad-date"}"#,
        r#"{"name":"item","name":"other","at":"2026-09-11T00:00:00Z"}"#,
        r#"{"name":"item","at":"2026-09-11T00:00:00Z","caller":"spoof"}"#,
    ] {
        request = t.request();
        request.input = document;
        assert_eq!(
            prepare(&t.paths, "fixture", &request, until())
                .unwrap_err()
                .code,
            Code::InvalidInput
        );
    }
    let revoked = policy_set(
        &t.paths,
        "fixture",
        br#"{"operations":[]}"#,
        Some(1),
        until(),
    )
    .unwrap();
    assert_eq!(revoked.policy_id, original.policy_id);
    assert_eq!(revoked.revision, 2);
    assert_eq!(
        prepare(&t.paths, "fixture", &t.request(), until())
            .unwrap_err()
            .code,
        Code::Forbidden
    );
}
impl Target {
    fn policy_error(&self, revision: Option<i64>) -> Code {
        policy_set(
            &self.paths,
            "fixture",
            br#"{"operations":[]}"#,
            revision,
            until(),
        )
        .unwrap_err()
        .code
    }
}

#[test]
fn preparation_bounds_the_target_envelope_and_keeps_original_deadline() {
    let root = root();
    let t = setup(root.path(), None, "127.0.0.1:9");
    t.fake_issuer();
    t.policy();
    // The business document alone fits; its projected target wrapper does not.
    let input = format!(
        "{{\"name\":\"{}\",\"at\":\"2026-09-11T00:00:00Z\"}}",
        "x".repeat(TARGET_LIMIT - 80)
    );
    assert!(input.len() < TARGET_LIMIT);
    let mut request = t.request();
    request.input = &input;
    assert_eq!(
        prepare(&t.paths, "fixture", &request, until())
            .unwrap_err()
            .code,
        Code::InvalidInput
    );
    assert_eq!(
        prepare(&t.paths, "fixture", &t.request(), Instant::now())
            .unwrap_err()
            .code,
        Code::Timeout
    );
    let mut config = Config::load(&t.paths.config).unwrap();
    config
        .adapters
        .get_mut("fixture")
        .unwrap()
        .permissions
        .operations
        .clear();
    std::fs::write(&t.paths.config, toml::to_string(&config).unwrap()).unwrap();
    assert_eq!(
        prepare(&t.paths, "fixture", &t.request(), until())
            .unwrap_err()
            .code,
        Code::Forbidden
    );
}

#[test]
fn changed_clock_or_connection_refuses_without_issuing_or_contacting_services() {
    let root = root();
    let t = setup(root.path(), None, "127.0.0.1:9");
    t.fake_issuer();
    t.policy();
    let preparation = prepare(&t.paths, "fixture", &t.request(), until()).unwrap();
    let output = root.path().join("proof.json");
    assert!(matches!(
        issue(
            &t.paths,
            "fixture",
            &t.request(),
            &"0".repeat(64),
            &output,
            until()
        ),
        Err(Error {
            code: Code::Forbidden,
            ..
        })
    ));
    assert!(!output.exists());
    let mut config = Config::load(&t.paths.config).unwrap();
    config.approval_clock.as_mut().unwrap().max_rate_error_ppm = 101;
    std::fs::write(&t.paths.config, toml::to_string(&config).unwrap()).unwrap();
    assert_eq!(
        prepare(&t.paths, "fixture", &t.request(), until())
            .unwrap_err()
            .code,
        Code::Forbidden
    );
    config.approval_clock.as_mut().unwrap().max_rate_error_ppm = 100;
    std::fs::write(&t.paths.config, toml::to_string(&config).unwrap()).unwrap();
    registry::Registry::new(&t.paths.state)
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &t.connection,
            &preparation.subject.target.connection_revision,
            NOW,
        )
        .unwrap();
    assert_eq!(
        prepare(&t.paths, "fixture", &t.request(), until())
            .unwrap_err()
            .code,
        Code::Revoked
    );
}

struct ClockServer {
    address: String,
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[test]
fn clock_exchange_precedes_leases_and_policy_is_rechecked_after_network() {
    let root = root();
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let t = setup(root.path(), None, &socket.local_addr().unwrap().to_string());
    t.fake_issuer();
    t.policy();
    let approved = prepare(&t.paths, "fixture", &t.request(), until()).unwrap();
    let paths = Paths {
        config: t.paths.config.clone(),
        state: t.paths.state.clone(),
    };
    let exchange = std::thread::spawn(move || {
        let mut packet = [0; 4096];
        let (size, peer) = socket.recv_from(&mut packet).unwrap();
        // This exclusive update would fail if the issuing command had already
        // acquired a policy/key lease or retained a metadata lifecycle handle.
        policy_set(&paths, "fixture", br#"{"operations":[]}"#, Some(1), until()).unwrap();
        socket
            .send_to(
                &clock_fixture::Fixture::default().reply(&packet[..size]),
                peer,
            )
            .unwrap();
    });
    let output = root.path().join("revoked-during-clock.json");
    let result = issue(
        &t.paths,
        "fixture",
        &t.request(),
        &approved.subject_sha256,
        &output,
        until(),
    );
    exchange.join().unwrap();
    assert!(matches!(
        result,
        Err(Error {
            code: Code::Forbidden,
            ..
        })
    ));
    assert!(!output.exists());
}
impl ClockServer {
    fn new() -> Self {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let address = socket.local_addr().unwrap().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));
        let stopped = stop.clone();
        let counted = count.clone();
        let thread = std::thread::spawn(move || {
            let mut packet = [0; 4096];
            while !stopped.load(Ordering::SeqCst) {
                match socket.recv_from(&mut packet) {
                    Ok((size, peer)) => {
                        counted.fetch_add(1, Ordering::SeqCst);
                        socket
                            .send_to(
                                &clock_fixture::Fixture::default().reply(&packet[..size]),
                                peer,
                            )
                            .unwrap();
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(e) => panic!("fixture clock socket: {e}"),
                }
            }
        });
        Self {
            address,
            count,
            stop,
            thread: Some(thread),
        }
    }
    fn requests(&self) -> usize {
        self.count.load(std::sync::atomic::Ordering::SeqCst)
    }
}
impl Drop for ClockServer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}

#[test]
#[ignore = "requires built production CLI and qualified disposable GNOME Secret Service"]
fn production_cli_approval_issuance_and_restart() {
    use std::process::Command;
    let executable =
        std::env::var_os("CONNECTORS_TEST_CLI").expect("exact built CLI path required");
    let mut custody = custody::tests::Fixture::new();
    let clock = ClockServer::new();
    let t = setup(custody.root.path(), Some(&custody.socket), &clock.address);
    let run = |args: &[&str], success: bool| {
        let output = Command::new(&executable)
            .args(["--output", "json", "--config"])
            .arg(&t.paths.config)
            .arg("--state-dir")
            .arg(&t.paths.state)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for bytes in [&output.stdout, &output.stderr] {
            assert!(
                !bytes
                    .windows(b"\"evidence\"".len())
                    .any(|w| w == b"\"evidence\"")
            );
        }
        if success {
            assert!(output.stderr.is_empty());
            serde_json::from_slice::<Value>(&output.stdout).unwrap()
        } else {
            assert!(output.stdout.is_empty());
            serde_json::from_slice::<Value>(&output.stderr).unwrap()
        }
    };
    run(&["approvals", "key-init", "--adapter", "fixture"], true);
    let policy_file = custody.root.path().join("policy.json");
    std::fs::write(&policy_file, br#"{"operations":["item.write"]}"#).unwrap();
    run(
        &[
            "approvals",
            "policy-set",
            "--adapter",
            "fixture",
            "--input-file",
            policy_file.to_str().unwrap(),
        ],
        true,
    );
    let arguments = |action| {
        vec![
            "approvals",
            action,
            "--adapter",
            "fixture",
            "--connection",
            &t.connection,
            "--operation",
            "item.write",
            "--schema",
            &t.schema,
            "--revision",
            "desc-1",
            "--input-json",
            INPUT,
        ]
    };
    let prepared = run(&arguments("prepare"), true)["result"]["preparation"].clone();
    let digest = prepared["subject_sha256"].as_str().unwrap();
    assert_eq!(digest, connectors_core::digest(&prepared["subject"]));
    assert_eq!(prepared["subject"].get("route"), Some(&Value::Null));
    for field in ["tenant", "realm", "executor"] {
        assert_eq!(
            prepared["subject"]["authority"]["scope"].get(field),
            Some(&Value::Null)
        );
    }
    assert_eq!(clock.requests(), 0);
    let proof = custody.root.path().join("proof.json");
    let mut args = arguments("issue");
    args.extend([
        "--approve-subject",
        digest,
        "--proof-output",
        proof.to_str().unwrap(),
    ]);
    let publication = run(&args, true)["result"].clone();
    assert_eq!(publication["subject_sha256"], digest);
    assert_eq!(publication["disposition"], "published");
    assert_eq!(clock.requests(), 1);
    let bytes = Secret(std::fs::read(&proof).unwrap());
    assert_eq!(
        std::fs::metadata(&proof).unwrap().permissions().mode() & 0o777,
        0o600
    );
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ProofDocument<'a> {
        reference: &'a str,
        evidence: &'a str,
    }
    let document: ProofDocument<'_> = serde_json::from_slice(&bytes.0).unwrap();
    assert_eq!(publication["reference"], document.reference);
    let evidence = approvals::Evidence::from_protected(
        document.reference.into(),
        Secret(document.evidence.as_bytes().to_vec()),
    )
    .unwrap();
    // Reconstruct current authority independently after the issuing process has
    // exited, and verify the exact proof against its retained public key.
    let resolved = resolve(&t.paths, "fixture", &t.request(), until()).unwrap();
    let source = resolved
        .config
        .approval_clock
        .as_ref()
        .unwrap()
        .acquire()
        .unwrap();
    let policy = policy_store(&t.paths, &t.adapter)
        .unwrap()
        .acquire(&resolved.policy.selection, "item.write")
        .unwrap();
    let key = t.keys().acquire_key().unwrap();
    let admitted = Admission {
        policy: &policy,
        key: &key,
        subject: &resolved.preparation.subject,
    };
    approvals::verify(&evidence, &resolved.preparation.subject, &admitted, &source).unwrap();
    drop(key);
    drop(policy);
    let before = clock.requests();
    run(&args, false); // no overwrite, no fresh clock or seed read
    assert_eq!(clock.requests(), before);
    assert!(bytes.0 == std::fs::read(&proof).unwrap());
    drop(bytes);
    custody.stop();
    custody.start(true);
    assert_eq!(
        run(&arguments("prepare"), true)["result"]["preparation"],
        prepared
    );
    let second = custody.root.path().join("second-proof.json");
    *args.last_mut().unwrap() = second.to_str().unwrap();
    let next = run(&args, true)["result"].clone();
    assert_ne!(next["reference"], publication["reference"]);
    assert!(second.exists());
    // Lock custody: preparation remains passive, issuance cannot claim success.
    custody.stop();
    custody.start(false);
    assert_eq!(
        run(&arguments("prepare"), true)["result"]["preparation"],
        prepared
    );
    let locked = custody.root.path().join("locked-proof.json");
    *args.last_mut().unwrap() = locked.to_str().unwrap();
    run(&args, false);
    assert!(!locked.exists());
    assert!(!t.paths.state.join("owner.sock").exists());
    assert!(!t.adapter.executable.path.exists());
    // Fixture metadata contains no business effect and no approval spend.
    let metadata = Metadata::inspect(&t.paths.state).unwrap();
    assert_eq!(
        metadata
            .connection
            .query_row("SELECT count(*) FROM registry_uses", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
