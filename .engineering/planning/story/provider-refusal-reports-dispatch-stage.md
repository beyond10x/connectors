---
format: aep.planning-md/3
id: story:provider-refusal-reports-dispatch-stage
kind: story
status: implemented
title: A refusal from the provider is reported at dispatch, not as an admission refusal
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: adapters/kubernetes/tests/local_runtime.rs
- confidence: cited
  path: adapters/kubernetes/tests/stage_origin_adversary.rs
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-core/src/lib.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: crates/connectors-sdk/src/lib.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T19:37:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-29T19:37:10Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-09-29T20:39:43Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Observed (knowledge-ingest consumer, 2026-09-29 ~18:58Z, v0.15.1)

One connection, `pipelines.list` on GitLab project 10: status 200. The same connection seconds later on
project 11:
`{"error":{"code":"failure","data":{"code":"forbidden","kind":"operational","next_action":"request_permission","stage":"admission"}},"ok":false}`.
`tags.list` on project 11 through the same connection: status 200. The consumer read this as a permission
gate in connectors and asked what admission checks per project.

## Cause (read in code)

- The catalog provider maps an upstream HTTP 403 to `ErrorCode::Forbidden` and 404/410 to `NotFound`
  (`adapters/catalog/src/lib.rs:621-632`).
- The host carries that as `runtime::Failure::Forbidden` → `owner::Code::Forbidden`
  (`crates/connectors-host/src/local/owner.rs:110`), the same code the host's own admission refusals use
  (`owner.rs:215,250,286`, `owner/supervisor.rs:508,513,...`).
- The CLI derives the stage from the code alone: `Forbidden => ("admission", "request_permission")`,
  `NotFound => ("admission", "check_configuration")` (`apps/connectors/src/local.rs:90-91`).

So GitLab refusing one project's pipelines is reported as connectors refusing admission. What GitLab's
refusal means for project 11 (the account's role, or the project's CI visibility) is not established here.

## Acceptance

- A provider's 403 or 404 on a read or write reaches the CLI with stage `dispatch` (the existing
  `connectors.cli` stage variant), code `forbidden` / `not_found` unchanged, and a next action that does not
  send the operator to connectors configuration (403: `request_permission`; 404: `none`).
- The host's own admission refusals keep stage `admission` and today's next actions.
- A test drives one connection through a fixture provider that answers 200 for one project and 403 for
  another and asserts both answers; a second asserts a host admission refusal still reads `admission`.
- `contracts/cli/v1alpha1/semantics.md` states which stage each origin reports.

## Narrowed for the wave (coordinator, 2026-09-29)

Reads only. A provider 404 keeps its wire code `service_failure` with `service_code: not_found` (it already
reported `dispatch`); only its next action becomes `none`. Writes, and provider timeout, capacity and
unsupported, move to story:provider-refusal-stage-for-writes-and-timeouts.
