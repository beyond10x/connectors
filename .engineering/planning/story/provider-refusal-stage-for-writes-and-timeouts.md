---
format: aep.planning-md/3
id: story:provider-refusal-stage-for-writes-and-timeouts
kind: story
status: draft
title: Provider refusals on writes, and provider timeouts, report dispatch with a fitting next action
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
