---
format: aep.planning-md/3
id: review-result:wave4-drafts-design-round-2
kind: review-result
status: active
title: Design critic, round 2, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the MCP draft `approve`, the ess_model draft `needs-revision`, the provider-budget draft
`needs-revision`. Read at commit 7ac4c2086. No test was run; one throwaway crate outside the tree
was checked with clippy and removed.

## Round-1 findings

All 9 are resolved in the stories. Finding 9 (`stage = admission`): acceptance 1 of the
provider-budget draft asserts `Failure::from_provider == Failure::Timeout`, which fixes the stage
(`crates/connectors-host/src/local/runtime.rs:524`); the wire `stage` itself is not asserted.

## MCP draft: no finding

Verified in source:

- `expected.0` stays 20,000; it is compared with `execution_ms` and is the read's `ReadBudget`
  (`apps/connectors/src/mcp/projection.rs:300`, `:437`, `:751`); only the operand at `:311`
  changes.
- `Reader::read` calls `ReadBudget::start(20000, …).constrain(until)`; `constrain` shortens the
  deadline to the request's `until` and leaves `execution_ms`
  (`crates/connectors-host/src/local/runtime/bounded.rs:82-95`).
- The owner compares `budget.limits()` with `(20_000, …)`, not the deadline
  (`crates/connectors-host/src/local/owner/governed/bounded.rs:70-75`).
- `configuration.rs:45` accepts any `request_milliseconds` from 1 to 120,000; `stdio.rs:90-96`
  uses it only as `request_time.min(10 s)` for the first snapshot.
- Walk test at budget 4,000, reserve 1,000, delay 1,000: walk deadline about 3 s from the request;
  the result is `stopped: "refused"`, error `timeout`, pages 1 or 2. "About three pages" is
  slightly off (2 to 3). Page 1 has about 1.9 s of headroom on the forge.
- `debug_override_ms` has no per-name logic (`crates/connectors-sdk/src/lib.rs:504-515`);
  `release_build_ignores_a_debug_override` (`:569`) takes an arbitrary name;
  `release_build_has_no_debug_assertions` (`:582`) guards the profile. One test of the helper is
  sound for two new names.

## ess_model draft

1. blocker, line 59: `std::fs::File::lock` is stable since 1.89; `adapters/sql/Cargo.toml` has
   `rust-version.workspace = true` (1.88); the gate runs `cargo clippy --workspace --all-targets
   -- -D warnings` (`crates/connectors-build/src/gate.rs:322-331`). In a throwaway crate with
   `rust-version = "1.88"` and clippy 1.98.1, a plain helper `fn` in `tests/t.rs` calling
   `f.lock()` got `clippy::incompatible_msrv`; the same call inside a `#[cfg(test)]` module did
   not. Change: `#[allow(clippy::incompatible_msrv)]` on the lock helper, with the reason. The
   `--msrv` step does not compile this binary at 1.88 (`gate.rs:455-475`, `:482-495`).
2. warning, line 61: a process killed at 60 s leaves `<key>.<pid>.tmp`; a later process with the
   same pid generates into a non-empty directory. Change: remove it before generating.

Fine: the form is the organization's second accepted form; the key is complete (`ess generate
--path spec/ess` reads only the 4 files); `ess_model.rs` carries `#![cfg(unix)]` (line 6), so
Windows never compiles it; `flock` on separate open files contends across processes; the child
test works under nextest (`crates/connectors-host/tests/provider_timeout_adversary.rs:38-52`),
and should return at once when its guard variable is absent.

## Provider-budget draft

1. warning, lines 91-93: the read POST's own timer is `remaining.min(read_post_timeout)`
   (`crates/connectors-host/src/http.rs:1275-1283`), and `read_post_timeout` may be 1 ms to 180 s
   (`http.rs:407`). When it is shorter than the remaining budget, a stalled handshake is cut by
   the request timer and marked. Change: set the read POST's deadline to the end of the timeout it
   applies less the lead, or write that a cut by `read_post_timeout` stays marked.
2. warning, lines 94-95: `tokio::time::timeout_at` polls the inner future before it checks the
   deadline, so a past deadline still starts the connect. Change: the layer checks the deadline
   before calling the inner service and returns a ready `TimedOut`; say the loss (a fresh
   connection with 1 to 49 ms left) is accepted.

Verified in source:

- `connector_layer` needs `Layer<BoxedConnectorService> + Clone + Send + Sync + 'static`, its
  service `Service<Unnameable, Response = Conn, Error = BoxError> + Clone + Send + Sync + 'static`
  with a `Send + 'static` future (reqwest 0.13.5 `src/async_impl/client.rs:2475-2481`).
  `Unnameable` and `Conn` are in a `pub(crate)` module (`connect.rs:1338`), so the layer must be
  generic over the request type and map its error to `BoxError`.
- The layer wraps DNS, TCP and TLS; `connect_timeout` is outermost (`connect.rs:93-155`).
- The connector is called inline on the caller's task (hyper-util 0.1.21
  `src/client/legacy/client.rs:216-238`, `:416-422`, `:498-519`); the task-local is visible when
  `call` runs. The story can state this as read.
- hyper-util maps the connector error with `e!(Connect, src)` (`client.rs:519`); `is_connect()`
  and `is_timeout()` are both true for an `io::Error` of kind `TimedOut` (`error.rs:119-170`).
- A connect error that is not `Canceled` drops the checkout (`client.rs:467-478`); when the
  checkout wins, the started connect finishes in a background task (`:427-448`) under the
  deadline captured at `call`.
- `PendingRequest::poll` polls its own timers before `in_flight`
  (`async_impl/client.rs:3096-3113`). If the task is not polled for more than the lead, both
  timers have fired and the marked timeout wins: a rare flake risk for acceptance 1 on a loaded
  forge.
- Both ruled-out forms are ruled out correctly; "a client per budget" should carry its reason: it
  loses the shared pool. No `h2` package is in `Cargo.lock`.
