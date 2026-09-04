---
format: aep.planning-md/1
id: review-result:adversary-cli-contract-pass-1
kind: review-result
status: active
title: Adversary pass 1 against the CLI surface contract
relations:
- reviews: story:cli-surface-contract
revision: 1
---
# Adversary pass 1 — `story:cli-surface-contract`

Worktree `wt-8854cc8bb2ed`, uncommitted work over base `7f85149`. `adp:adversary`.
138,739 tokens, 49 tool uses, 832 s.

## Header, as returned

```
verdict: red
cases: executed 11->20, red 8
origin: introduced 12, pre-existing 0, undecided 0
wrote-outside-worktree: 6 paths (all under the assigned scratch root)
needs-coordinator: yes
```

## What the attack was

The coordinator's brief named the suspicion: `connectors-cli` accepts no commands, so the generated
tree carries none, and a tree with no commands cannot drift from a parser with 16. The adversary
was told to construct the drift the contract claims to catch and see whether anything went red.

It wrote `crates/connectors-cli/tests/cli_surface_drift.rs`, 9 cases, restating the five assertions
of `cli_surface.rs` over a parser and a tree passed as arguments — because those assertions live
inside `#[test]` fns of another binary and cannot be called. Eight went red.

**The contract compares first-level words only.** `connectors connection harvest`, added to the
parser and to nothing else, raises no refusal. Neither does `connectors event invoke`, an operation
verb under the event group. The story's `## Acceptance` says the test "fails when a command is added
to the parser and not the specification", and its `## Defect` says nothing catches "a command added
in the wrong place". The parser already carries 14 subcommands under those 5 groups.

The generator was ruled out as the variable: regeneration against the unmodified specification is
byte-identical twice, and a one-sentence `summary:` edit moves only the `.about(...)` line.

## Findings, verbatim

```findings
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 163
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the contract compares first-level words only, so a subcommand added under a declared group or added under the wrong group raises no refusal, which is what the story's Acceptance and Defect sections both say it catches
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 302
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a stale committed tree is not caught by the crate suite because the only tree assertions are group-name presence and a word count, and a one-word summary edit in the specification moves nothing else
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 260
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the target countdown reads only the leaves' arguments, so a --target declared on the connection group itself is invisible, and that is where the next story's flag naturally goes
- file: docs/design/19-the-cli-surface.md
  line: 98
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the accepted one-group-at-a-time cutover cannot keep the gate green because the emitted group carries no subcommand and cli_surface.rs refuses a declared group whose parser word carries none
- file: ess/system/components.yaml
  line: 86
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the citation architecture_fence.rs:303 points at a ConnectorBackend assertion inside focused_adapters_are_focused, not at product_cli_is_a_thin_frontend which spans lines 310 to 380
- file: ess/generated/clap/crates/connectors-cli/Cargo.toml
  line: 7
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the emitted manifest sits inside the root workspace directory and is in neither members nor exclude, so every cargo command against it is refused before it is read
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 301
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a tree that swaps completions for any other word keeps the count at seven and passes, but I could show no automated path that produces such a tree
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 53
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the allowlist header claims every entry is a read or a process-lifecycle verb, and init and connect are neither by their own reason strings in the same list
- file: crates/connectors-cli/tests/cli_surface.rs
  line: 243
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the countdown's size assertion compares a literal to a constant twelve lines above it in the same file, so it measures the test file rather than the parser
- file: ess/generated/clap/crates/connectors-cli/src/main.rs
  line: 22
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the emitted binary writes the completion script straight to stdout and panics under a pipe that closes early, which is the exact failure the hand-written lib.rs buffers to avoid, and the emitter is ess rather than this repository
- file: scripts/gate.sh
  line: 75
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the byte-identity diff runs only under --final, which only the release workflow invokes, so a stale generated tree reaches main and is first refused at a release cut
- file: ess/generated/clap/crates/connectors-cli/src/tree.rs
  line: 15
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the emitted binary's about text is the component summary rather than the product's and the emitted tree carries no global output flag, both of which a cutover would ship into connectors --help unnoticed
```
