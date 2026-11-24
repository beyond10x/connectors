---
format: aep.planning-md/3
id: story:owner-and-providers-run-in-root-directory
kind: story
status: implemented
title: The owner and its providers run in / instead of the caller's working directory
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/tests/cold_owner_start.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/private-adapter.md
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/process.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T12:40:48Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-23T12:40:49Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-24T06:33:43Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Defect

The owner process and the provider processes it starts keep the working directory of the CLI
call that started the owner. They outlive that call (the owner ends after 10 idle minutes, and
its pool keeps an idle provider after a read), so the caller's directory stays in use by
processes the caller does not know.

Reported 2026-11-23 about 13:55 CET by the release-review consumer (dev-operator), on the
workstation's main install 0.65.2: the owner (pid 4008330, up 27 min) and two providers (jira,
up 25 min; slack-search, up 12 min) had as working directory the worktree of a release-review
run that ended at 13:23 CET, and `worktree finish` could not remove that tree.

Read on main 2e5ddabe9d8, not reproduced by a run:

- `crates/connectors-host/src/local/owner/transport.rs:1660-1668`: the owner's `Command` gets
  its arguments, a cleared environment and null standard streams. No `current_dir`. Its
  `pre_exec` hook (`:1698`) calls `setsid` and `dup2`, no `chdir`.
- `crates/connectors-host/src/local/runtime/process.rs:406-447`: a provider is a direct
  `command.spawn()` by the owner, no shell, no second fork, no `current_dir`; its hook (`:427`)
  has no `chdir`. A provider's parent is the owner.
- `rg -n 'current_dir|set_current_dir|chdir' crates/connectors-host/src apps/connectors/src`
  finds nothing outside tests.

Two other spawns do not outlive the call and are not part of the defect: the consumer launch
(`local/runtime/launch.rs:416`, the CLI waits for it and it keeps the caller's standard
streams) and the provider's `--print-*` and `--version` calls (`local/config.rs:2224`, the CLI
waits up to 10 s).

## Outcome

On Unix the owner and every provider process it starts run in `/`, whatever directory the CLI
was called from.

## Design

- `transport.rs`: `command.current_dir("/")` on the owner's `Command` (Linux and macOS share
  that code).
- `runtime/process.rs`: `current_dir("/")` on the provider's `Command` on Unix, so a provider
  does not depend on its parent's directory.
- std runs `chdir` before the `pre_exec` closures (1.99.1,
  `library/std/src/sys/process/unix/unix.rs:342` and `:388`); `/proc/self/fd/<n>` is absolute,
  so the order does not change what is executed.
- Not changed: the consumer launch and `Executable::print` (`config.rs:2224`). A relative path
  in a provider's `executable.args` therefore resolves against `/` in the provider the owner
  starts, and against the caller's directory in the CLI's own `--print-*` calls, as before.
  `executable.path` must be absolute (`config.rs:2177-2190`); no relative argument was found in
  the repository's tests, fixtures and documents.

## Acceptance

1. A test in `apps/connectors/tests/cold_owner_start.rs` (no new test binary), Linux only
   (`#[cfg(target_os = "linux")]`). It creates `D` with `container.jail.directory("/work/D",
   0o700)` and runs the CLI with a second `pre_exec` closure, added after `jail.command(...)`,
   that calls `chdir` on `/work/D` (the jail's own hook ends with `chroot` and `chdir("/")`,
   `crates/connectors-custody/src/mounted/fixture.rs:279-280`, and closures run in the order
   added; `Command::current_dir` would be overridden by that hook). The call is a read that
   starts an owner and a provider. While both run:
   - the owner's `/proc/<pid>/cwd` link equals `container.path("/")` (the test reads it from
     outside the chroot);
   - at least one process whose parent is the owner (the field after the last `)` in
     `/proc/<pid>/stat`) has a command line holding `cold_owner_start_adapter_fixture` and
     `--connectors-private-fd`, and the `cwd` link of each such process equals
     `container.path("/")`. A pid that vanishes between the listing and the read is skipped.
   The helper that lists the owner's children is new, in `cold_owner_start.rs`. On main the
   test fails with the path of `D` in its message; that red run's output is in the report.
2. The same test: after `setup owner-stop`, the same read made from the jail's `/` gives the
   same exit code and the same parsed `result` as the call made from `D` (both calls cold).
3. `contracts/cli/v1alpha1/owner.md`, in its text on how the CLI starts the owner, says the
   owner runs in `/` on Unix. `contracts/cli/v1alpha1/private-adapter.md` says a Unix adapter
   starts in `/`, beside its Windows "Current directory" bullet (`:263-266`).
4. `CHANGELOG.md` has an entry under `## Unreleased`, `### Fixed` (the headings are added when
   absent): the defect, `/`, and the two halves of the Design's fourth point.
5. `cargo nextest run -p connectors -E 'binary(cold_owner_start)'` exits 0 with no test over
   60 s and ends within 120 s; the new test's time is in the report. `cargo clippy -p
   connectors-host -p connectors --all-targets -- -D warnings` exits 0.

## Out of scope

- Windows: `CreateProcessW` in `crates/connectors-host/src/local/owner/transport/windows.rs:521`
  passes null as its 8th argument (`lpCurrentDirectory`), so the owner inherits the caller's
  directory there too; Windows providers already start in their image copy's directory
  (`artifact/windows.rs:554`, `:631`). Not changed here: no consumer runs it, and the Windows
  host job is red at its step 10 (story:windows-mcp-tool-call-answers-timeout). It follows
  that story.
- The directory of the consumer launch and of `Executable::print`.
- The main install of the workstation (0.65.2) is not upgraded by this story.

## Review

review-result:owner-cwd-acceptance-round-1 (needs-revision, 3 blockers, 5 warnings) is answered
in this revision: the jail's `chdir`, the link value seen from outside the chroot, the child
listing, `launch.rs` dropped, `print` named as unchanged, the parsed result, the contract
files and the changelog heading, the 120 s bound. No second round was run: every finding was a
correction of a single fact with the fix given.
