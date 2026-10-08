---
format: aep.planning-md/3
id: story:invoke-timeout-after-dispatch-stage
kind: story
status: implemented
title: A read that times out after dispatch reports dispatch, not admission
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T16:02:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T16:02:36Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T17:07:09Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A local `connectors operations invoke` of a read whose deadline passes after the read use was
dispatched (`connectors.credential_evidence.DispatchReadUse` recorded) reports the stage that
happened: `dispatch` (or `outcome_unknown`), not `admission`. Today every timeout that is not the
provider's own maps to `stage: admission`, `next_action: retry_explicitly`
(`apps/connectors/src/local.rs:167`), including the owner's wait on its worker
(`crates/connectors-host/src/local/owner/supervisor.rs:272`) after the request may already have
reached the provider. The metadata store also records the build version of the binary that wrote
each local runtime bootstrap, so a refusal can be traced to the build that produced it.

Observed 2026-10-07 22:31Z on a local store: a Tavily `websearch.search` answered
`{"code":"timeout","stage":"admission"}` while the store shows `DispatchReadUse` at 22:31:35.66 and
`ReleaseReadUse` at 22:31:36.76, against the invoke's fixed 30-second deadline
(`apps/connectors/src/local/session.rs:339`). Which build ran is unknown; the store does not say.

## Acceptance

- A test where the worker dispatches a read and the CLI deadline passes before its reply: the
  refusal carries `stage: dispatch` (or `code: outcome_unknown`), never `stage: admission`.
- A timeout before any dispatch keeps `stage: admission`.
- The local runtime bootstrap record carries the writing build's version; a test reads it back.
- Spec first: the stage mapping and the recorded build version are declared in ESS before the code.
