use super::*;
use crate::local::config;
use connectors_sdk::WriteOutcome;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};

fn record(root: &Path, name: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(b"event\n").unwrap();
    file.sync_all().unwrap();
}
fn count(root: &Path, name: &str) -> usize {
    std::fs::read_to_string(root.join(name))
        .unwrap_or_default()
        .lines()
        .count()
}
fn bootstrap(write: bool) -> Bootstrap {
    let mut bootstrap = tests::bootstrap();
    if write {
        let mut descriptor: Value = serde_json::from_str(&bootstrap.descriptor).unwrap();
        let mut operation = descriptor["operations"][0].clone();
        operation["id"] = json!("write");
        operation["input_schema"] = json!({"type":"object","required":["value"],"properties":{"value":{"type":"boolean"}},"additionalProperties":false});
        descriptor["operations"]
            .as_array_mut()
            .unwrap()
            .push(operation);
        descriptor["revision"] = json!("fixture-private-descriptor");
        bootstrap.descriptor = descriptor.to_string();
        bootstrap.requirements.push(Requirement {
            operation: "write".into(),
            profile: "fixture".into(),
            scopes: BTreeSet::new(),
            effect: Effect::Write,
        });
    }
    bootstrap
}
struct Fixture {
    root: PathBuf,
    mode: String,
}
#[async_trait::async_trait]
impl Adapter for Fixture {
    fn bootstrap(&self) -> Bootstrap {
        bootstrap(false)
    }
    fn bootstrap_v2(&self) -> Result<Bootstrap> {
        Ok(bootstrap(true))
    }
    async fn validate(&self, _: &str, _: Secret) -> Result<Baseline> {
        Err(Failure::Unsupported)
    }
    async fn invoke(&self, operation: &str, _: &str, _: Secret, _: Value) -> Result<Value> {
        assert_eq!(operation, "read");
        Ok(json!({"value":true}))
    }
    async fn prepare_write(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Box<dyn PreparedWrite>> {
        assert_eq!(operation, "write");
        assert_eq!(partition, "partition");
        assert_eq!(document.0, b"fictional-only");
        record(&self.root, "prepare");
        if self.mode == "preflight-refused" {
            return Err(Failure::Forbidden);
        }
        if self.mode == "slow-preflight" {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Ok(Box::new(Pending {
            root: self.root.clone(),
            mode: self.mode.clone(),
            secret: document,
            input,
        }))
    }
}
// A composition with only the original Adapter implementation uses the default
// refusal. Its v1 read projection remains usable by the new host.
struct Legacy(Fixture);
#[async_trait::async_trait]
impl Adapter for Legacy {
    fn bootstrap(&self) -> Bootstrap {
        self.0.bootstrap()
    }
    async fn validate(&self, p: &str, s: Secret) -> Result<Baseline> {
        self.0.validate(p, s).await
    }
    async fn invoke(&self, o: &str, p: &str, s: Secret, i: Value) -> Result<Value> {
        self.0.invoke(o, p, s, i).await
    }
}
struct Pending {
    root: PathBuf,
    mode: String,
    secret: Secret,
    input: Value,
}
impl Drop for Pending {
    fn drop(&mut self) {
        record(&self.root, "destroy");
    }
}
#[async_trait::async_trait]
impl PreparedWrite for Pending {
    async fn execute(self: Box<Self>) -> WriteOutcome<Value> {
        assert_eq!(self.secret.0, b"fictional-only");
        assert_eq!(self.input, json!({"value":true}));
        record(&self.root, "send");
        let error = || {
            connectors_core::Error::new(
                ErrorCode::Forbidden,
                "private native error must never escape",
            )
        };
        match self.mode.as_str() {
            "refused" => WriteOutcome::Refused(error()),
            "unknown" => WriteOutcome::Unknown(error()),
            "applied-error" => WriteOutcome::Applied(Err(error())),
            "bad-output" => WriteOutcome::Applied(Ok(json!({"value":"bad"}))),
            "oversize-output" => {
                WriteOutcome::Applied(Ok(json!({"value":true,"extra":"x".repeat(RESULT_LIMIT)})))
            }
            "lost" => std::process::exit(0),
            "timeout" => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                WriteOutcome::Applied(Ok(self.input.clone()))
            }
            _ => WriteOutcome::Applied(Ok(self.input.clone())),
        }
    }
}

#[test]
fn fixture() {
    let args: Vec<_> = std::env::args().collect();
    if !args.iter().any(|arg| arg == "--connectors-private-fd") {
        return;
    }
    let mode = args
        .iter()
        .find_map(|a| a.strip_prefix("write-mode="))
        .unwrap()
        .to_owned();
    let root = PathBuf::from(
        args.iter()
            .find_map(|a| a.strip_prefix("write-root="))
            .unwrap(),
    );
    if mode.starts_with("peer-") {
        peer_fixture(&root, &mode);
        return;
    }
    let adapter = Fixture {
        root,
        mode: mode.clone(),
    };
    if mode == "legacy" {
        let _ = serve(3, Legacy(adapter));
    } else {
        let _ = serve(3, adapter);
    }
}

fn peer_fixture(root: &Path, mode: &str) {
    // An independent peer writes literal JSON instead of the production v2
    // codecs. This checks host admission and failure handling, not just a
    // round-trip between two implementations of the same enum.
    let mut stream = unsafe { UnixStream::from_raw_fd(3) };
    let until = Instant::now() + Duration::from_secs(10);
    let hello = channel::read::<Value>(&mut stream, until, false, 0).unwrap();
    let protocol = if mode == "peer-ready-version" {
        json!(VERSION)
    } else {
        hello.control["version"].clone()
    };
    let mut projection = bootstrap(true);
    if mode == "peer-ready-effect" {
        projection.requirements[1].effect = Effect::Unknown;
    }
    if mode == "peer-ready-instance" {
        projection.instance = "other".into();
    }
    let mut ready = json!({"kind":"ready", "version":protocol,"nonce":hello.control["nonce"],
        "child_incarnation":hello.control["child_incarnation"],"bootstrap":projection});
    if mode == "peer-ready-nonce" {
        ready["nonce"] = json!(uuid::Uuid::new_v4().to_string());
    }
    if mode == "peer-ready-incarnation" {
        ready["child_incarnation"] = json!(uuid::Uuid::new_v4().to_string());
    }
    channel::write(&mut stream, &ready, None, &[], until).unwrap();
    let Ok(request) = channel::read::<Value>(&mut stream, until, true, INPUT_LIMIT) else {
        return;
    };
    record(root, "received-secret");
    let id = request.control["id"].clone();
    let preparation_id = uuid::Uuid::new_v4().to_string();
    let mut prepared = json!({"kind":"prepared_write","id":id,"preparation_id":preparation_id});
    match mode {
        "peer-prepare-id" => prepared["id"] = json!(uuid::Uuid::new_v4().to_string()),
        "peer-prepare-nil" => prepared["preparation_id"] = json!(uuid::Uuid::nil().to_string()),
        "peer-prepare-uuid" => {
            prepared["preparation_id"] = json!("ABCDEFAB-1234-1234-1234-123456789ABC")
        }
        "peer-prepare-extra" => prepared["unreviewed"] = json!(true),
        _ => (),
    }
    channel::write(
        &mut stream,
        &prepared,
        None,
        if mode == "peer-prepare-document" {
            b"{}"
        } else {
            b""
        },
        until,
    )
    .unwrap();
    let Ok(request) = channel::read::<Value>(&mut stream, until, false, 0) else {
        return;
    };
    record(root, "received-commit-or-cancel");
    if mode == "peer-cancel" {
        channel::write(&mut stream,&json!({"kind":"cancelled_write","id":id,"preparation_id":uuid::Uuid::new_v4().to_string()}),None,&[],until).unwrap();
        return;
    }
    assert_eq!(request.control["kind"], "commit_write");
    let mut reply = json!({"kind":"write_result","id":id,"effect":"applied"});
    let mut document = br#"{"kind":"success","value":{"value":true}}"#.to_vec();
    match mode {
        "peer-result-id" => reply["id"] = json!(uuid::Uuid::new_v4().to_string()),
        "peer-result-effect" => reply["effect"] = json!("unreviewed"),
        "peer-result-refused-success" => reply["effect"] = json!("refused"),
        "peer-result-json" => document = b"{".to_vec(),
        "peer-result-extra" => {
            document = br#"{"kind":"failure","code":"forbidden","private":"native error"}"#.to_vec()
        }
        "peer-result-schema" => {
            document = br#"{"kind":"success","value":{"value":"bad"}}"#.to_vec()
        }
        _ => (),
    }
    channel::write(&mut stream, &reply, None, &document, until).unwrap();
    // The parent must reject and reap this exact live child after a bad reply.
    let _ = channel::read::<Value>(&mut stream, until, false, 0);
}

#[test]
fn mismatched_readiness_refuses_before_any_credential_frame() {
    for mode in [
        "peer-ready-version",
        "peer-ready-nonce",
        "peer-ready-incarnation",
        "peer-ready-instance",
        "peer-ready-effect",
        "peer-ready-v1-write",
    ] {
        let root = temp();
        let protocol = if mode == "peer-ready-v1-write" {
            PrivateProtocol::V1
        } else {
            PrivateProtocol::V2
        };
        assert!(
            matches!(
                Child::spawn(&selection(root.path(), mode, protocol)),
                Err(Failure::ReadinessMismatch)
            ),
            "{mode}"
        );
        assert_eq!(count(root.path(), "received-secret"), 0, "{mode}");
    }
}

#[test]
fn malformed_preparation_cancel_and_result_replies_reap_the_exact_child() {
    for mode in [
        "peer-prepare-id",
        "peer-prepare-nil",
        "peer-prepare-uuid",
        "peer-prepare-extra",
        "peer-prepare-document",
        "peer-cancel",
        "peer-result-id",
        "peer-result-effect",
        "peer-result-refused-success",
        "peer-result-json",
        "peer-result-extra",
        "peer-result-schema",
    ] {
        let root = temp();
        let mut child = Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
        if mode.starts_with("peer-prepare-") {
            assert_eq!(
                prepare(&mut child, 2000).err(),
                Some(Failure::Protocol),
                "{mode}"
            );
            assert_eq!(count(root.path(), "received-commit-or-cancel"), 0, "{mode}");
        } else {
            let pending = prepare(&mut child, 2000).unwrap();
            if mode == "peer-cancel" {
                assert_eq!(pending.cancel(), Err(Failure::Protocol));
            } else {
                let result = pending.commit();
                assert_eq!(
                    result.effect,
                    if mode == "peer-result-schema" {
                        WriteEffect::Applied
                    } else {
                        WriteEffect::Unknown
                    },
                    "{mode}"
                );
                assert_eq!(result.result.err(), Some(Failure::Protocol), "{mode}");
            }
            assert_eq!(count(root.path(), "received-commit-or-cancel"), 1, "{mode}");
        }
        assert!(!child.live, "{mode}");
        assert!(child.process.try_wait().unwrap().is_some(), "{mode}");
    }
}
fn selection(root: &Path, mode: &str, protocol: PrivateProtocol) -> config::Adapter {
    static EXECUTABLE: OnceLock<(PathBuf, String)> = OnceLock::new();
    let (path, hash) = EXECUTABLE.get_or_init(|| {
        let path = std::env::current_exe().unwrap();
        let hash = hex::encode(Sha256::digest(std::fs::read(&path).unwrap()));
        (path, hash)
    });
    config::Adapter {
        instance_id: "fixture".into(),
        adapter_id: "fixture".into(),
        configuration_revision: "fixture-config".into(),
        protocol: "v1alpha1".into(),
        private_protocol: Some(protocol),
        startup: config::Startup::OnDemand,
        restart: config::Restart::Never,
        permissions: Default::default(),
        executable: config::Executable {
            path: path.clone(),
            sha256: hash.clone(),
            args: vec![
                "--exact".into(),
                "local::runtime::process::write_tests::fixture".into(),
                "--".into(),
                format!("write-mode={mode}"),
                format!("write-root={}", root.display()),
            ],
        },
    }
}
fn prepare(child: &mut Child, budget_ms: u64) -> Result<PreparedInvocation<'_>> {
    child.prepare_write(
        "write",
        "fixture-private-descriptor",
        "partition",
        &Secret(b"fictional-only".to_vec()),
        br#"{"value":true}"#,
        connectors_sdk::now_ms() + budget_ms,
    )
}
fn temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("private-v2-")
        .tempdir()
        .unwrap()
}

