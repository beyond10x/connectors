---
format: aep.planning-md/3
id: story:failed-connect-reports-its-cause
kind: story
status: draft
title: A connect that fails with a known cause does not report outcome_unknown
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:owner-recovers-after-failed-acquisition
scope:
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/tests/failed_connect.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 6
---
## Observed
Found by the black-box CLI surface test of release 0.18.0 on 2026-09-30 (raw output under the tester's sandbox, outside the repository).
`connections connect --adapter jira --profile atlassian.basic --credential-file <fake>` against an unreachable
provider answers `outcome_unknown` / `retry_status` at stage `publication` (exit 1), while `connections status
--acquisition` then says `failed` / `rejected` and nothing was published. Same on all three catalog adapters and
on 0.17.0. Contradicts semantics.md:370-371 and :436 (`outcome_unknown` means the publication acknowledgement
itself is uncertain).
## Acceptance
- A connect whose acquisition fails before publication answers with the cause the status reports (provider
  unreachable, rejected credential), not `outcome_unknown`.
- `outcome_unknown` remains only for an uncertain publication acknowledgement.

## Decided (coordinator, 2026-09-30)

- Cause (read in code): `Capture::complete` rewrites every `Unavailable`/`Timeout` to `OutcomeUnknown`
  (`crates/connectors-host/src/local/owner/transport.rs:100-102`), including the owner's own `Reply::Failed`
  sent after it recorded the acquisition as `failed`/`rejected` (`:1310-1370`, `Client::value` `:497`).
  `Client::revalidate` does the same (`:430-435`).
- Fix: an error that arrived as a `Reply::Failed` frame keeps its code; only a lost reply (`read_reply` or write
  failure) becomes `outcome_unknown`, which keeps scenario C19. Same for revalidate.
- The CLI reports an owner's definite `unavailable`/`timeout` on connect or revalidate at stage `dispatch` with
  `retry_explicitly`, matching `connections status` for a `failed` acquisition.
- Acceptance narrowed: status records only `rejected`/`expired`, so the answer names the owner's code, not
  "provider unreachable". A scenario row "acquisition failed before publication" joins scenarios.md.
- story:owner-recovers-after-failed-acquisition shares the remap but its main symptom (an owner refusing
  invokes until restarted) is owner state; it stays separate and is re-tested after this lands.
