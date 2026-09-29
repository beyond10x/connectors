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
        // The preflight's own provider request was sent and its transport
        // deadline passed, as `http::provider_error` marks it.
        if self.mode == "preflight-provider-timeout" {
            return Err(Failure::from_provider(
                connectors_core::Error::new(ErrorCode::Timeout, "provider request timed out")
                    .answered(),
            ));
        }
        if self.mode == "slow-preflight" {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        // Stands in for a scheduler that spends the first budget before the
        // preparation can answer.
        if self.mode == "stalled-preflight" {
            tokio::time::sleep(Duration::from_millis(2_500)).await;
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
            // The provider's own answer, as the catalog adapter marks it.
            "provider-forbidden" => WriteOutcome::Refused(error().answered()),
            "provider-not-found" => WriteOutcome::Refused(
                connectors_core::Error::new(ErrorCode::NotFound, "private native error").answered(),
            ),
            "unknown" => WriteOutcome::Unknown(error()),
            "applied-error" => WriteOutcome::Applied(Err(error())),
            "bad-output" => WriteOutcome::Applied(Ok(json!({"value":"bad"}))),
            "oversize-output" => {
                WriteOutcome::Applied(Ok(json!({"value":true,"extra":"x".repeat(RESULT_LIMIT)})))
            }
            "lost" => std::process::exit(0),
            "timeout" => {
                // Outlasts every budget `expiring` hands out; the host kills it.
                tokio::time::sleep(Duration::from_secs(600)).await;
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
    let deadline_ms = request.control["deadline_ms"].as_u64().unwrap();
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
    if mode == "peer-expired-eof" {
        record(root, "send");
        // Close at the captured wall deadline. It may precede the parent's
        // independently captured monotonic/socket timeout by a fraction of a ms.
        loop {
            let remaining = deadline_ms.saturating_sub(connectors_sdk::now_ms());
            if remaining == 0 {
                return;
            }
            if remaining > 2 {
                std::thread::sleep(Duration::from_millis(remaining - 1));
            } else {
                std::hint::spin_loop();
            }
        }
    }
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
                prepare(&mut child, HANG_BUDGET_MS).err(),
                Some(Failure::Protocol),
                "{mode}"
            );
            assert_eq!(count(root.path(), "received-commit-or-cancel"), 0, "{mode}");
        } else {
            let pending = prepare(&mut child, HANG_BUDGET_MS).unwrap();
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
/// The budget of every preparation and invocation whose test is not about its
/// deadline expiring: a hang guard, out of reach of a loaded scheduler.
const HANG_BUDGET_MS: u64 = 60_000;

fn prepare(child: &mut Child, budget_ms: u64) -> Result<PreparedInvocation<'_>> {
    prepare_by(child, connectors_sdk::now_ms() + budget_ms)
}
fn prepare_by(child: &mut Child, deadline_ms: u64) -> Result<PreparedInvocation<'_>> {
    child.prepare_write(
        "write",
        "fixture-private-descriptor",
        "partition",
        &Secret(b"fictional-only".to_vec()),
        br#"{"value":true}"#,
        deadline_ms,
    )
}

/// The largest budget `expiring` hands out before it gives up.
const EXPIRING_CAP: Duration = Duration::from_secs(64);

