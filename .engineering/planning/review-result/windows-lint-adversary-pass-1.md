---
format: aep.planning-md/3
id: review-result:windows-lint-adversary-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: the local gate lints the Windows target'
relations:
- reviews: story:the-local-gate-lints-the-windows-target
revision: 1
---
```
unit: story:the-local-gate-lints-the-windows-target, unstaged working tree over faa134f0d in the worktree `windows-lint`
verdict: NEEDS-CHANGE (test gaps; nothing in the production code went red)
cases: executed 346→351 (package), gate module 34→39, red 0 of mine; 1 red reader test, unchanged against HEAD
origin: introduced 4 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: whether F1 blocks acceptance 7; clippy on my module
```

**Brief change of 08:31 CET:** no cargo command ran from under `~/.cache`, and there was no scratch-copy build, so no slot job id to name. I did not edit production lines of `gate.rs` either: the role file forbids it, so each mutant is expressed inside a test instead.

**1. `git --no-pager diff --stat`** (five paths were dirty when I started; I added only to `gate.rs`, appended test module at lines 1623-1869):
```
 .engineering/planning/story/the-local-gate-lints-the-windows-target.md | 14 +-
 .github/workflows/rust-gate.yml     |   9 +
 CHANGELOG.md                        |  12 +
 crates/connectors-build/src/gate.rs | 452 ++++++++-
 docs/development.md                 |  38 +-
```

**2. Cases added** — module `gate::windows_lint_adversary_tests`, all green now. They catch mutants, so they were never red on this tree; first run alone 08:36-08:37 CET, slot job 4094, 5 passed.
- `run_checks_the_prerequisites_first_and_executes_the_windows_lint` reads the body of `run` and asserts the place and the `?` of both new lines.
- `each_mutant_of_run_that_loses_the_windows_lint_is_refused` applies six mutants to that body; the check refuses each.
- `the_host_probe_misses_a_target_rustc_has_no_core_of` plants a `rustc` on `PATH` in a child process; the probe says no for an absent, empty or mingw-only directory and yes with `libcore-*`.
- `a_probe_without_the_library_check_finds_a_target_that_is_not_installed` is the control: the mutant twin answers yes under the same fixture.
- `the_error_for_missing_prerequisites_is_the_line_the_guide_quotes` compares the full error text with the guide's quote and heading.

**3. Suite run after the cases** (08:39 CET): `cargo nextest run --locked -p connectors-build -E 'test(/^gate::/)'` → `39 tests run: 39 passed, 312 skipped`, exit 0, 3.5 s. The run before it added the reader test of F1: `40 tests run: 39 passed, 1 failed`, exit 100.

**4. Findings**

| id | severity | file:line | verdict / origin | what, evidence, smallest fix |
|---|---|---|---|---|
| F1 | warning | `crates/connectors-build/tests/cli_spec_mapping_adversary.rs:192` | CONFIRMED / pre-existing | `the_er_rs_citation_lands_on_the_cursor_expiry_path` is red: `ess/domains/cli.yaml:2206` cites `er.rs:5015-5017`, which now reads `AttemptRecord`; the cursor path is at `er.rs:4995`. Both files are unmodified against HEAD, and the implementor's `readers.log` shows the same failure. The gate's test step runs this binary, so acceptance 7 cannot pass until it is fixed. Fix: correct the citation, in its own change. |
| F2 | warning | `crates/connectors-build/src/gate.rs:320` | NEEDS-CHANGE / introduced | None of the six tests fails when `run` drops the lint line, discards its result, puts it under `if !msrv`, or drops or moves the prerequisite check (`:200`). With the lint line dropped, `gate: windows lint holds; exit=0` still prints, and that line is what acceptance 7 reads. Fix: keep my two source tests, or move the step into a function that takes `execute`, like `test_step`. |
| F3 | warning | `crates/connectors-build/src/gate.rs:122` | NEEDS-CHANGE / introduced | `the_host_probe_finds_what_the_windows_lint_needs` stays green when the `libcore-` check is removed. `rustc --print target-libdir --target aarch64-pc-windows-msvc` (not installed) exits 0 and names a missing directory. Reached by any host without the target: the gate would fail at the lint instead of at its start. Fix: keep my two probe tests. |
| F4 | note | `crates/connectors-build/src/gate.rs:107` | INFEASIBLE / introduced | The probe runs `x86_64-w64-mingw32-gcc`, but the `cc` crate first reads `CC_x86_64_pc_windows_gnu`, `TARGET_CC` and `CC`. A host with `CC` set can pass the probe and fail the lint, or the reverse. Read from `cc`'s documentation, not run; no host found that sets one (this workstation and the forge runner do not). Fix: one sentence in `docs/development.md`. |
| F5 | note | `crates/connectors-build/src/gate.rs:200` | INFEASIBLE / introduced | On this workstation the documented `cargo run --locked -p connectors-build -- gate` (`docs/development.md:199`) runs the gate here, while the shim sends its `cargo clippy` to build-1d. The probe then answers for a different host than the lint runs on. Both hosts have both prerequisites today. Fix: none needed now. |

