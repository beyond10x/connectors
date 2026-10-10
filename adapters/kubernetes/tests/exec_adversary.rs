//! Adversary cases for `pods.exec` (wave 20261010e), driven from
//! `contracts/mutations/v1alpha1/semantics.md` §4.2 against `exec::settle`
//! and `Prepared::query`. Recorded streams only; no cluster is contacted.
use connectors_core::{Error, ErrorCode, Result};
use connectors_kubernetes::{
    Config, HelmReleaseReads, Kubernetes,
    exec::{self, ExecStream, Upgrade},
};
use connectors_sdk::{AuthenticatedHttp, HttpResponse, WriteOutcome};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Arc};

struct NoReads;
#[async_trait::async_trait]
impl AuthenticatedHttp for NoReads {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        panic!("an exec reached the read port");
    }
}

struct Recorded(VecDeque<Result<Option<Vec<u8>>>>);
#[async_trait::async_trait]
impl ExecStream for Recorded {
    fn protocol(&self) -> &str {
        "v5.channel.k8s.io"
    }
    async fn next_message(&mut self) -> Result<Option<Vec<u8>>> {
        self.0.pop_front().unwrap_or(Ok(None))
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
fn accepted(messages: Vec<Result<Option<Vec<u8>>>>) -> Upgrade {
    Upgrade::Accepted(Box::new(Recorded(messages.into())))
}

fn kubernetes() -> Kubernetes {
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["pods".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs: false,
        pod_exec: true,
        kubeconfig_contexts: false,
    };
    let effective = json!({"format":"connectors-kubernetes-local/1","instance":"recorded",
        "api_base":"https://fixture.invalid/","ca_digest":null,"namespaces":["engineering"],
        "resource_kinds":["pods"],"discover_hosts":false,"helm_release_reads":"off",
        "pod_exec":true});
    Kubernetes::new("recorded", config, effective, Arc::new(NoReads)).unwrap()
}

fn input(command: Value, max_output_bytes: u32) -> Value {
    json!({"namespace":"engineering","pod":"api-0","container":"api",
        "command":command,"max_output_bytes":max_output_bytes})
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

/// §4.2: "A cut never splits a UTF-8 scalar (an incomplete trailing scalar is
/// withheld); other invalid bytes are replaced with U+FFFD." Output that has
/// an invalid byte earlier and is cut inside "€" must withhold the partial
/// scalar, not replace it.
#[tokio::test]
async fn exec_cut_after_an_invalid_byte_still_withholds_the_incomplete_trailing_scalar() {
    let adapter = kubernetes();
    let prepared = adapter.prepare_exec(input(json!(["cat", "x"]), 4)).unwrap();
    // 0xFF (invalid), 'a', then "€" (E2 82 AC) cut after two bytes by the bound.
    let result = applied(
        exec::settle(
            prepared,
            accepted(vec![frame(1, b"\xFFa\xE2\x82\xAC"), success()]),
        )
        .await,
    );
    assert_eq!(result["stdout_truncated"], true);
    assert_eq!(result["stdout"], "\u{FFFD}a");
}

/// §4.2 outcome table and the transport paragraph: the deadline is reported
/// as `timeout` and a host byte bound as truncation (`capacity`), both
/// `unknown`. The host port reports them with those codes; the attempt's
/// unknown error must keep them rather than flatten them into `unavailable`.
#[tokio::test]
async fn exec_host_deadline_and_byte_bound_keep_their_codes_in_the_unknown_outcome() {
    let adapter = kubernetes();
    for (code, ended) in [
        (
            ErrorCode::Timeout,
            Error::new(ErrorCode::Timeout, "upgraded stream passed its deadline"),
        ),
        (
            ErrorCode::Capacity,
            Error::new(
                ErrorCode::Capacity,
                "upgraded stream reached its byte bound",
            ),
        ),
    ] {
        let prepared = adapter
            .prepare_exec(input(json!(["sleep", "999"]), 64))
            .unwrap();
        let error =
            unknown(exec::settle(prepared, accepted(vec![frame(1, b"partial"), Err(ended)])).await);
        assert_eq!(error.code, code, "{}", error.message);
    }
}

/// The command is an explicit argument vector: each element, shell
/// metacharacters, spaces, newlines, an `&command=` lookalike and non-ASCII
/// included, is one `command` term, in order and unchanged, and no shell is
/// added in front of it.
#[test]
fn exec_arguments_are_separate_unchanged_command_terms() {
    let argv = [
        "printf",
        "a b; rm -rf /",
        "line\nnext",
        "&command=sh&tty=true",
        "%2F..%2F",
        "\u{fc}n\u{ef}c\u{f6}d\u{e9}",
        "$(id)`id`|&>",
    ];
    let prepared = kubernetes().prepare_exec(input(json!(argv), 64)).unwrap();
    let query = prepared.query();
    let commands: Vec<&str> = query
        .iter()
        .filter(|(key, _)| *key == "command")
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(commands, argv);
    let keys: Vec<&str> = query.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        keys.iter()
            .filter(|key| **key != "command")
            .collect::<Vec<_>>(),
        [&"container", &"stdin", &"stdout", &"stderr", &"tty"]
    );
    let tty: Vec<&String> = query
        .iter()
        .filter(|(key, _)| *key == "tty")
        .map(|(_, value)| value)
        .collect();
    assert_eq!(tty, [&"false".to_owned()]);
}

/// A redirect, a 101 the host did not verify, or any status outside the
/// refusal list is no proof the process did not start: never `refused`,
/// never `applied`.
#[tokio::test]
async fn exec_redirect_and_unlisted_answers_are_unknown() {
    let adapter = kubernetes();
    for status in [101, 200, 301, 302, 307, 308, 409, 429, 500, 503] {
        let prepared = adapter.prepare_exec(input(json!(["true"]), 64)).unwrap();
        unknown(exec::settle(prepared, Upgrade::Answered(status)).await);
    }
}

/// Status arriving in two channel-3 messages is one document; output after a
/// settled status does not unsettle it; an exit code of 255 is a completed
/// attempt.
#[tokio::test]
async fn exec_split_status_and_boundary_exit_code_settle() {
    let adapter = kubernetes();
    let status = json!({"metadata":{},"status":"Failure","reason":"NonZeroExitCode",
        "details":{"causes":[{"reason":"ExitCode","message":"255"}]}})
    .to_string();
    let (head, tail) = status.as_bytes().split_at(10);
    let prepared = adapter.prepare_exec(input(json!(["false"]), 64)).unwrap();
    let result = applied(
        exec::settle(
            prepared,
            accepted(vec![
                frame(3, head),
                frame(3, tail),
                Err(Error::unavailable()),
            ]),
        )
        .await,
    );
    assert_eq!(result["exit_code"], 255);
}

/// A stream past the adapter's consumable bound before its status is unknown,
/// even when the status would follow.
#[tokio::test]
async fn exec_over_the_consumable_bound_is_unknown_even_with_a_later_status() {
    let adapter = kubernetes();
    let prepared = adapter.prepare_exec(input(json!(["yes"]), 64)).unwrap();
    let chunk = vec![b'y'; 1024 * 1024];
    let mut messages: Vec<_> = (0..16).map(|_| frame(1, &chunk)).collect();
    messages.push(success());
    let error = unknown(exec::settle(prepared, accepted(messages)).await);
    assert_eq!(error.code, ErrorCode::Capacity);
}
