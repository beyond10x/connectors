---
format: aep.planning-md/3
id: story:test-binary-per-package
kind: story
status: active
title: Each package's integration tests build into one binary
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-11T19:51:14Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-11T19:51:15Z", actor: "human:timo", revision: 3}
---
## Finding

The workspace builds 314 test binaries; their run time sums to 2,801 s, 175 of them hold 5 tests
or fewer, and the 12 longest take 45% of the run time (load investigation, 2026-11-11, session
b10x-connectors). Each integration test file under `tests/` is its own binary that links the whole
crate graph again: adapters/catalog alone has 113 test files. Linking dominates the gate's CPU on
the shared workstation.

## Outcome

Each package's integration tests build into one binary (`tests/main.rs` declaring the files as
modules, `autotests = false`), starting with adapters/catalog, so a gate links a handful of test
binaries per package instead of one per file.

## Acceptance

- `cargo build --tests --timings` on the gate's packages, before and after, recorded in the story:
  number of test binaries and total build CPU time fall, with adapters/catalog at one binary.
- Every test that ran before still runs: the test names listed by `cargo test -- --list` before and
  after are the same set, ignored ones included, and connectors-build `ignored` classifies them the
  same.
