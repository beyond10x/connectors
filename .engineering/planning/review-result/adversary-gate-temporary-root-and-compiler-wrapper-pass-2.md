---
format: aep.planning-md/3
id: review-result:adversary-gate-temporary-root-and-compiler-wrapper-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: gate-temporary-root-and-compiler-wrapper'
relations:
- reviews: story:gate-temporary-root-and-compiler-wrapper
revision: 1
---
unit: story:gate-temporary-root-and-compiler-wrapper, commits 7f3cc6b31 + 903a5dfc1 plus my uncommitted test module, worktree wave1001c-gate
verdict: INFEASIBLE
cases: executed 135→140, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 tree (<scratch>/p2) plus 2 logs (<scratch>/pass2-red.log, <scratch>/pass2-suite.log)
needs-coordinator: none

Cases in `#[cfg(test)] mod adversary_pass2_tests` (gate.rs:543), all green: exit codes 2, 101, 255 preserved; signals
9, 6, 11 reported as signals; test arguments byte for byte (--exact, space, newline, -, --, empty, $HOME with
backticks and *, --test-threads=1); awkward binary paths (-dash, --, newline, space); roots with a leading dash,
newline or tab. Suite: `cargo test -p connectors-build --no-fail-fast` EXIT=0, 140 executed; fmt exit 0.

Not broken: the gate's --config through real cargo on 1.98.1, 1.88.0, 1.91.0 (panic exit 101, SIGKILL reported,
exit code 7 preserved); doctests get the root on 1.98.1 and 1.91.0, not 1.88.0 (documented); same host key on all
toolchains; explicit --target equal to the host works; decoy CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=false
outranked; dotted triple key parses; pass-1 assertions not weakened by the helper change. dash not installed, so
the POSIX sh -c path under dash (CI's /bin/sh) is untested.

```findings
- file: crates/connectors-build/src/gate.rs
  line: 42
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: keying the runner to the host triple drops it when a non-host build target is configured (CARGO_BUILD_TARGET or build.target), so tests silently get the caller's TMPDIR where the cfg(all()) form applied; nothing the gate or CI runs sets such a target
```