#[test]
fn prepared_request_does_not_send_until_commit_and_cancel_destroys_before_ack() {
    let root = temp();
    let mut child = Child::spawn(&selection(root.path(), "ok", PrivateProtocol::V2)).unwrap();
    let pending = prepare(&mut child, 2000).unwrap();
    assert_eq!(count(root.path(), "prepare"), 1);
    assert_eq!(count(root.path(), "send"), 0);
    pending.cancel().unwrap();
    assert_eq!(count(root.path(), "destroy"), 1);
    assert_eq!(count(root.path(), "send"), 0);
    assert!(child.running().unwrap());
    let result = prepare(&mut child, 2000).unwrap().commit();
    assert_eq!(result.effect, WriteEffect::Applied);
    assert_eq!(result.result.unwrap(), json!({"value":true}));
    assert_eq!(count(root.path(), "send"), 1);
    assert_eq!(count(root.path(), "destroy"), 2);
    let incarnation = child.incarnation.clone();
    child.stop(&incarnation).unwrap();
    assert!(child.process.try_wait().unwrap().is_some());
}

#[test]
fn native_outcomes_and_lost_replies_never_repeat_a_send() {
    for (mode, effect, failure) in [
        ("ok", WriteEffect::Applied, None),
        ("refused", WriteEffect::Refused, Some(Failure::Forbidden)),
        ("unknown", WriteEffect::Unknown, Some(Failure::Forbidden)),
        (
            "applied-error",
            WriteEffect::Applied,
            Some(Failure::Forbidden),
        ),
        ("bad-output", WriteEffect::Applied, Some(Failure::Protocol)),
        (
            "oversize-output",
            WriteEffect::Applied,
            Some(Failure::Protocol),
        ),
        ("lost", WriteEffect::Unknown, Some(Failure::Unavailable)),
        ("timeout", WriteEffect::Unknown, Some(Failure::Timeout)),
    ] {
        let root = temp();
        let mut child = Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
        let result = prepare(&mut child, if mode == "timeout" { 300 } else { 2000 })
            .unwrap()
            .commit();
        assert_eq!(result.effect, effect, "{mode}");
        assert_eq!(result.result.err(), failure, "{mode}");
        assert_eq!(count(root.path(), "send"), 1, "{mode}");
        if matches!(mode, "lost" | "timeout") {
            assert!(!child.live);
            assert!(child.process.try_wait().unwrap().is_some());
            assert!(prepare(&mut child, 2000).is_err());
        }
    }
}

