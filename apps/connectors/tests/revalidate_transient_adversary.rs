//! Adversarial cases for story:revalidation-retries-a-transient-failure
//! (beyond10x/connectors#174), driven from contracts/cli/v1alpha1/semantics.md
//! §6 "Revalidation retries" against the production CLI, owner and an adapter
//! child under the read-only `mounted_secret` binding (`support/mounted.rs`).
//!
//! The adapter child answers each validation by the mode the test writes to
//! `/work/mode` and appends one line per validation to `/work/attempts`, so
//! the test counts what the owner actually sent.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_host::local::{registry, runtime};
use mounted::*;
use serde_json::{Value, json};
use std::{
    io::Write,
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "revalidate_adversary_adapter_fixture";

struct Scripted;

#[async_trait::async_trait]
impl runtime::Adapter for Scripted {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    async fn validate(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        let mode = std::fs::read_to_string("/work/mode").unwrap_or_default();
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/work/attempts")
            .unwrap();
        writeln!(log, "{}", mode.trim()).unwrap();
        match mode.trim() {
            // The adapter's or host's own limit, not a marked upstream answer
            // (`Failure::Capacity`, as `Failure::from_service` maps an
            // unanswered `ErrorCode::Capacity`).
            "capacity" => return Err(runtime::Failure::Capacity),
            "unavailable" => return Err(runtime::Failure::Unavailable),
            _ => {}
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
        _: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        Err(runtime::Failure::Unsupported)
    }
}

#[test]
fn revalidate_adversary_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Scripted);
    }
}

fn attempts(container: &Container) -> usize {
    std::fs::read_to_string(container.path("/work/attempts"))
        .unwrap_or_default()
        .lines()
        .count()
}

fn connected() -> (Container, String, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    assert_eq!(attempts(&container), 1, "the connect sent one validation");
    (
        container,
        summary["connection"].as_str().unwrap().to_owned(),
        summary["revision"].as_str().unwrap().to_owned(),
    )
}

fn revalidate(container: &Container, connection: &str, revision: &str) -> std::process::Output {
    container.run(&[
        "connections",
        "revalidate",
        "--adapter",
        ALIAS,
        "--connection",
        connection,
        "--expected-revision",
        revision,
    ])
}

/// semantics.md §6 lists "a provider timeout, rate limit, internal error or
/// *marked* capacity answer" as transient. The adapter's own unmarked
/// `capacity` is not one: the owner must not send it again. Today
/// `transient` (supervisor.rs:100) folds `F::Capacity` into
/// `provider_capacity`, sends it three times, and the CLI then reports it at
/// stage `admission` (owner_failure: host `Capacity`) with neither `cause`
/// nor `attempts`: a failure that claims nothing was dispatched after three
/// validations were.
#[test]
fn an_unmarked_capacity_refusal_is_not_sent_again_or_names_its_attempts() {
    let (container, connection, revision) = connected();
    std::fs::write(container.path("/work/mode"), "capacity").unwrap();
    let (code, data) = refusal(&revalidate(&container, &connection, &revision));
    let sent = attempts(&container) - 1;
    assert_eq!(code, Some(1), "{data}");
    if sent > 1 {
        // Retried: then §6 says the failure names its cause and attempts.
        assert_eq!(
            (&data["stage"], &data["cause"], &data["attempts"]),
            (
                &json!("dispatch"),
                &json!("provider_capacity"),
                &json!(sent)
            ),
            "the owner sent {sent} validations: {data}"
        );
    }
    assert_eq!(
        sent, 1,
        "an unmarked capacity refusal was sent {sent} times: {data}"
    );
}

/// semantics.md §6: the owner retries "only while … the owner is not
/// stopping", and the failure carries `attempts`, "the number of validations
/// the owner sent, when it sent any", with `cause` `owner_stopping` "when the
/// owner began to stop during the revalidation". The owner stops during the
/// 1 s pause after the second failed attempt.
#[test]
fn an_owner_stop_during_the_retries_names_owner_stopping_and_the_attempts_sent() {
    let (container, connection, revision) = connected();
    std::fs::write(container.path("/work/mode"), "unavailable").unwrap();
    let (output, stop) = std::thread::scope(|scope| {
        let pending = scope.spawn(|| revalidate(&container, &connection, &revision));
        let until = Instant::now() + Duration::from_secs(20);
        // Connect's validation and two of the revalidation's: the owner is
        // now in its 1 s pause before the third.
        while attempts(&container) < 3 {
            assert!(Instant::now() < until, "the revalidation never sent two");
            std::thread::sleep(Duration::from_millis(10));
        }
        let stop = container.run(&["setup", "owner-stop"]);
        (pending.join().unwrap(), stop)
    });
    let sent = attempts(&container) - 1;
    let (code, data) = refusal(&output);
    assert_eq!(
        (code, &data["code"], &data["cause"], &data["attempts"]),
        (
            Some(1),
            &json!("unavailable"),
            &json!("owner_stopping"),
            &json!(2)
        ),
        "owner-stop: {}\nsent {sent}: {data}",
        String::from_utf8_lossy(&stop.stderr)
    );
    assert_eq!(
        sent, 2,
        "a validation was sent after the stop began: {data}"
    );
}

/// The `--output json` success of a revalidation that needed a retry writes
/// nothing to stderr.
#[test]
fn a_retried_revalidation_that_succeeds_writes_nothing_to_stderr() {
    let (container, connection, revision) = connected();
    // One transient failure, then the adapter answers: switch the mode back
    // once the first attempt is recorded.
    std::fs::write(container.path("/work/mode"), "unavailable").unwrap();
    let output = std::thread::scope(|scope| {
        let pending = scope.spawn(|| revalidate(&container, &connection, &revision));
        let until = Instant::now() + Duration::from_secs(20);
        while attempts(&container) < 2 {
            assert!(Instant::now() < until, "the revalidation never sent one");
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::write(container.path("/work/mode"), "ok").unwrap();
        pending.join().unwrap()
    });
    let revalidated = success(&output);
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(attempts(&container), 3);
}
