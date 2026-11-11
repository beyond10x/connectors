---
format: aep.planning-md/3
id: story:owner-errors-are-readable
kind: story
status: active
title: An owner failure behind outcome_unknown leaves a readable cause
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:30Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:31Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Observed
A CLI write whose owner channel fails for any reason (send or reply read) is reported as
`outcome_unknown` at stage `publication` with `attempt: null` and `source_audit` unavailable
(`crates/connectors-host/src/local/owner/transport.rs:716-734`,
`apps/connectors/src/local.rs:181`, `apps/connectors/src/local/operations.rs:204-207`). The owner
process is started with stderr to null (`owner/transport.rs:1000`), so its own error is lost. On
2026-10-16 this appeared intermittently in the guarded_merge journey on `main` and in the
OpenRouter custody journey; its cause could not be read.

## Acceptance
- The owner's diagnostic output goes to a bounded, owner-only log file under the state
  directory, never to the CLI's stdout or stderr (the `--output json` stderr carries exactly one
  envelope on failure and nothing on success).
- An `outcome_unknown` caused by an owner-side failure names that log's path and the owner's
  error code in its error data; no secret reaches the log (a test writes a created credential
  through a failing owner and finds it in no log byte).
- The intermittent guarded_merge `outcome_unknown` is reproduced at most three times (bounded
  load) and its cause recorded here, or reasoned from the code.

## Cause of the guarded write outcome_unknown

Recorded 2026-11-10 for the third Acceptance line. Reasoned from the code, not reproduced: the guarded_merge journey needs a Secret Service and a catalog provider. The fix is drafted as story:write-reply-waits-out-the-owners-delivery-grace.

- `WriteClient::invoke` (crates/connectors-host/src/local/owner/transport.rs) sends the write, then reads the
  reply with `channel::read_with_cancel(.., until, ..)`, where `until = deadline.until()`: the write's own
  mutation deadline, with no grace added.
- Any failure of that send/read closure, including the timeout, is mapped by
  `.map_err(|_| Error::from(Code::OutcomeUnknown))`.
- The owner, in `exchange_write` (same file), runs `write_action` up to that same deadline and then writes
  its reply with a separate 5 s "bounded delivery grace" (`Instant::now() + Duration::from_secs(5)`), so a
  terminal answer may legitimately arrive after the client stopped waiting.
- Under host load a guarded write that spends nearly its whole budget (custody unlock, adapter start,
  dispatch) answers after `until`: the client reports `outcome_unknown` at stage `publication`, `attempt`
  null, `source_audit` unavailable (`operations.rs` outcome_unknown arm), though the owner finished.
- The read client already has the grace: `ANSWER_GRACE` (2 s) in `Client::invoke`; `WriteClient::invoke` has none.
- Fix candidate: wait out the owner's delivery grace in `WriteClient::invoke`'s reply read.