#[test]
fn schema_preflight_and_read_path_refusals_have_no_write() {
    let root = temp();
    let mut child = Child::spawn(&selection(
        root.path(),
        "preflight-refused",
        PrivateProtocol::V2,
    ))
    .unwrap();
    assert_eq!(prepare(&mut child, 2000).err(), Some(Failure::Forbidden));
    assert_eq!(count(root.path(), "prepare"), 1);
    assert_eq!(
        child
            .prepare_write(
                "write",
                "fixture-private-descriptor",
                "partition",
                &Secret(b"fictional-only".to_vec()),
                br#"{"value":"bad"}"#,
                connectors_sdk::now_ms() + 2000
            )
            .err(),
        Some(Failure::InvalidInput)
    );
    assert_eq!(count(root.path(), "prepare"), 1);
    assert_eq!(
        child.invoke(
            "write",
            "fixture-private-descriptor",
            "partition",
            &Secret(b"fictional-only".to_vec()),
            br#"{"value":true}"#,
            connectors_sdk::now_ms() + 2000
        ),
        Err(Failure::Unsupported)
    );
    assert_eq!(count(root.path(), "send"), 0);
    assert!(
        child
            .invoke(
                "read",
                "fixture-private-descriptor",
                "partition",
                &Secret(b"fictional-only".to_vec()),
                b"{}",
                connectors_sdk::now_ms() + 2000
            )
            .is_ok()
    );
}

