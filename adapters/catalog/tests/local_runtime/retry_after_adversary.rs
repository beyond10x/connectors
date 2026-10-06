//! Adversary cases for story:catalog-honours-retry-after, through the real
//! catalog child.
//!
//! `contracts/cli/v1alpha1/semantics.md`: "a failure carrying
//! `retry_after_seconds` therefore names a delay that did not fit, or the delay
//! the second `429` named"; the acceptance: "A `Retry-After` beyond the
//! deadline ... yields a refusal that states the delay".
use super::*;
use connectors_host::local::runtime::Refusal;
use std::time::Instant;

fn read(
    child: &mut Child,
    revision: &str,
    project: &str,
    deadline_ms: u64,
) -> (Result<Value, Refusal>, Duration) {
    let started = Instant::now();
    let result = child
        .invoke_explained(
            "project.get",
            revision,
            "one",
            &token(true),
            &serde_json::to_vec(&json!({ "id": format!("org/{project}") })).unwrap(),
            connectors_sdk::now_ms() + deadline_ms,
        )
        .map(|bytes| serde_json::from_slice(&bytes).unwrap());
    (result, started.elapsed())
}

fn requests(provider: &Provider, project: &str) -> usize {
    let route = format!("/api/v4/projects/org%2F{project}");
    provider
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.split('?').next() == Some(route.as_str()))
        .count()
}

/// `retry_wait` admits a wait that ends any amount before the deadline
/// (`*end < deadline`, `adapters/catalog/src/local.rs`), leaving the second
/// request no time of its own. A `Retry-After: 1` against a 1.5 s deadline is
/// slept, the second request (800 ms, an ordinary provider latency) cannot
/// finish, and the caller gets a timeout after the whole deadline instead of
/// the `rate_limited` refusal naming one second it would have had at once.
#[test]
fn a_wait_that_leaves_the_second_request_no_time_still_states_the_delay() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let (result, waited) = read(&mut child, &revision, "fixture-adversary-busy-slow", 1_500);
    let refusal = result.expect_err("the fixture never answers 200");
    assert_eq!(
        (refusal.failure, refusal.retry_after_seconds),
        (Failure::ProviderRateLimited, Some(1)),
        "after {waited:?} and {} request(s)",
        requests(&provider, "fixture-adversary-busy-slow")
    );
}

/// `Retry-After` is a singleton field (RFC 9110 §10.2.3). A `429` carrying two
/// conflicting field lines names no delay the engine can read as one; the
/// transport keeps the last line (`BTreeMap::insert`,
/// `crates/connectors-host/src/http.rs`), so the provider's one-hour delay is
/// replaced by one second and the read is sent again after one second.
#[test]
fn conflicting_retry_after_lines_are_not_waited_out_at_the_shorter_one() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let project = "fixture-adversary-busy-conflicting";
    let (result, waited) = read(&mut child, &revision, project, 30_000);
    let refusal = result.expect_err("the fixture never answers 200");
    assert_eq!(refusal.failure, Failure::ProviderRateLimited);
    assert_eq!(
        requests(&provider, project),
        1,
        "sent again after {waited:?}; named {:?}",
        refusal.retry_after_seconds
    );
    assert_ne!(refusal.retry_after_seconds, Some(1));
}
