---
format: aep.planning-md/1
id: review-result:adversary-fences-pass-2
kind: review-result
status: active
title: Adversary pass 2 against the rewritten CLI surface fences
relations:
- reviews: story:cli-surface-fences-assert-what-they-claim
revision: 1
---
# Adversary pass 2 — `story:cli-surface-fences-assert-what-they-claim`

Worktree `wt-d6c9d0f84db6`, over base `7b2d680`. `adp:adversary`.
191,801 tokens, 73 tool uses, 1,013 s.

## Header, as returned

```
verdict: red
cases: executed 45->48, red 3
origin: introduced 6, pre-existing 0, undecided 0
wrote-outside-worktree: 7 paths (all under the assigned scratch root)
needs-coordinator: yes
```

## The one that blocks a merge

`crates/connectors-cli/tests/cli_surface.rs:1698` is tracked and opens
`crates/connectors-cli/tests/adversary_fence_probe.rs` with a `read` that panics when the file is
absent. That file is untracked. `cargo test -p connectors-cli` is green in this working tree and
**red in any clone of the branch that merges it**, panicking in
`every_declaration_the_adversary_probe_copies_is_still_a_copy` before it compares anything.

## The documented flow the derivation does not see

`docs/design/19-the-cli-surface.md:115` says the twelve non-derived paths, `connect` among them,
send no protocol request and that the tree says nothing about them. The adversary walked
`Command::Connect` → `connect::dispatch` → `LocalClient` → the requests those bodies build,
asserting each hop so a rename fails loudly rather than emptying the case:

```
connectors connect reaches 7 protocol requests through connect::dispatch and LocalClient
  activate_candidate      CandidateActivate, CandidateSearch
  begin_connect_session   ConnectSessionCreate
  finish_connect_session  ConnectSessionStatus, Describe
  materialize_admitted    Materialize
  observations            ObservationSearch
3 carry the naming.wire of a command connectors-service accepts — the derivation's own
definition of Forwarded
```

The derivation reads the dispatch arm and not its callee, so a verb whose arm delegates to
`connectors-console` is invisible to it.

## The unit's own edit, run backwards

`docs/design/19-the-cli-surface.md:111` says the twelve tree-derived kinds "rest on no sentence at
all". The adversary removed `` `CandidateSearch`, `` from the read-verb sentence **in memory only** —
the exact edit this unit's diff made — and re-ran the extraction. `connection candidates` derives
`Read` from the shipped file and `Unmodelled` from the edited one. Eight of the twelve rest on a
prose enumeration inside a YAML comment.

## What held

- All 12 derived kinds re-derived by hand from `lib.rs:672-869`: 12/12 agree with the shipped kinds,
  and none agrees for a wrong reason.
- The read-verb enumeration is true: `CandidateSearch` and `SessionReconcile` are genuine reads, and
  the read/wire partition covers every variant of all three enums exactly once, `SessionSignal`
  excepted and correctly `Unmodelled`.
- Laundering a kind by adding a verb to the enumeration is refused — `operation signal` → `Read`
  leaves `Unmodelled` unused and `every_kind_of_exception_is_used_…` catches it.
- Renaming a leaf with `#[command(name = …)]` is refused.
- The 14 re-synced declarations in the pass-1 probe are verbatim, verified independently.
- `ess` 0.18.0 regenerates the committed tree byte-identically, `diff -ru` exit 0.

## Findings, verbatim

```findings
- file: docs/design/19-the-cli-surface.md
  line: 115
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the page says the twelve non-derived paths including `connect` send no protocol request, and `connectors connect` builds seven through `connect::dispatch` and `LocalClient`, three of them carrying the `naming.wire` of a command `connectors-service` accepts.
- file: docs/design/19-the-cli-surface.md
  line: 111
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the page says the twelve tree-derived kinds "rest on no sentence at all", and the eight `Read` ones are decided by a prose enumeration inside a YAML comment that this unit's own diff edited to change one of them.
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 1698
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the tracked contract suite opens the untracked `adversary_fence_probe.rs` with a panicking `read`, so `cargo test -p connectors-cli` is green in this working tree and red in any clone of the branch that merges it.
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 1117
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the derivation's group names and protocol prefixes are hand-kept literals, the exact shape `deployment_protocol_modules` was corrected away from in this same diff, and `crates/protocol/src` declares five request enums rather than three - unreachable today because no fourth group command exists.
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 1706
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the copy-control derives 14 shared declarations (all verbatim, independently verified) but guards on at least 11, so three copies may leave the comparison silently, and struct or pub fn declarations are invisible to its extractor entirely.
- file: docs/design/19-the-cli-surface.md
  line: 112
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: only `12 of the 26` is held to the derivation, and by a contains check, so the `2 more` grouping count and the named residual twelve can drift the moment the derivation's coverage changes.
```
