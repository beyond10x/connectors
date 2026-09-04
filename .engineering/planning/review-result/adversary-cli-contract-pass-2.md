---
format: aep.planning-md/1
id: review-result:adversary-cli-contract-pass-2
kind: review-result
status: active
title: Adversary pass 2 against the CLI surface contract
relations:
- reviews: story:cli-surface-contract
revision: 1
---
# Adversary pass 2 — `story:cli-surface-contract`

Worktree `wt-8854cc8bb2ed`, uncommitted over `7f85149`. `adp:adversary`.
164,828 tokens, 57 tool uses, 1,033 s.

## Header, as returned

```
verdict: red
cases: executed 23->29, red 6
origin: introduced 11, pre-existing 0, undecided 0
wrote-outside-worktree: 1 path (an empty scratch directory)
needs-coordinator: no
```

## What the attack was

Pass 1 found the contract compared first-level words only. The correction rewrote every comparison
in the file. Pass 2 was told not to re-run that ground and to attack what the correction
introduced — chiefly the 26-entry exception list, now the contract's escape hatch, and the drift
suite's copied constants.

**The contract held. Its documentation did not.** Eight of eleven findings are prose that still
describes the pre-correction contract: the design document names a constant that does not exist,
describes the exception list as first-level words when 17 of its 26 entries are two- and three-word
paths, and says the test asserts a length it no longer asserts. The drift suite cites a 33-line
range for constants that are not in it and quotes four sentences that occur nowhere in the file
they are attributed to.

Two of those four quotations tell a reader the live contract still has the count assertion and the
leaves-only `carries_target` that pass 1 removed.

## The one property finding

`cli_surface.rs:322` documents the `--target` candidate set as "read from the tree", and it is a
three-element literal filtered by `is_file()`. `crates/protocol/src` declares **five** request
enums a deployment answers — `catalog`, `connection`, `datasource`, `event`, `operation`. A filter
can only shrink a literal, so a group named after `catalog` or `datasource` would never enter
`TARGET_EXCEPTIONS` and the countdown would reach zero without it. No such group exists today.

## What held

- The 26-entry list is **complete and exact**, forced in both directions. The adversary enumerated
  the parser by hand: 32 paths = 5 declared groups + `completions` + 26 exceptions.
- Every `Grouping` entry is the only kind with children; every `Forwarded` entry names a command
  `TARGET.md` lists as accepted by `connectors-service`; no `Read` entry names a declared command.
  No misclassification.
- Every `path:line` in `ess/system/components.yaml` checked for meaning rather than bounds. All
  exact.
- The new workspace `exclude` misses nothing: `production_sources` is only called under `crates/`,
  `json_governance` has no JSON to see, the other fences read the root manifest only.
- The flipped cutover case asserts what the amended design document now says.

## Findings, verbatim

```findings
- file: docs/design/19-the-cli-surface.md
  line: 65
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the accepted design document names a constant `UNSPECIFIED_WORDS` that no file of this repository declares; the contract's constant is `UNSPECIFIED_PATHS`
- file: docs/design/19-the-cli-surface.md
  line: 65
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the section bounding the escape hatch describes the exception list as first-level words that the parser gains, and 17 of its 26 entries are two- and three-word paths, so the accepted document still describes the contract the correction replaced
- file: docs/design/19-the-cli-surface.md
  line: 69
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the page says the test asserts `TARGET_EXCEPTIONS` has exactly three members, and the contract makes no assertion about its length at all
- file: crates/connectors-cli/tests/cli_surface_drift.rs
  line: 550
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: four sentences the drift suite attributes to `cli_surface.rs` occur nowhere in it, and two of them tell the reader the live contract has a count assertion and a leaves-only `carries_target` that it no longer has
- file: crates/connectors-cli/tests/cli_surface_drift.rs
  line: 13
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the header cites `cli_surface.rs:62-94` as where its two constants were copied from and that range declares neither, and the five ranges it calls the four assertions hold no assertion
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 322
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the countdown's candidate set is documented as read from the tree and is a three-element literal filtered only by `is_file()`, while `crates/protocol` declares five request enums a deployment answers, so the set can never grow and a `catalog` or `datasource` group would reach zero without entering it
- file: crates/connectors-cli/tests/cli_surface_drift.rs
  line: 42
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the control test is described as refusing a copy that has stopped being one, and `contract_refusals` reads only the path column, so the `Unspecified` kind and the reason string can diverge between the two files with nothing red
- file: crates/catalog-build/tests/main/architecture_fence.rs
  line: 33
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the comment justifying the 1006 to 1014 raise enumerates nine lines for an eight-line change, counting four lines of doc where `pub fn command()` carries three
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 315
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: one descendant carrying `--target` retires the whole group from the countdown, so a partial rollout would empty the list while a group's reads still infer the target, but the story puts the flag on the groups and nothing was found that reaches the partial state
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 579
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the generated-tree comparison is a flat word sequence and cannot see structure, so a tree nesting one declared group under another would pass, but while the component accepts no command the emitted tree can only be flat and nothing reaches it
- file: crates/connectors-cli/tests/cli_surface_drift.rs
  line: 659
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the case says the cited claims require that the emitted crate can be built and measures `cargo metadata --no-deps --offline`, which reads the manifest and compiles nothing
```