#[test]
fn drop_eof_and_original_deadline_destroy_pending_without_a_write() {
    for mode in ["drop", "eof", "expiry", "slow-preflight"] {
        let root = temp();
        let mut child = Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
        let pending =
            prepare(&mut child, if mode == "slow-preflight" { 400 } else { 300 }).unwrap();
        match mode {
            "drop" => drop(pending),
            "eof" => {
                pending
                    .child
                    .channel
                    .shutdown(std::net::Shutdown::Both)
                    .unwrap();
                // The child exits on EOF; retain the exact owned handle until it does.
                pending.child.process.wait().unwrap();
                assert_eq!(count(root.path(), "destroy"), 1);
                drop(pending);
            }
            "expiry" => {
                let result = channel::read::<Value>(
                    &mut pending.child.channel,
                    Instant::now() + Duration::from_secs(2),
                    false,
                    0,
                );
                assert!(matches!(result, Err(Failure::Unavailable)));
                assert_eq!(count(root.path(), "destroy"), 1);
                drop(pending);
            }
            _ => {
                std::thread::sleep(Duration::from_millis(250));
                assert_eq!(pending.remaining(), Err(Failure::Timeout));
                let result = pending.commit();
                assert_eq!(result.effect, WriteEffect::Unknown);
                assert_eq!(result.result.err(), Some(Failure::Timeout));
            }
        }
        assert_eq!(count(root.path(), "send"), 0, "{mode}");
        assert!(!child.live);
        assert!(child.process.try_wait().unwrap().is_some());
    }
}

