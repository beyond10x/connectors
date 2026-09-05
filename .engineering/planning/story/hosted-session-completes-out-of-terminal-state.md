---
format: aep.planning-md/1
id: story:hosted-session-completes-out-of-terminal-state
kind: story
status: active
title: A hosted connect session can complete out of a terminal state
relations:
- informed_by: review-result:adversary-ess-domain-pass-1
scope:
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
revision: 4
---
# Story: a hosted connect session can complete out of a terminal state

## What was measured

`crates/integration-catalog/src/hosted.rs:758` writes `current.state =
ConnectSessionState::Completed` with no re-check that the session is still `Pending`. The `Pending`
filter is at `:731`; the lock is then released, `verify_credential` (`:748`) and `commit_connection`
(`:751`) are awaited, and the write happens at `:758`.

`expire_sessions` (`:379-387`) runs at the head of both `hosted_completion_page` (`:708`) and
`complete_hosted_session` (`:723`), and writes `Expired` at `:383`. A concurrent request inside the
await window can therefore write `Expired`, and the later write produces `Expired -> Completed` —
out of a state the domain model treats as terminal
(`ess/system/domains/connection.yaml`, `ConnectSession`).

The Slack path takes a `hosted_completion_lock` (`crates/integration-slack/src/backend.rs:757`);
this path takes none.

## What reaches it

**Not established.** Read, not executed. The adversary that found it reported
`verdict: INFEASIBLE` — it read the window and could not construct the interleaving. Two hosted
completion requests for one session, overlapping inside the two awaits, is the shape; whether any
caller produces that is unmeasured.

This distinction is the finding's size. A window read out of the code is a hypothesis about a race;
it is not evidence that anybody has run one, and the fix's priority turns on which it is.

## Shape

Re-read and re-check the session's state after the awaits and before the write, or take the
completion lock the Slack path already takes. Either makes the terminal state terminal.

## Acceptance

- A test drives two overlapping completions of one hosted session and shows the second does not
  move a session out of `Expired`.
- The state written at `hosted.rs:758` is guarded by a check performed after `commit_connection`
  returns.

## Provenance

`review-result:adversary-ess-domain-pass-1`, finding 7, `origin: pre-existing`. Filed rather than
routed to the implementor, because it is a defect in the repository the ESS specification exposed by
modelling the lifecycle, not a defect the specification introduced.
