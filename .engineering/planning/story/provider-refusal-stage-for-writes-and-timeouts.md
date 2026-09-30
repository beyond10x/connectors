---
format: aep.planning-md/3
id: story:provider-refusal-stage-for-writes-and-timeouts
kind: story
status: implemented
title: Provider refusals on writes, and provider timeouts, report dispatch with a fitting next action
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/engine.rs
- confidence: cited
  path: adapters/sql/src/lib.rs
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/operations.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/execution.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/process/write_tests.rs
- confidence: cited
  path: crates/connectors-host/tests/provider_timeout_adversary.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T21:31:57Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T21:31:57Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-09-30T00:49:52Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Source

Left over from story:provider-refusal-reports-dispatch-stage (2026-09-29), reported by its implementor:

- A refused write already reports stage `dispatch`, but with next action `retry_status` for every code
  (`apps/connectors/src/local/operations.rs` `project`); a provider 403 on a write should say
  `request_permission`, a 404 `none`. Carrying the origin needs `crates/connectors-host/src/local/owner/mutation.rs`.
- Provider `Timeout` and `Capacity` still report `admission` / `retry_explicitly`, and `Unsupported`
  `admission` / `none` (`apps/connectors/src/local.rs` `owner_failure`).

## Acceptance

- A provider 403 / 404 on a guarded write reports `dispatch` with `request_permission` / `none`.
- A provider timeout or capacity refusal reports `dispatch`; the host's own timeout and capacity refusals keep
  `admission`.
- `contracts/cli/v1alpha1/semantics.md` §6 covers writes.

## Decided for the wave (coordinator, 2026-09-29)

- The catalog adapter maps every refused write status to one `ErrorCode::Forbidden` without the upstream
  mark (`adapters/catalog/src/lib.rs:541-549`). Split it the way reads already do: 400/409/412/422 →
  `InvalidInput`, 401 → `Unauthorized`, 403 → `Forbidden`, 404/410 → `NotFound`, each marked `answered()`;
  any other 4xx keeps `Forbidden`, marked.
- `mutation::Failure` gains `origin`, stored in `StoredOutcome` with `#[serde(default)]` (old ledger
  entries replay as Host). A binary older than this change cannot read a new entry; state that in the
  CHANGELOG line.
- Provider timeout: a request the provider transport sent and whose deadline passed (`http.rs`
  `provider_error`) and the SQL adapter's statement timeout are marked as upstream answers and report
  `dispatch`. A write whose outcome is unknown keeps `outcome_unknown`. Provider capacity: only an
  upstream answer the adapter marks (HTTP 429 stays `rate_limited`); the host's own limits keep
  `admission`. `Unsupported` stays `admission` (every source is raised before dispatch).
- Host approval refusals on writes keep `retry_status`.
- No CLI guarded-write harness exists; host-level tests through the owner's write path are enough.