#[test]
fn replacement_material_wrong_ids_and_other_requests_close_pending_without_send() {
    for mode in [
        "id",
        "preparation",
        "deadline",
        "input",
        "secret",
        "invoke",
        "prepare",
        "stop",
        "cancel",
    ] {
        let root = temp();
        let mut child = Child::spawn(&selection(root.path(), "ok", PrivateProtocol::V2)).unwrap();
        let pending = prepare(&mut child, 2000).unwrap();
        let mut request =
            json!({"kind":"commit_write","id":pending.id,"preparation_id":pending.preparation_id});
        match mode {
            "id" => request["id"] = json!(uuid::Uuid::new_v4().to_string()),
            "preparation" => request["preparation_id"] = json!(uuid::Uuid::new_v4().to_string()),
            "deadline" => request["deadline_ms"] = json!(connectors_sdk::now_ms() + 120000),
            "invoke" => {
                request = json!({"kind":"invoke","request_id":"read","operation":"read","revision":"fixture-private-descriptor","partition":"partition","deadline_ms":connectors_sdk::now_ms()+1000})
            }
            "prepare" => {
                request = json!({"kind":"prepare_write","id":pending.id,"operation":"write","revision":"fixture-private-descriptor","partition":"partition","deadline_ms":connectors_sdk::now_ms()+1000})
            }
            "stop" => request = json!({"kind":"stop","request_id":"stop"}),
            "cancel" => {
                request["kind"] = json!("cancel_write");
                request["id"] = json!(uuid::Uuid::new_v4().to_string());
            }
            _ => (),
        }
        let secret = Secret(b"replacement".to_vec());
        channel::write(
            &mut pending.child.channel,
            &request,
            if mode == "secret" {
                Some(&secret)
            } else {
                None
            },
            if mode == "input" { b"{}" } else { b"" },
            pending.until,
        )
        .unwrap();
        assert!(
            channel::read::<Value>(
                &mut pending.child.channel,
                pending.until,
                false,
                RESULT_LIMIT
            )
            .is_err(),
            "{mode}"
        );
        assert_eq!(count(root.path(), "destroy"), 1, "{mode}");
        drop(pending);
        assert_eq!(count(root.path(), "send"), 0, "{mode}");
    }
}

#[test]
fn duplicate_commit_refuses_and_legacy_selection_never_exposes_a_write() {
    let root = temp();
    let mut child = Child::spawn(&selection(root.path(), "ok", PrivateProtocol::V2)).unwrap();
    let pending = prepare(&mut child, 2000).unwrap();
    let request = writes::WriteRequest::Commit {
        id: pending.id.clone(),
        preparation_id: pending.preparation_id.clone(),
    };
    assert_eq!(pending.commit().effect, WriteEffect::Applied);
    let until = Instant::now() + Duration::from_secs(2);
    channel::write(&mut child.channel, &request, None, &[], until).unwrap();
    assert!(channel::read::<Value>(&mut child.channel, until, false, RESULT_LIMIT).is_err());
    assert_eq!(count(root.path(), "send"), 1);
    child.terminate();
    for mode in ["ok", "legacy"] {
        let mut child = Child::spawn(&selection(root.path(), mode, PrivateProtocol::V1)).unwrap();
        assert_eq!(child.bootstrap().descriptor().unwrap().operations.len(), 1);
        assert_eq!(prepare(&mut child, 2000).err(), Some(Failure::Unsupported));
        assert!(
            child
                .invoke(
                    "read",
                    "fixture-descriptor",
                    "partition",
                    &Secret(b"fictional-only".to_vec()),
                    b"{}",
                    connectors_sdk::now_ms() + 2000
                )
                .is_ok()
        );
        channel::write(
            &mut child.channel,
            &request,
            None,
            &[],
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert!(
            channel::read::<Value>(
                &mut child.channel,
                Instant::now() + Duration::from_secs(2),
                false,
                0
            )
            .is_err()
        );
    }
    assert!(Child::spawn(&selection(root.path(), "legacy", PrivateProtocol::V2)).is_err());
    assert_eq!(count(root.path(), "send"), 1);
}
