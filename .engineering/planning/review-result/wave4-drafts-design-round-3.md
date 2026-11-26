---
format: aep.planning-md/3
id: review-result:wave4-drafts-design-round-3
kind: review-result
status: active
title: Design critic, round 3, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the ess_model draft `approve`, the provider-budget draft `needs-revision`; the approval
of the MCP draft from round 2 still holds. Read at commit 304f24f16. No build or test was run.

## Round-2 findings

- ess_model draft, finding 1 (`File::lock` against `rust-version` 1.88): resolved by the allow on
  the lock function. The gate's step at 1.88 checks the library of `connectors-sql` alone
  (`crates/connectors-build/src/gate.rs:455-475`).
- ess_model draft, finding 2 (a stale temporary directory): resolved.
- Provider-budget draft, finding 1 (the read POST's own timer): resolved by the second option; a
  new finding follows from it (finding 2 below).
- Provider-budget draft, finding 2 (`timeout_at` polls the connect first): resolved in form; the
  loss it accepts breaks two tests (finding 1 below).

## ess_model draft: no finding

Verified in source: `File::lock` is `flock`, each thread opens a file of its own, so the lock
contends between the threads of one `cargo test` process and between the processes of nextest;
files are opened close-on-exec, so the `ess` child does not hold the lock; a failed generation
drops the file; the rename of a directory onto an absent name is atomic and every caller is
serialised, so no reader sees a partly written directory; the child-test form matches
`crates/connectors-host/tests/provider_timeout_adversary.rs:38-52`. Inferred, not compiled: an
allow on the enclosing function silences `clippy::incompatible_msrv` at the call.

## MCP draft

Verified in source: `Served::server` is called by `Peer::start` and `spawn` alone; `spawn` has one
caller, `paused`; `paused` has two callers, the control and the on-time test. In the tool-call
file `served()` and `Peer::start` serve the walk test and the integral-`max_pages` test alone.

## Provider-budget draft

1. blocker: "With less than the lead left, a call that needs a new connection fails at once with
   the unmarked `Timeout`; that loss is accepted." Two tests of
   `crates/connectors-host/tests/http_budget.rs` fail under it: lines 104-109 (200 ms left, the
   first `get` on a fresh client must answer 200) and lines 164-176 (150 ms left, asserts
   `upstream_answer` and one request sent). The binary is in neither the scope nor acceptance 5.
   Change: set the task-local only when the time left at `send` exceeds a multiple of the lead;
   below that the call is as today; list `http_budget` as passing unchanged.
2. warning: the read POST change is untested and reaches no production caller: `with_read_posts`
   is called by `adapters/grafana/src/local.rs:154` and `adapters/tavily/src/local.rs:75`,
   `with_provider_budget` by `adapters/catalog/src/local.rs:45,1248,2458`. The case item 3 keeps
   marked is contrary to `contracts/cli/v1alpha1/semantics.md:2392`. Change: add one acceptance
   for the read POST, or move it to "Not in this story".

Verified in source: with any layer reqwest uses `Connector::WithLayers` and `connect_timeout` is
the outermost layer (`connect.rs:125-157`); hyper-util calls the connector inside the first poll
of the connect (`service/oneshot.rs:38-48`, `client.rs:498-519`), so the layer sees the
task-local, and the deadline is kept per call; a connector error becomes a connect error
(`client.rs:519`); `PendingRequest::poll` checks its timers before the request
(`async_impl/client.rs:3096-3113`); the client has no redirects, no retries and no proxy
(`crates/connectors-host/src/http.rs:255-257`), and no `h2` package is in `Cargo.lock`;
hyper-util's own retry of a pooled request that was canceled before it was sent calls the
connector again under the same scope.

Open, needs a compile: the generic layer and service against reqwest's private request and
connection types, with a `Send + 'static` future.
