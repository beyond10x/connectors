//! `pods.exec`, a mutation, against recorded exec-subresource streams: the
//! binary messages a `v5.channel.k8s.io` server sends, each led by its channel
//! byte, ending in the v1 `Status` on channel 3. No cluster is contacted.
use connectors_core::{Error, ErrorCode, Result};
use connectors_kubernetes::{
    Config, HelmReleaseReads, Kubernetes,
    exec::{self, ExecStream, Upgrade},
};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse, WriteOutcome};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

/// Counts every read request; an exec must never reach this port.
struct Reads(Mutex<usize>);
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        *self.0.lock().unwrap() += 1;
        Ok(HttpResponse {
            status: 500,
            headers: BTreeMap::new(),
            body: Vec::new(),
        })
    }
}

/// A recorded stream: the selected subprotocol and the messages in order. An
/// `Err` entry is the connection being lost at that point.
struct Recorded {
    protocol: &'static str,
    messages: VecDeque<Result<Option<Vec<u8>>>>,
}
#[async_trait::async_trait]
impl ExecStream for Recorded {
    fn protocol(&self) -> &str {
        self.protocol
    }
    async fn next_message(&mut self) -> Result<Option<Vec<u8>>> {
        self.messages.pop_front().unwrap_or(Ok(None))
    }
}

