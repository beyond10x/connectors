---
format: aep.planning-md/3
id: story:absent-operation-reports-not-found
kind: story
status: active
title: An operation id the adapter does not expose answers not_found
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/src/local/operations.rs
- confidence: inferred
  path: apps/connectors/src/local/session.rs
- confidence: cited
  path: apps/connectors/tests/absent_operation.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 8}
---
## Observed
Found by the black-box CLI surface test of release 0.18.0 on 2026-09-30 (raw output under the tester's sandbox, outside the repository).
`operations describe --operation nosuch.op` answers `forbidden` / `request_permission`;
`operations invoke --operation nosuch.op --schema x --revision y` answers `stale_description`.
Contradicts semantics.md:444 and scenarios.md:26 (C05).
## Acceptance
- describe and invoke of an operation id the adapter does not expose answer `not_found`; one that exists but is
  not granted keeps `forbidden`.

## Decided (coordinator, 2026-09-30)

- Cause (read in code): describe checks the grant (`apps/connectors/src/local/operations.rs:53`) before
  existence (`:56`); invoke compares the revision (`crates/connectors-host/src/local/owner.rs:260`) before the
  existence lookup (`:210`); the owner's `Request::Invoke` checks the grant first (`owner/transport.rs:1383`).
- Order everywhere: existence → grant → revision. Approval prepare (`owner/approval_issuance.rs:219`) gets the
  same order. `operations list` keeps hiding ungranted ids; the contract (semantics.md:444-446) separates
  `not_found` from `forbidden` on purpose.