/// Spawn `mode` and prepare it under a deadline meant to expire, then hand
/// the preparation and its budget to `run`. A loaded scheduler can spend the
/// budget before the preparation answers, which is not what these tests
/// assert: an attempt whose preparation failed only after its deadline had
/// passed is discarded, and the next one, on a fresh child and root, gets
/// twice the budget. A preparation that fails before its deadline is a real
/// failure.
fn expiring<R>(
    mode: &str,
    mut run: impl FnMut(&Path, PreparedInvocation<'_>, Duration) -> R,
) -> (tempfile::TempDir, Child, R) {
    let mut budget = Duration::from_secs(2);
    loop {
        let root = temp();
        let mut child = Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
        let deadline_ms = connectors_sdk::now_ms() + budget.as_millis() as u64;
        let outcome = prepare_by(&mut child, deadline_ms).map(|p| run(root.path(), p, budget));
        let failure = match outcome {
            Ok(result) => return (root, child, result),
            Err(failure) => failure,
        };
        assert!(
            connectors_sdk::now_ms() >= deadline_ms && budget < EXPIRING_CAP,
            "{mode}: preparation failed with {failure:?} under a {budget:?} budget"
        );
        // The discarded attempt is still a preparation that outlived its
        // deadline: a socket timeout or a peer that closed at the deadline,
        // an exact child already reaped, and nothing sent.
        assert!(
            matches!(failure, Failure::Timeout | Failure::Unavailable),
            "{mode}: {failure:?}"
        );
        assert!(!child.live, "{mode}: child left live after its deadline");
        assert!(child.process.try_wait().unwrap().is_some(), "{mode}");
        assert_eq!(count(root.path(), "send"), 0, "{mode}");
        budget *= 2;
    }
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
    let pending = prepare(&mut child, HANG_BUDGET_MS).unwrap();
    assert_eq!(count(root.path(), "prepare"), 1);
    assert_eq!(count(root.path(), "send"), 0);
    pending.cancel().unwrap();
    assert_eq!(count(root.path(), "destroy"), 1);
    assert_eq!(count(root.path(), "send"), 0);
    assert!(child.running().unwrap());
    let result = prepare(&mut child, HANG_BUDGET_MS).unwrap().commit();
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
        (
            "peer-expired-eof",
            WriteEffect::Unknown,
            Some(Failure::Timeout),
        ),
    ] {
        // The expiring modes keep a deadline that passes, on a budget `expiring` grows
        // past whatever a loaded scheduler spends before `prepare` returns; the rest never
        // consult theirs. See story:host-suite-subprocess-bounds.
        let (root, mut child, result) = if matches!(mode, "timeout" | "peer-expired-eof") {
            expiring(mode, |_, pending, _| pending.commit())
        } else {
            let root = temp();
            let mut child =
                Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
            let result = prepare(&mut child, HANG_BUDGET_MS).unwrap().commit();
            (root, child, result)
        };
        assert_eq!(result.effect, effect, "{mode}");
        assert_eq!(result.result.err(), failure, "{mode}");
        assert_eq!(count(root.path(), "send"), 1, "{mode}");
        if matches!(mode, "lost" | "timeout" | "peer-expired-eof") {
            assert!(!child.live);
            assert!(child.process.try_wait().unwrap().is_some());
            assert!(prepare(&mut child, HANG_BUDGET_MS).is_err());
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
    assert_eq!(
        prepare(&mut child, HANG_BUDGET_MS).err(),
        Some(Failure::Forbidden)
    );
    assert_eq!(count(root.path(), "prepare"), 1);
    assert_eq!(
        child
            .prepare_write(
                "write",
                "fixture-private-descriptor",
                "partition",
                &Secret(b"fictional-only".to_vec()),
                br#"{"value":"bad"}"#,
                connectors_sdk::now_ms() + HANG_BUDGET_MS
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
            connectors_sdk::now_ms() + HANG_BUDGET_MS
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
                connectors_sdk::now_ms() + HANG_BUDGET_MS
            )
            .is_ok()
    );
}

#[test]
fn drop_eof_and_original_deadline_destroy_pending_without_a_write() {
    for mode in [
        "drop",
        "eof",
        "expiry",
        "slow-preflight",
        "stalled-preflight",
    ] {
        // `drop` and `eof` never consult the deadline, so theirs is out of reach; the
        // other modes expire on a budget `expiring` grows past whatever a loaded
        // scheduler spends before `prepare` returns. See story:host-suite-subprocess-bounds.
        let (root, mut child, ()) = match mode {
            "drop" | "eof" => {
                let root = temp();
                let mut child =
                    Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
                let pending = prepare(&mut child, HANG_BUDGET_MS).unwrap();
                if mode == "eof" {
                    pending
                        .child
                        .channel
                        .shutdown(std::net::Shutdown::Both)
                        .unwrap();
                    // The child exits on EOF; retain the exact owned handle until it does.
                    pending.child.process.wait().unwrap();
                    assert_eq!(count(root.path(), "destroy"), 1);
                }
                drop(pending);
                (root, child, ())
            }
            "expiry" => expiring(mode, |root, pending, budget| {
                let result = channel::read::<Value>(
                    &mut pending.child.channel,
                    Instant::now() + budget + Duration::from_secs(30),
                    false,
                    0,
                );
                assert!(matches!(result, Err(Failure::Unavailable)));
                assert_eq!(count(root, "destroy"), 1);
                drop(pending);
            }),
            _ => expiring(mode, |_, pending, budget| {
                // Sleep past the budget rather than a hand-tuned margin over it.
                std::thread::sleep(budget + Duration::from_millis(100));
                assert_eq!(pending.remaining(), Err(Failure::Timeout));
                let result = pending.commit();
                assert_eq!(result.effect, WriteEffect::Unknown);
                assert_eq!(result.result.err(), Some(Failure::Timeout));
            }),
        };
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
        let pending = prepare(&mut child, HANG_BUDGET_MS).unwrap();
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
    let pending = prepare(&mut child, HANG_BUDGET_MS).unwrap();
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
        assert_eq!(
            prepare(&mut child, HANG_BUDGET_MS).err(),
            Some(Failure::Unsupported)
        );
        assert!(
            child
                .invoke(
                    "read",
                    "fixture-descriptor",
                    "partition",
                    &Secret(b"fictional-only".to_vec()),
                    b"{}",
                    connectors_sdk::now_ms() + HANG_BUDGET_MS
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

/// A guarded write the provider refuses crosses the private boundary and the
/// owner's settlement projection as the provider's refusal, with the next
/// action the CLI reports at `dispatch`.
#[test]
fn a_provider_refusal_of_a_write_names_the_provider_and_a_fitting_next_action() {
    use crate::local::owner::{Origin, mutation};
    for (mode, failure, action) in [
        (
            "provider-forbidden",
            Failure::ProviderForbidden,
            "request_permission",
        ),
        ("provider-not-found", Failure::ProviderNotFound, "none"),
        // A child's plain refusal is not the provider's answer.
        ("refused", Failure::Forbidden, "retry_status"),
    ] {
        let root = temp();
        let mut child = Child::spawn(&selection(root.path(), mode, PrivateProtocol::V2)).unwrap();
        let result = prepare(&mut child, HANG_BUDGET_MS).unwrap().commit();
        assert_eq!(result.effect, WriteEffect::Refused, "{mode}");
        assert_eq!(result.result.as_ref().err(), Some(&failure), "{mode}");
        assert_eq!(count(root.path(), "send"), 1, "{mode}");
        let (classification, outcome) = mutation::native_outcome(result);
        assert_eq!(classification, mutation::Classification::Refused, "{mode}");
        let error = outcome.unwrap_err();
        assert_eq!(
            error.origin,
            if mode == "refused" {
                Origin::Host
            } else {
                Origin::Provider
            },
            "{mode}"
        );
        assert_eq!(error.next_action(classification), action, "{mode}");
        assert_eq!(error.stage(classification), "dispatch", "{mode}");
    }
}

/// A preflight request the provider transport sent and whose deadline passed
/// is the provider's timeout; the host's own expired budget stays the host's.
#[test]
fn a_provider_timeout_is_the_providers_and_the_host_deadline_stays_the_hosts() {
    use crate::local::owner::{self, Code, Origin, mutation};
    let root = temp();
    let mut child = Child::spawn(&selection(
        root.path(),
        "preflight-provider-timeout",
        PrivateProtocol::V2,
    ))
    .unwrap();
    let refused = prepare(&mut child, HANG_BUDGET_MS).err().unwrap();
    assert_eq!(refused, Failure::ProviderTimeout);
    assert_eq!(count(root.path(), "send"), 0);
    let error = owner::Error::from(refused);
    assert_eq!(error.code, Code::Timeout);
    assert_eq!(error.origin, Origin::Provider);
    // The preparation failed, so the write was not attempted (execution.rs).
    let error = mutation::Failure::from(error);
    let not_attempted = mutation::Classification::NotAttempted;
    assert_eq!(error.next_action(not_attempted), "retry_explicitly");
    assert_eq!(error.stage(not_attempted), "dispatch");
    // The host's own deadline: an already expired budget never reaches the child.
    let root = temp();
    let mut child = Child::spawn(&selection(root.path(), "ok", PrivateProtocol::V2)).unwrap();
    let expired = prepare_by(&mut child, connectors_sdk::now_ms() - 1)
        .err()
        .unwrap();
    assert_eq!(expired, Failure::Timeout);
    assert_eq!(count(root.path(), "prepare"), 0);
    let error = owner::Error::from(expired);
    assert_eq!(error.code, Code::Timeout);
    assert_eq!(error.origin, Origin::Host);
    let error = mutation::Failure::from(error);
    assert_eq!(error.next_action(not_attempted), "retry_status");
    assert_eq!(error.stage(not_attempted), "admission");
}