fn frame(channel: u8, data: &[u8]) -> Result<Option<Vec<u8>>> {
    let mut message = vec![channel];
    message.extend_from_slice(data);
    Ok(Some(message))
}
fn success() -> Result<Option<Vec<u8>>> {
    frame(3, br#"{"metadata":{},"status":"Success"}"#)
}
fn exited(code: &str) -> Result<Option<Vec<u8>>> {
    frame(
        3,
        json!({"metadata":{},"status":"Failure",
            "message":format!("command terminated with non-zero exit code: error executing command [false], exit code {code}"),
            "reason":"NonZeroExitCode",
            "details":{"causes":[{"reason":"ExitCode","message":code}]}})
        .to_string()
        .as_bytes(),
    )
}
fn accepted(messages: Vec<Result<Option<Vec<u8>>>>) -> Upgrade {
    Upgrade::Accepted(Box::new(Recorded {
        protocol: "v5.channel.k8s.io",
        messages: messages.into(),
    }))
}

fn kubernetes(pod_exec: bool) -> (Kubernetes, Arc<Reads>) {
    let reads = Arc::new(Reads(Mutex::new(0)));
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["pods".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs: false,
        pod_exec,
        kubeconfig_contexts: false,
    };
    // The local effective document: the federated schema refuses pod_exec.
    let mut effective = json!({"format":"connectors-kubernetes-local/1","instance":"recorded",
        "api_base":"https://fixture.invalid/","ca_digest":null,"namespaces":["engineering"],
        "resource_kinds":["pods"],"discover_hosts":false,"helm_release_reads":"off"});
    if pod_exec {
        effective["pod_exec"] = json!(true);
    }
    let adapter = Kubernetes::new("recorded", config, effective, reads.clone()).unwrap();
    (adapter, reads)
}

fn input() -> Value {
    json!({"namespace":"engineering","pod":"api-0","container":"api",
        "command":["printenv","HOSTNAME"],"max_output_bytes":1024})
}

fn applied(outcome: WriteOutcome<Value>) -> Value {
    match outcome {
        WriteOutcome::Applied(Ok(value)) => value,
        WriteOutcome::Applied(Err(error)) => panic!("applied without a result: {error:?}"),
        WriteOutcome::Refused(error) => panic!("refused: {error:?}"),
        WriteOutcome::Unknown(error) => panic!("unknown: {error:?}"),
    }
}
fn unknown(outcome: WriteOutcome<Value>) -> Error {
    match outcome {
        WriteOutcome::Unknown(error) => error,
        WriteOutcome::Applied(_) => panic!("applied, expected unknown"),
        WriteOutcome::Refused(error) => panic!("refused, expected unknown: {error:?}"),
    }
}
fn refused(outcome: WriteOutcome<Value>) -> Error {
    match outcome {
        WriteOutcome::Refused(error) => error,
        WriteOutcome::Applied(_) => panic!("applied, expected refused"),
        WriteOutcome::Unknown(error) => panic!("unknown, expected refused: {error:?}"),
    }
}

#[tokio::test]
async fn exec_success_returns_its_exit_status_and_bounded_output() {
    let (adapter, reads) = kubernetes(true);
    assert!(
        adapter
            .descriptor()
            .operations
            .iter()
            .any(|o| o.id == "pods.exec")
    );
    let prepared = adapter.prepare_exec(input()).unwrap();
    // The argument vector is sent term by term with no shell in front of it,
    // to the exec subresource, with only stdout and stderr opened.
    assert_eq!(
        prepared.segments(),
        [
            "api",
            "v1",
            "namespaces",
            "engineering",
            "pods",
            "api-0",
            "exec"
        ]
    );
    assert_eq!(
        prepared.query(),
        vec![
            ("container", "api".to_owned()),
            ("command", "printenv".to_owned()),
            ("command", "HOSTNAME".to_owned()),
            ("stdin", "false".to_owned()),
            ("stdout", "true".to_owned()),
            ("stderr", "true".to_owned()),
            ("tty", "false".to_owned()),
        ]
    );
    assert_eq!(exec::PROTOCOLS, ["v5.channel.k8s.io", "v4.channel.k8s.io"]);
    assert_eq!(prepared.timeout(), std::time::Duration::from_secs(30));
    // Preparation made no request at all.
    assert_eq!(*reads.0.lock().unwrap(), 0);

    let result = applied(
        exec::settle(
            prepared,
            accepted(vec![
                // A server opens each channel with an empty message.
                frame(1, b""),
                frame(2, b""),
                frame(1, b"api-0\n"),
                frame(2, b"note\n"),
                success(),
            ]),
        )
        .await,
    );
    assert_eq!(result["exit_code"], 0);
    assert_eq!(result["stdout"], "api-0\n");
    assert_eq!(result["stderr"], "note\n");
    assert_eq!(result["stdout_truncated"], false);
    assert_eq!(result["stderr_truncated"], false);
    assert_eq!(result["container"], "api");
    assert_eq!(
        result["provenance"]["resource"],
        "engineering/pods/api-0/exec"
    );
}

#[tokio::test]
async fn exec_non_zero_exit_is_a_completed_attempt_with_that_status() {
    let (adapter, _) = kubernetes(true);
    let prepared = adapter.prepare_exec(input()).unwrap();
    let result = applied(
        exec::settle(
            prepared,
            accepted(vec![frame(2, b"no such variable\n"), exited("3")]),
        )
        .await,
    );
    assert_eq!(result["exit_code"], 3);
    assert_eq!(result["stderr"], "no such variable\n");
}

#[tokio::test]
async fn exec_output_over_the_bound_is_truncated_and_flagged_and_the_status_still_read() {
    let (adapter, _) = kubernetes(true);
    let prepared = adapter.prepare_exec(bounded_input(4)).unwrap();
    let result = applied(
        exec::settle(
            prepared,
            accepted(vec![
                frame(1, b"abc"),
                frame(1, b"defgh"),
                frame(2, "ab\u{e9}\u{e9}".as_bytes()),
                frame(1, b"discarded"),
                exited("1"),
            ]),
        )
        .await,
    );
    assert_eq!(result["stdout"], "abcd");
    assert_eq!(result["stdout_truncated"], true);
    // "ab\u{e9}" is exactly four bytes; the second "\u{e9}" is discarded.
    assert_eq!(result["stderr"], "ab\u{e9}");
    assert_eq!(result["stderr_truncated"], true);
    assert_eq!(result["exit_code"], 1);

    let prepared = adapter.prepare_exec(bounded_input(3)).unwrap();
    let result = applied(
        exec::settle(
            prepared,
            accepted(vec![frame(1, "ab\u{e9}".as_bytes()), success()]),
        )
        .await,
    );
    // The bound falls inside "é": the incomplete scalar is withheld, not
    // replaced.
    assert_eq!(result["stdout"], "ab");
    assert_eq!(result["stdout_truncated"], true);
}

fn bounded_input(bound: u32) -> Value {
    let mut input = input();
    input["max_output_bytes"] = json!(bound);
    input
}

#[tokio::test]
async fn exec_without_approval_is_refused() {
    // The read path carries no approval and records no attempt: an exec
    // there is refused before any request, even when exec is configured.
    let (adapter, reads) = kubernetes(true);
    let error = adapter.invoke("pods.exec", input()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(*reads.0.lock().unwrap(), 0);
    // Unconfigured, it is not advertised and cannot even be prepared.
    let (adapter, reads) = kubernetes(false);
    assert!(
        !adapter
            .descriptor()
            .operations
            .iter()
            .any(|o| o.id == "pods.exec")
    );
    assert_eq!(
        adapter.prepare_exec(input()).unwrap_err().code,
        ErrorCode::Forbidden
    );
    assert_eq!(*reads.0.lock().unwrap(), 0);
}

#[tokio::test]
async fn exec_lost_stream_is_an_unknown_outcome() {
    let (adapter, _) = kubernetes(true);
    // Lost after output and before the status: the command may still be
    // running or have finished; nothing settles it.
    let lost = unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            accepted(vec![frame(1, b"partial"), Err(Error::unavailable())]),
        )
        .await,
    );
    assert_eq!(lost.code, ErrorCode::Unavailable);
    // A clean close without a status is no settlement either.
    unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            accepted(vec![frame(1, b"partial")]),
        )
        .await,
    );
    // The upgrade request was sent and its answer lost.
    unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            Upgrade::Lost(Error::unavailable()),
        )
        .await,
    );
    // A server error is not proof the command did not start.
    unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            Upgrade::Answered(500),
        )
        .await,
    );
    // A subprotocol that was not offered cannot be decoded safely.
    unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            Upgrade::Accepted(Box::new(Recorded {
                protocol: "channel.k8s.io",
                messages: vec![success()].into(),
            })),
        )
        .await,
    );
    // A failure status that is not an exit code does not say whether the
    // command ran.
    unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            accepted(vec![frame(
                3,
                br#"{"metadata":{},"status":"Failure","reason":"InternalError","message":"error"}"#,
            )]),
        )
        .await,
    );
    // A channel this binding did not open.
    unknown(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            accepted(vec![frame(0, b"stdin?"), success()]),
        )
        .await,
    );
}

