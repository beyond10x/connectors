---
format: aep.planning-md/1
id: review-result:adversary-ess-domain-pass-1
kind: review-result
status: active
title: Adversary pass 1 against the connectors ESS domain
relations:
- reviews: story:connectors-ess-domain
revision: 1
---
# Adversary pass 1 — `story:connectors-ess-domain`

Worktree `wt-2a8ad5b88c79`, branch `impl/connectors-ess-domain`, base `7c66c54` plus the untracked
`ess/` tree. `adp:adversary`. 191,533 tokens, 82 tool uses, 1,166 s.

## Header, as returned

```
verdict: red
cases: executed 57->62, red 5
origin: introduced 10, pre-existing 1, undecided 0
wrote-outside-worktree: 6 paths (all under the assigned scratch root)
needs-coordinator: no
```

## What the attack was

The specification's central claim is that every state, transition and field is read from this
repository and cited by `path:line`, and that everything unreadable carries an `UNMAPPED:` marker
naming what would settle it. The adversary attacked the claim by **opening the citations**, and
wrote a fence — `crates/catalog-build/tests/main/ess_citation_fence.rs`, 337 lines, 5 cases — that
opens them mechanically.

The load-bearing observation: `ess validate` and `bash scripts/gate.sh --final` both stay green
with all eleven findings standing. **Nothing in either opens a cited line.** A citation is a claim
no existing check reads, which is why five of the findings are only reachable by a test that was
written to read them.

## What held

- 49 top-level enums under `crates/protocol/src/`; exactly 7 named `*State`, all 7 modelled.
- Every enum-variant citation matches its cited range exactly, across 8 enums.
- 48 distinct `docs/design/01-domain-model.md` ranges opened line by line; all correct but one.
- All 36 `UNMAPPED:` markers carry a settle clause — 0 without, matching the unit's claim.
- `ess validate` twice: identical. `ess specify compile --format json` twice: 116,158 bytes,
  byte-identical.
- The `Establishing` and `OutcomeUnknown` markers are honest: neither variant is ever assigned.

## Findings

One byte-level change from what the adversary returned, and it is recorded rather than silent: the
last finding's `message` value began with a `"`, so YAML read it as a quoted scalar and the store
refused the whole block — `did not find expected key at line 77 column 50`. The value is now single
quoted and the original opening words are preserved inside it. Every other line is as returned.

This is a defect in the adversary's output shape, not in its finding.

```findings
- file: ess/system/domains/connection.yaml
  line: 612
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: connectors.connection.CandidateNotActivatable carries neither a citation nor an UNMAPPED marker, and no such refusal exists — crates/integration-kubernetes/src/local.rs:286-296 returns the existing Connection for an already-activated candidate.
- file: ess/system/domains/runtime.yaml
  line: 288
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: connectors.runtime.SessionStateConflict carries neither a citation nor an UNMAPPED marker, and crates/integration-sip/src/backend/sessions.rs:45-71 returns Ok(status) for an already-terminated session rather than refusing.
- file: ess/system/domains/catalog.yaml
  line: 53
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the ProviderSummary field citation stops at crates/protocol/src/catalog.rs:85 while the struct runs to :95, dropping configurable and setup_profiles with no UNMAPPED marker.
- file: ess/system/domains/connection.yaml
  line: 148
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the Channel UNMAPPED marker's reason, "ChannelSummary carries no connection_ref at all", is contradicted by crates/protocol/src/event.rs:76 in a file the same document already cites.
- file: ess/system/domains/connection.yaml
  line: 178
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the ConnectSession transitions were read from one registry, and crates/integration-catalog/src/hosted.rs performs the same moves at :383 and :758 without ever being opened.
- file: ess/system/domains/connection.yaml
  line: 303
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: reobserve is declared Withdrawn->Observed but the cited site (integration-monitoring/src/backend.rs:1562) leaves connection_ref set and the derivation at :1659-1660 reads it first, so a materialized observation reobserves into Materialized.
- file: crates/integration-catalog/src/hosted.rs
  line: 758
  category: concurrency
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: Completed is written after two awaits without re-checking Pending, so a concurrent expire_sessions can produce Expired->Completed; I read the window but could not build the interleaving.
- file: ess/system/domains/connection.yaml
  line: 107
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the Connection entity drops auth_profile (crates/protocol/src/connection.rs:199) with no UNMAPPED marker while route gets one, and auth_profile is the input of a command this document declares.
- file: ess/system/domains/connection.yaml
  line: 556
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the not-materializable wrong_state outcome cites a connection_not_found refusal (local.rs:640-643) while the actual wrong-state refusal, integration-monitoring/src/backend.rs:1184-1189, is in an implementation the document never opened.
- file: ess/system/domains/deployment.yaml
  line: 95
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the quotation "every cache is bound to the credential generation that produced the evidence" is cited to docs/design/01-domain-model.md:283 but spans 282-283.
- file: ess/system/domains/connection.yaml
  line: 181
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the claim "which the code enforces at line 204" does not cover fail_pending (connect_session.rs:225-236), the path the same comment cites for Failed at :233, whose own guard is line 228.'
```
