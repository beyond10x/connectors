---
format: aep.planning-md/1
id: review-result:adversary-catalog-pipeline-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the bundle pipeline
relations:
- reviews: story:catalog-bundle-pipeline
revision: 1
---
# Adversary pass 1 — story:catalog-bundle-pipeline

Worktree `wt-35ad348b4293` at `143d958` plus the unit's uncommitted work. One new file,
`crates/connectors-catalog/tests/pipeline_adversarial.rs`, eight cases, two red and six kept green
so the negative results stay reproducible rather than asserted.

```
unit: 1
verdict: red
cases: executed 116→124, red 2
origin: introduced 3, pre-existing 2, undecided 0
wrote-outside-worktree: three paths under the assigned scratch root, each named for the adversary
needs-coordinator: yes
```

## What held

The empty-directory case the implementor flagged is not reachable through `run`: `valid_id` is
checked before `create_dir_all`, a fresh directory's index read cannot fail, and the bundle write
cannot fail without an obstruction inside a directory that therefore already existed. A provider
name cannot collide with `index.json`, because `valid_id` forbids a slash and the suffix is fixed.
Replacement against an index whose file is missing rewrites it. A directory source refuses at the
read step; a symlinked source records the link's own name. The bytes are read once and reused, so
nothing changes between steps. All five step labels match their serialised names. No error path
reaches a caller without a step.

Two things raised as prose rather than as findings: a FIFO source makes `run` block in `open` rather
than refuse, and no case was written because a hanging case in a shared suite is worse than the
finding; and a source whose name is not valid UTF-8 is refused at the read step with a message that
is false of it, which is base behaviour and too small to carry alone.

## Findings

```findings
- file: crates/connectors-catalog/src/pipeline.rs
  line: 182
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "A write-step refusal leaves a bundle file in the directory, falsifying the story's Scope sentence that a refusal leaves the directory exactly as it was; bundle::write writes the bundle before the index and the pipeline has no cleanup path."
- file: crates/connectors-catalog/src/bundle.rs
  line: 100
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "Two runs into one directory lose each other's index entries: eight concurrent runs all returned Ok and wrote their bundle, and the index named two of them, so six records carry an entry read_index does not report."
- file: crates/connectors-catalog/tests/pipeline.rs
  line: 101
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The three assertions comparing a record's steps against ORDER are tautological, because the steps are built from ORDER, so any reordering of the stated sequence stays green."
- file: crates/connectors-catalog/src/pipeline.rs
  line: 96
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Failure implements Display but not std::error::Error, so the boxed failure run returns cannot be used with the question mark operator into any standard error type."
- file: crates/connectors-catalog/src/pipeline.rs
  line: 171
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "The extract-step parse arm is unreachable because ingest already parsed the same bytes with the same function and target type, and it introduces a second wording for what ingest names Malformed."
- file: crates/connectors-catalog/src/bundle.rs
  line: 98
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "A request whose directory is the empty path silently targets the process working directory, verified at the standard library primitives only, with no caller of run existing to reach it."
```