#[tokio::test]
async fn exec_refused_before_a_stream_is_a_refusal_not_an_unknown() {
    let (adapter, _) = kubernetes(true);
    for (status, code) in [
        (403, ErrorCode::Forbidden),
        (404, ErrorCode::NotFound),
        (400, ErrorCode::InvalidInput),
    ] {
        let error = refused(
            exec::settle(
                adapter.prepare_exec(input()).unwrap(),
                Upgrade::Answered(status),
            )
            .await,
        );
        assert_eq!(error.code, code);
    }
    refused(
        exec::settle(
            adapter.prepare_exec(input()).unwrap(),
            Upgrade::NotSent(Error::unavailable()),
        )
        .await,
    );
}

#[tokio::test]
async fn exec_outside_the_configured_namespaces_or_bounds_is_refused_before_any_io() {
    let (adapter, reads) = kubernetes(true);
    let mut outside = input();
    outside["namespace"] = json!("kube-system");
    assert_eq!(
        adapter.prepare_exec(outside).unwrap_err().code,
        ErrorCode::Forbidden
    );
    for (field, value) in [
        ("command", json!([])),
        ("command", json!([""])),
        ("command", json!(["x".repeat(4097)])),
        ("container", json!("api/../x")),
        ("pod", json!("api-0/exec")),
        ("timeout_seconds", json!(61)),
        ("max_output_bytes", json!(1_048_577)),
        ("shell", json!(true)),
    ] {
        let mut invalid = input();
        invalid[field] = value;
        assert_eq!(
            adapter.prepare_exec(invalid).unwrap_err().code,
            ErrorCode::InvalidInput,
            "{field}"
        );
    }
    assert_eq!(*reads.0.lock().unwrap(), 0);
}

#[test]
fn exit_codes_are_read_only_from_a_status_this_binding_can_settle() {
    assert_eq!(exec::exit_code(br#"{"status":"Success"}"#), Some(0));
    assert_eq!(
        exec::exit_code(
            br#"{"status":"Failure","reason":"NonZeroExitCode","details":{"causes":[{"reason":"ExitCode","message":"137"}]}}"#
        ),
        Some(137)
    );
    for status in [
        &b""[..],
        b"not json",
        br#"{"status":"Failure","reason":"NonZeroExitCode","details":{"causes":[]}}"#,
        br#"{"status":"Failure","reason":"NonZeroExitCode","details":{"causes":[{"reason":"ExitCode","message":"0"}]}}"#,
        br#"{"status":"Failure","reason":"NonZeroExitCode","details":{"causes":[{"reason":"ExitCode","message":"256"}]}}"#,
        br#"{"status":"Failure","reason":"NonZeroExitCode","details":{"causes":[{"reason":"ExitCode","message":"007"}]}}"#,
        br#"{"status":"Failure","reason":"NonZeroExitCode","details":{"causes":[{"reason":"ExitCode","message":"1"},{"reason":"ExitCode","message":"2"}]}}"#,
        br#"{"status":"Failure","reason":"InternalError"}"#,
    ] {
        assert_eq!(exec::exit_code(status), None);
    }
}
