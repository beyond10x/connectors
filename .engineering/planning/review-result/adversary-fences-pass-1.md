---
format: aep.planning-md/1
id: review-result:adversary-fences-pass-1
kind: review-result
status: active
title: Adversary pass 1 against the rewritten CLI surface fences
relations:
- reviews: story:cli-surface-fences-assert-what-they-claim
revision: 1
---
# Adversary pass 1 — `story:cli-surface-fences-assert-what-they-claim`

Worktree `wt-d6c9d0f84db6`, uncommitted over `7b2d680`. `adp:adversary`.
214,285 tokens, 85 tool uses, 1,246 s.

## Header, as returned

```
verdict: red
cases: executed 1252->1261, red 6
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: 7 paths (all under the assigned scratch root)
needs-coordinator: no
```

## Method

The brief told the adversary how the previous two passes over this code were beaten — by reading
assertions and judging them sound — and prescribed construction instead. It carried
character-for-character copies of the contract's `Unspecified`, `UNSPECIFIED_PATHS`,
`exception_list_refusals`, `parser_paths` and six more helpers into its probe, so the kind cases run
the shipped code rather than a paraphrase, with a control test proving the copies are still copies.

## What it found

**The escape hatch is narrower than the unit claimed, and the narrowing is in a different place.**
Relabelling each of the 19 non-`Lifecycle` entries to `Lifecycle` with a neutral reason is refused
for **2** of them — the two `Grouping` ones, caught by the subcommand rule. The other 17 pass. The
`Read | Lifecycle | Flow | Unmodelled` arm refuses only when the entry's own reason string
volunteers a `connectors.<domain>.<Command>` token, and that string is written by whoever wants the
entry.

The unit's own refused `Forwarded` construction for `connection materialize` raises `[]` once the
kind column alone is changed.

**A third line-shift citation break in one day.** The diff inserted about 192 lines above
`ess_claim_fence.rs:156`, and two documents cite that line as what enforces the `naming.wire` rule.
It was correct at base — `git show 7b2d680:… | sed -n 156p` is that function's signature. The unit's
own `every_citation_this_unit_wrote_resolves` passes it because 156 is inside the file, and says in
its own comment that it does not check the line means what the sentence says.

**A contradiction between the code and both documents.** `cli_surface.rs:961` lowercases the
qualified name's last segment; both documents say the typed word is that segment "verbatim and
un-cased" and list the eleven CamelCase words it produces. The set holds `supervisechannel` and none
of the eleven, so the `Unmodelled` contradiction is inert for every wire-less command.

## What held

- Every assertion in the five fence files swept for the guard-clause class: **none remain**. The
  base's four are gone, and the two inverted ones are confirmed inverted at base and correctly
  directed now.
- All 18 new `UNMAPPED:` event markers check out; all 20 event names have zero occurrences under
  `crates/`, as the markers claim.
- Every counted claim in the design document: 16 accepted commands, 16 target refusals, 11 of 16
  carrying no `naming.wire`, 5 request-enum modules, 26 entries / 9 / 17. All correct.
- The `ess` pin 0.16.0 → 0.18.0: the committed tree regenerates byte-identically, `diff -ru` exit 0.
- The unit's four lane counts (37 / 83 / 1132 / 75) verified independently.
- Adding a command costs 4 files beyond the parser, three of them enforced. That narrowing is real.
  What is free is the kind.

## Findings, verbatim

```findings
- file: docs/design/19-the-cli-surface.md
  line: 57
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the page cites ess_claim_fence.rs:156 as what enforces the naming.wire rule and this diff moved that rule to lines 348-409, leaving the citation on a whitespace-splitting helper inside fn prose
- file: ess/system/components.yaml
  line: 112
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the specification carries the same citation to ess_claim_fence.rs:156 and it lands on the same unrelated helper line, which is the exact class the unit added every_citation_of_the_specification_resolves to close
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 1085
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an entry's kind is a claim the tree can contradict for 2 of the 19 non-Lifecycle entries, because the Read, Lifecycle, Flow and Unmodelled arms refuse only when the entry's own reason string volunteers a declared command name
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 961
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: words_the_specification_can_type lowercases the qualified name's last segment while both documents state the typed word is that segment verbatim and un-cased and name the eleven CamelCase words it produces, so the Unmodelled contradiction is inert for every wire-less command
- file: crates/connectors-console/tests/adversary_readability_pass2.rs
  line: 140
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the case quotes "so that the last column starts on screen" from output.rs:30 and this diff rewrote that header, so the sentence is nowhere in the file and the cited line now says something else
- file: ess/system/components.yaml
  line: 126
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unspecified-path enumeration is a YAML comment that ess specify validate never reads, and the kind and reason a reviewer would need are not in it — they remain in a test file
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 704
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: every_citation_this_unit_wrote_resolves filters to .rs and .yaml while its sibling in ess_citation_fence.rs includes .md, so a .md line citation in the design document would be read by nothing — no such citation exists today
```
