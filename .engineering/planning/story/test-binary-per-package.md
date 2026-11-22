---
format: aep.planning-md/3
id: story:test-binary-per-package
kind: story
status: active
title: Each package's integration tests build into one binary
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-11T19:51:14Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-11T19:51:15Z", actor: "human:operator", revision: 3}
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

## Measured, 2026-11-11 (wave/load-and-flakes, adapters/catalog only)

`cargo test -p connectors-catalog-provider --no-run` through the build slot (shared target, sccache
on, so times are indicative):

- before: 120 test executables, cargo Finished 2m 37s
- one `main` target: 3 executables (lib, bin, main), Finished 1m 48s and 1m 50s
- with derived_instance_adversary split out again (it sets CONNECTORS_INSTALLATION process-wide):
  4 executables, Finished 2m 01s

Test names from `-- --list --include-ignored`: 1205 before, the same 1205 after, 1208 with the
declared-module guard and the two adversary cases; 86 ignored tests, all classified in
crates/connectors-build/src/ignored.rs. Full non-ignored run on the one-target tree: main 1087
passed, 86 ignored; lib 17; bin 16; exit 0.

`cargo build --tests --timings` was not run: it builds the whole workspace, which the shared
workstation's no-load rule does not allow during a wave. The other packages (connectors-host,
apps/connectors, connectors-build and the rest) still build one binary per test file.
