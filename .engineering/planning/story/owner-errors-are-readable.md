---
format: aep.planning-md/3
id: story:owner-errors-are-readable
kind: story
status: draft
title: An owner failure behind outcome_unknown leaves a readable cause
revision: 1
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
