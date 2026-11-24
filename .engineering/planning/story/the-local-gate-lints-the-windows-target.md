---
format: aep.planning-md/3
id: story:the-local-gate-lints-the-windows-target
kind: story
status: active
title: The local gate lints the host and the CLI for a Windows target
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:one-pinned-rust-toolchain-for-the-local-gate-and-the-forge
scope:
- confidence: cited
  path: .github/workflows/rust-gate.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: docs/development.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T05:19:08Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-24T07:27:52Z", actor: "human:timo", revision: 10}
---
## Defect

Pull request 290 (release 0.77.0) went red on the forge after a green local gate: the job "host
and CLI on windows-latest" failed its lint step on 2026-11-24 with two dead-code errors
(`Evidence::decodes` at `crates/connectors-host/src/local/approvals/proof.rs:336` and
`seam::set` at `crates/connectors-host/src/local/owner/mutation/execution.rs:819`). Both items
are test-only and their callers are compiled on Unix only. The job ran 27 min 32 s before it
answered, and the pull request's other checks had to run a second time.

The local gate lints for Linux only. Code under `#[cfg(windows)]`, and code that is dead only on
Windows, is first compiled on the forge. The folder rule "The local gate equals the forge" asks
that the fix of such a failure includes the change that makes the local gate catch it.

## Change

1. `crates/connectors-build/src/gate.rs` gets a command builder `windows_lint(root, temp,
   ess)` beside `cargo(...)` (`:30`). It returns the forge's Windows lint
   (`.github/workflows/windows-host.yml:52`) with two additions, `--offline` like the gate's
   other cargo steps (`:222-228`) and `--target x86_64-pc-windows-gnu`: `cargo clippy --locked
   --offline -p connectors-host -p connectors --all-targets --target x86_64-pc-windows-gnu --
   -D warnings`. `gate::run` executes it after the Linux lint and then
   prints `gate: windows lint holds; exit=0`, like its other steps. `run` stays one linear function.
2. A function `windows_lint_prerequisites(probe)` returns the names of what is missing: the
   Rust target `x86_64-pc-windows-gnu` of the active toolchain and the C compiler
   `x86_64-w64-mingw32-gcc` (the build scripts of `ring` and `libsqlite3-sys` compile C for
   the target). `probe` is a function argument, so a test passes its own. When the list is not
   empty the gate ends non-zero and prints each name. It does not skip the step.
3. `docs/development.md` names the two prerequisites.
4. The first Linux job of `.github/workflows/rust-gate.yml` (the one that runs the gate,
   `:99`) installs the target on the pinned toolchain and the compiler package before the
   gate. Without that the gate of change 2 ends non-zero on the forge.

## Acceptance

1. A worktree of commit 507b602a2 (the head that failed on the forge; it has no
   `rust-toolchain.toml`), linted with `cargo +1.98.1 clippy --locked --offline -p
   connectors-host -p connectors --all-targets --target x86_64-pc-windows-gnu -- -D warnings`,
   ends non-zero, and its output holds `decodes` is never used and `set` is never used (rustc's
   dead-code wording, each name in backticks).
2. A worktree of commit 60bd87b6b (the fix), linted with the same `cargo +1.98.1` command,
   ends 0. Measured on 2026-11-24 05:21 CET on the build server with rustc 1.99.1: exit 0,
   `Finished` after 1 min 09 s.
3. The wave report names the wall time of the step's second run in one tree (`Finished`
   line). Over 120 s, it names the cut taken (one package, or no `--all-targets`) and its time.
4. A test in the `tests` module of `gate.rs` (beside `:1070`) reads
   `.github/workflows/windows-host.yml`, takes its `cargo clippy` line, and compares it with
   the arguments of `windows_lint(...)` from `clippy` on (the `--config` pair that `cargo(...)`
   puts first, `gate.rs:40-44`, is left out): the builder's arguments with `--offline`,
   `--target` and `x86_64-pc-windows-gnu` removed equal the workflow line's. It fails when the
   workflow's line gains or loses a package or a flag.
5. Two tests of `windows_lint_prerequisites`: with a probe that reports no Rust target the
   list holds `x86_64-pc-windows-gnu`; with one that reports no C compiler it holds
   `x86_64-w64-mingw32-gcc`. A third test asserts that the gate's error for a non-empty list
   holds each name.
6. `rg -n 'x86_64-pc-windows-gnu' docs/development.md` and `rg -n 'x86_64-w64-mingw32-gcc'
   docs/development.md` each find a line.
7. In the forge run of the pull request the check "repository gate" passes, and its log
   holds the line `gate: windows lint holds; exit=0`. The wave report names the minutes the
   job gained against the last run of `main` before it (package install, target, the cold
   build of the Windows dependencies).
8. A Linux-only test named `the_host_probe_finds_what_the_windows_lint_needs` calls
   `windows_lint_prerequisites` with the gate's real probe and asserts an empty list. It
   passes as a pool job on the build server (`cargo nextest run --locked -p connectors-build
   -E 'test(the_host_probe_finds_what_the_windows_lint_needs)'`) and in the forge's gate
   job. The whole gate is not started on the shared machine (the forge's gate job took
   39 min for pull request 290); statement 7 shows its run. A missing prerequisite is shown
   with test doubles only (statement 5).
9. Each new test is timed in the wave report; none is over 5 s.

## Limits

- `x86_64-pc-windows-gnu` is not the forge's `x86_64-pc-windows-msvc`. rustc evaluates `cfg`
  the same on both, so dead code and `cfg(windows)` type errors show on both. The tree's only
  `target_env` conditions (`transport.rs:1739`, `:4931`) name Linux. Clippy does not link: a
  link error or an error of the MSVC C runtime still shows on the forge's job only. That job
  stays the check of the build and the tests.
- macOS gets no such step here: the workstation has no C compiler for `x86_64-apple-darwin`.

## Order

- After `story:one-pinned-rust-toolchain-for-the-local-gate-and-the-forge`: both change
  `rust-gate.yml` and `docs/development.md`, and the Windows target is installed on the
  pinned toolchain, not on `stable`.
- Shares `gate.rs` and `rust-gate.yml` with `story:definitions-pin-covers-catalog` (draft) and
  `story:gate-runs-a-dependency-advisory-check`; not built in one wave with them.
