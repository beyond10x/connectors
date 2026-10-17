---
format: aep.planning-md/3
id: story:revalidate-retries-transient-failure
kind: story
status: implemented
title: Revalidation retries a transient failure and names its cause
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T20:58:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T20:58:29Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T21:04:06Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Observed
`connections revalidate` failed a caller's run with `unavailable` at stage `dispatch` and
`retry_explicitly`, while the next attempt succeeded (issue
https://github.com/beyond10x/connectors/issues/174; the same failure is in
docs/evidence/gitlab-current-20261002). Cause: the adapter child's pooled HTTP client is built
with `retry::never()` (`crates/connectors-host/src/http.rs:180`) and its current-thread runtime
runs only inside `block_on` (`crates/connectors-host/src/local/runtime/server.rs:143,231`), so a
keep-alive connection the provider closed while idle is reused by the next identity probe and
fails; the owner validated once and the CLI dropped the cause.

## Acceptance
- A revalidation whose pooled connection was closed by the provider while idle succeeds on a new
  connection (`apps/connectors/tests/revalidate_transient.rs`), red before the fix with the
  reported envelope.
- A transient failure (unavailable, timeout, rate limit, internal, capacity) is retried at most
  3 times inside the revalidation deadline; a 401 or 403 is sent exactly once.
- A failure that remains names its `cause` and `attempts` in the error data; code, stage and
  next action are unchanged (`ess/domains/cli.yaml`, `contracts/cli/v1alpha1/semantics.md` §6).
