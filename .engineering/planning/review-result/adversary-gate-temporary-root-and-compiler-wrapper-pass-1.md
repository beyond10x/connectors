---
format: aep.planning-md/3
id: review-result:adversary-gate-temporary-root-and-compiler-wrapper-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: gate-temporary-root-and-compiler-wrapper'
relations:
- reviews: story:gate-temporary-root-and-compiler-wrapper
revision: 1
---
unit: story:gate-temporary-root-and-compiler-wrapper, commit 7f3cc6b31 plus my uncommitted test module in worktree wave1001c-gate
verdict: CONFIRMED
cases: executed 131→134, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 tree (<scratch>/adv, plus <scratch>/t), all inside the assigned scratch
needs-coordinator: no

Cases in a new `#[cfg(test)] mod adversary_tests` at the end of crates/connectors-build/src/gate.rs (:363):
- runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign (:409): red — a failing test binary at …/build=1/debug/deps/fails reported success through the gate runner (status ExitStatus(unix_wait_status(0)), 135 bytes of stdout instead of running it)
- runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign (:429): red — left: "" right: "/checkout/.local/tmp/gate-AbCdEf\n"
- runner_preserves_awkward_roots (:447): green (space, ", ', \, =, $`;|&, é)
Probe with real cargo 1.98.1: CARGO_TARGET_DIR=…/tgt=eq and a panicking test — cargo test printed the environment instead of test results and exited 0.

Suite: `cargo test -p connectors-build --no-fail-fast` EXIT=101, 134 executed (132 passed, 2 failed); without the
cases EXIT=0, 131. fmt and clippy clean.

Not broken: every subcommand shape the gate uses accepts --config (fmt, clippy, metadata, tree, build, doc,
+1.88.0/+1.91.0 check); unit, integration and doctest binaries get the gate root on 1.98.1; no cargo call keeps
TMPDIR; ess and aep still get the gate TMPDIR; CI unchanged (1.98.1, no runner, no TMPDIR).

```findings
- file: crates/connectors-build/src/gate.rs
  line: 12
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the `env` runner reads a test-binary path containing `=` as an assignment, so it never runs the binary, prints the environment and exits 0, which turns failing tests green
- file: crates/connectors-build/src/gate.rs
  line: 18
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a host-triple runner from config or CARGO_TARGET_<TRIPLE>_RUNNER silently outranks the gate's cfg(all()) runner, so tests lose the gate TMPDIR without an error; passing --config target.<host-triple>.runner would outrank both
- file: crates/connectors-build/src/gate.rs
  line: 18
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: cargo 1.88 does not apply the runner to doctests (stable 1.98.1 does), and build scripts now get the caller's TMPDIR, which docs/development.md does not state
```
