---
format: aep.planning-md/3
id: approval-record:wave-d243-c1
kind: approval-record
status: approved
title: 'Wave D243-C1: GitHub App token, service-account admission, served keyed writes'
relations:
- decides: story:definition-lowerer-reads-published-features
- decides: story:catalog-github-app-installation-token
- decides: story:served-instance-admits-a-service-account-token
- decides: story:served-keyed-writes-under-a-standing-approval
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-30T03:15:03Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
---
## Wave D243-C1

Four units on the integration branch `wave/d243-c1` (tree `conn-d243-c1`, base `origin/main`
v0.59.3 `5843c939e`), one full gate at the end, one pull request through the bot, release
v0.61.0 (or the next minor if another wave claims it first).

| unit | story | depends on |
|---|---|---|
| C0 | `story:definition-lowerer-reads-published-features` | none |
| C1 | `story:catalog-github-app-installation-token` | none |
| C2 | `story:served-instance-admits-a-service-account-token` | none |
| C3 | `story:served-keyed-writes-under-a-standing-approval` | C2 |

`story:git-smart-http-relay-with-quarantine` (C4) is accepted and belongs to the next wave.

## Approval

The orchestrator session, 2026-10-30, under the grant of the workspace instructions (class C: a
wave, its commits, pull request, merge and release): "run wave D243-C1 of the connectors
repository. Approved under the grant".

The operator's direction, 2026-10-30, relayed verbatim:
- "we want to inject a synthetic tool which is basically proxying the thing, so the secret lives
  outside on the connector's side or the tool side ... and the agent is just calling a tool via
  some proxy, effectively never seeing the secret"
- "use llm-gateway or Substrate, and wire it into Loom so other apps besides takt can use it"

Decisions the orchestrator took for the design of 2026-10-30: B1, a projected service-account
token checked with TokenReview; C1, git smart-HTTP with quarantine; D1, a configurable pre-push
check on the instance. The standing write approval is option A of
`decision-blocker:standing-write-approval-mode` (operator, 2026-10-22).

## Decided in the wave

- C0 merges the committed tip of `wave/2026-10-29-definitions-runtime` into this branch, so the
  install FEATURES list and the lowerer can be held equal by a test (D247). That wave's own pull
  request stays its owner's; whichever reaches `main` first carries those commits.
- C3, served placement: the standing grant is part of the instance's mounted configuration,
  written by the operator and read on every call; see the story's Decided section.