Attack points 1, 4, 5 and 6 gave no finding.

**5. Attacked and not broken**
- Point 1: `run` is linear; every early exit is an `Err`; `main.rs:408-410` is its only caller; `msrv` only adds steps after the lint.
- Point 2: `RUSTUP_TOOLCHAIN` and a `+toolchain` reach the probe's `rustc` and the gate's `cargo` alike; both use `current_dir(root)`.
- Point 3: target dropped, `-D warnings` dropped, `--offline` dropped and a compiler probe that always says yes are all killed by the existing six.
- Point 4: only the "repository gate" job runs this package's tests on Linux; `journeys` runs `ignored` only; the macOS and Windows jobs test other packages.
- Point 5: `rustup target add` runs in the checkout after `rustup toolchain install`, so it lands on 1.98.1 (same order as `pages.yml:50-52`); both new commands sit under `steps.scope.outputs.run != 'false'`.
- Point 6: every claim in the guide and the changelog matches the code; `mingw-w64-gcc` is the Arch package (`pacman -Qo`).

**Not checked:** the gate itself; acceptance 1, 2 and 7; clippy on my module (rustfmt on a scratch copy is clean); the forge job's added minutes and disk; whether `gcc-mingw-w64-x86-64` with `--no-install-recommends` brings the headers; the lint with `CC` set; a mutant that drops `current_dir(root)` from the probe.

**6. Paths written outside the worktree**, all in `<session scratch>/winlint/adversary/`: `cases-alone.log`, `suite.log`, `cases-final.log`, `fmt-probe.log`, `gate-fmt-probe.rs`. My worktree lease `adversary-winlint-4fb1ea8b` is released.

```findings
- file: crates/connectors-build/tests/cli_spec_mapping_adversary.rs
  line: 192
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the_er_rs_citation_lands_on_the_cursor_expiry_path is red because cli.yaml:2206 cites er.rs:5015-5017, which no longer holds the cursor path, and the gate's test step runs it
- file: crates/connectors-build/src/gate.rs
  line: 320
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: none of the six tests fails when run drops, discards or conditions the Windows lint or its prerequisite check, and the holds line still prints
- file: crates/connectors-build/src/gate.rs
  line: 122
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the host probe test stays green when the libcore check is removed, although rustc names a directory for a target it has no libraries of
- file: crates/connectors-build/src/gate.rs
  line: 107
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the probe runs x86_64-w64-mingw32-gcc while the cc crate first reads CC_x86_64_pc_windows_gnu, TARGET_CC and CC, and no host that sets one was found
- file: crates/connectors-build/src/gate.rs
  line: 200
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: on a host whose cargo shim builds on another node the probe answers for the gate's host and the lint runs on the node, and both hosts have the prerequisites today
```
