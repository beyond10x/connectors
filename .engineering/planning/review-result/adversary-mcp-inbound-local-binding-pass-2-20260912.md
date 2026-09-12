---
format: aep.planning-md/1
id: review-result:adversary-mcp-inbound-local-binding-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — MCP inbound local binding, wave 4a
relations:
- reviews: story:mcp-inbound-local-binding
revision: 1
---
```
unit: mcp-w4a
verdict: red
cases: executed 62→64, red 2
origin: introduced 6, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes
```

One untracked test file; no implementation, contract, scenario or planning file touched. Both cases derive their bound from outside themselves — the lease ceiling through the model's own `# Normative owner:` comment into the sessions contract §4.1, cross-checked against the model's cap; the scenario count out of the story's acceptance. No millisecond literal and no file count appears in the case file.

The first run of case 1 failed on its own derivation guard, a `contains` that missed the author's hard wrap. That is a typo, not a finding; a whitespace normaliser was added and the red below is the re-run, still before any suite run.

Suite: before 64 with the new target deselected, after 64, red 2, exit 101. The unit's 13 checks and pass 1's 3 cases stay green. fmt and clippy exit 0 on the new file.

**Finding 1.** Five acts across four of the eight traces admit data or renew at the exact instant the admitting lease expires. The unit's own guard compares with `>`, which is why they shipped green — the boundary reading it adopted from pass 1's wording is the only thing between this trace set and a red gate. The contract does not support that reading: the gate stops **by** the effective expiry with no grace, the expiry is a local terminal fact that atomically enters `closing`, and the deadline is defined as *including* delivery delay and already-buffered output — so an admission placed on it delivers past it by construction. The unit's own document says "before the effective deadline" twice, and one trace writes the contradiction out loud. Three of the four then close asserting `Ready` at that instant, which §4.1 says is `Closing` with terminal `lease_expired`.

**Finding 2.** The acceptance says six scenario files one to one with six behaviours; the directory carries eight, and no planning record moved with the split. The check that used to pin six now compares the directory against the row count of the document the same unit wrote — both halves the unit's, so nothing compares either to the acceptance. Splitting two behaviours into routes was the right answer to pass 1's finding read as a table defect; read as a deliverable it changed the countable thing the acceptance counts.

**Finding 3.** §3.3 asserts `renew` as a transition the binding performs and the progress row names only `permit_data`, so a reader implementing from the table implements no renewal and kills every request outliving one lease. The new check accepts it because it tests row *or* trace and the trace performs it.

**Finding 4.** The graceful close records `peer_shutdown: confirmed` on the strength of EOF on its own stdin. The archived page the same section cites makes closing that stream step one of three, after which the client waits and may force-terminate — so at EOF the peer has not shut down. The sessions contract names the state precisely and `unconfirmed` is available.

**Findings 5 and 6, both INFEASIBLE.** The document-states-the-obligation check asserts only that the literal `2,000 ms` appears somewhere, and the cutoff ceiling carries the same digits two lines away. And the lease-deadline guard is skipped entirely when no lease was ever issued. Nothing in the tree reaches either.

Attacked and could not break: the parse-error split against the schema's own definitions; the `id`-unreadable licence; the other four derivations, each of which panics loudly when its source sentence moves; the ownership check; the binding coordinates, including both evasions pass 1 named; the renewal against the replay rules; the two double-route behaviours against the acceptance; and all eight traces' teardown and cutoff arithmetic re-derived by hand.

```findings
- file: adapters/mcp/contracts/server/v1alpha1/scenarios/progress-stops-at-the-result.yaml
  line: 87
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Five acts across four of the eight traces admit data or renew a lease at the exact
    instant the admitting lease expires, and the sessions contract stops the gate by that
    expiry with no grace, makes it a terminal fact entering closing, and counts delivery
    delay and buffered output inside the deadline.
- file: .engineering/planning/story/mcp-inbound-local-binding.md
  line: 35
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The acceptance says six scenario files one to one with six behaviours and the directory
    carries eight, no planning record moved with the split, and the check that used to pin
    six now compares the directory against the row count of the document the same unit
    wrote.
- file: adapters/mcp/contracts/server/v1alpha1/semantics.md
  line: 266
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    Section 3.3 asserts renew as a transition the binding performs and the progress row
    names only permit_data, so a reader implementing from the table implements no renewal
    and kills every request that outlives one lease.
- file: adapters/mcp/contracts/server/v1alpha1/scenarios/graceful-stdin-close-ends-the-session.yaml
  line: 119
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The graceful close records peer shutdown as confirmed on the strength of an EOF on its
    own stdin, which the archived page the same section cites makes step one of a
    three-step client shutdown, so the binding records a peer exit it did not observe.
- file: crates/connectors-build/tests/mcp_inbound_local_binding.rs
  line: 1233
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    The check that the document states the lease ceiling asserts only that the literal
    appears somewhere, and the cutoff ceiling carries the same digits two lines away.
- file: crates/connectors-build/tests/mcp_inbound_local_binding.rs
  line: 1064
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    The lease-deadline guard is skipped whenever no lease was ever issued, so a trace that
    admits data with no lease anywhere in it passes every timing check the correction added.
```
