---
format: aep.planning-md/1
id: review-result:adversary-ess-domain-pass-2
kind: review-result
status: active
title: Adversary pass 2 against the connectors ESS domain
relations:
- reviews: story:connectors-ess-domain
revision: 1
---
# Adversary pass 2 — `story:connectors-ess-domain`

Worktree `wt-2a8ad5b88c79`, branch `impl/connectors-ess-domain`, working tree over `7c66c54`.
`adp:adversary`. 183,169 tokens, 71 tool uses, 917 s.

## Header, as returned

```
verdict: red
cases: executed 62->67, red 5
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: 2 paths (both under the assigned scratch root)
needs-coordinator: no
```

## What the attack was

Pass 1 attacked citation drift and its fence now checks the mechanical part. Pass 2 was told not to
re-run it, and attacked the layer above: **claims the specification makes about the tree that no
check reads.** It wrote a second fence — `crates/catalog-build/tests/main/ess_claim_fence.rs`,
5 cases — where every assertion is an implication: *the document still says X* implies *the tree
still does X*. Correcting either side turns it green, and neither side goes green by accident.

The 62 pre-existing cases, including all five of pass 1's, still pass.

## The one that would have escaped the wave

`naming.wire` is taken verbatim as the address, and `story:cli-surface-contract` — the next unit of
this same wave — generates a committed, byte-gated clap tree from these values. Four of the six are
kebab-case while both request enums are `#[serde(tag = "method", rename_all = "snake_case")]`, so
they name methods every decoder in the tree refuses. The frozen vectors agree:
`contracts/connector-connection/v0alpha1/vectors.json:10,20,28` carry `candidate_search`,
`candidate_activate`, `connect_session_create`.

The two single-word commands, `materialize` and `invoke`, match exactly — which is what shows the
intent was the method tag and not a separate CLI word.

## What held

- All **226** `path:line` citations in `ess/system/` swept for missing files, past-EOF, inverted and
  blank-line endpoints. Four trailing overshoots found, all below the finding threshold.
- The cross-domain claim: `catalog.rs:101` is verbatim "Stable auth-profile id, exactly as
  `ConnectSessionCreateRequest.auth_profile` accepts it". Holds.
- The `wrong_state:` audit: exactly one remains, and `connect_session.rs:204` does refuse from every
  non-`Pending` state. Both halves hold.
- The `Connection` lifecycle against the only drawn one: five transitions and their `from:` sets
  match `docs/design/01-domain-model.md:198-201` exactly.
- Every new type and error citation from correction round 1 points where it says.
- `ess validate --path ess/system` exit 0; the 40 `UNMAPPED:` count is exact.

## Findings, verbatim

```findings
- file: ess/system/domains/connection.yaml
  line: 492
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: four naming.wire values are kebab-case while both request enums are rename_all snake_case, so connect-session-create, candidate-activate, session-terminate and session-reconcile name methods every decoder in the tree refuses, and story:cli-surface-contract generates a committed clap tree from these values.
- file: ess/system/domains/connection.yaml
  line: 608
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: MaterializeObservation says its declared refusal is the one refusal the command performs, and fn materialize at crates/integration-monitoring/src/backend.rs:1177 refuses six times, the first being NotFound on an unknown observation_ref that the sibling ActivateCandidate models as CandidateNotFound.
- file: ess/system/domains/runtime.yaml
  line: 258
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: TerminateSession says not-found is the only refusal the path performs, and fn session_terminate also returns OperationErrorCode::Unavailable from the audit append at crates/integration-sip/src/backend/sessions.rs:65 with no outcome naming it.
- file: ess/system/domains/connection.yaml
  line: 209
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the new UNMAPPED marker says Failed to Completed is reachable on the hosted path, and nothing under crates/integration-catalog/src ever assigns ConnectSessionState::Failed over a map that is built empty and never deserialized, so it names a transition the tree cannot perform.
- file: ess/system/domains/connection.yaml
  line: 208
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the same marker says the hosted path writes Completed with no state guard at all and asks to be settled by taking connect_session.rs:204's guard, which crates/integration-catalog/src/hosted.rs:731 already is, so the marker cannot be closed as written; the real defect is the unguarded re-lookup at :754-758 after the awaits at :748-753 drop the lock.
- file: crates/catalog-build/tests/main/ess_citation_fence.rs
  line: 20
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: cargo fmt -p catalog-build --check reports three hunks in the unit's new fence file and a mod reordering in tests/main.rs, which scripts/gate.sh does not run and therefore never catches.
- file: crates/catalog-build/tests/main/ess_citation_fence.rs
  line: 184
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: every_connect_session_state_write_is_cited greps only "state = ConnectSessionState::" so struct-literal and indirect writes are invisible to it, but no file in the tree escapes it today, so showing the gap needs a constructed fixture.
- file: ess/system/domains/runtime.yaml
  line: 282
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: ReconcileSession.terminated declares moves Session.settle while the handler it cites at crates/integration-sip/src/backend/mod.rs:636-648 only calls session_status, so the command observes the move rather than performing it.
```
