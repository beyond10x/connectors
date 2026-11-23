---
format: aep.planning-md/3
id: story:windows-mcp-tool-call-answers-timeout
kind: story
status: active
title: A tool call through the MCP stdio server answers on Windows
summary: On windows-latest tools/call answers timeout after 2 s; step 10 of the Windows host job is red
relations:
- blocks: story:mcp-stdio-on-macos-and-windows
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/src/mcp/projection.rs
- confidence: cited
  path: contracts/cli/v1alpha1/private-bounded-read.md
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/bounded.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/channel/windows.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/channel/windows/tests.rs
revision: 41
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T11:04:21Z", actor: "human:timo", revision: 40, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-11-23T11:04:22Z", actor: "human:timo", revision: 41, decided_on: {"recorded":{"review_outcome":6}}}
---
## Defect

On `windows-latest` a tool call through the MCP stdio server answers `timeout` about 2 s after
the test binary starts. `initialize` and `tools/list` of the same session answer.

Run 45148199646 (pull request 284, head 0fadfb1878f, 2026-11-23), job "host and CLI on
windows-latest", step 10 "The MCP stdio server serves a session on the platform's own
custody": `the_stdio_server_serves_a_session_and_answers_a_call` panics at
`apps/connectors/tests/mcp_stdio_serves.rs:512` with

```
{"error":{"code":-32000,"data":{"audit_ref":null,"audit_status":"unavailable","error":{"code":"timeout","message":"Connectors request refused"},"request_id":null,"status":"error","version":"v1alpha2"},"message":"Connectors request refused"},"id":3,"jsonrpc":"2.0"}
```

`test result: FAILED. 0 passed; 1 failed; ... finished in 2.16s`. The server's stderr is empty.
`connections connect` of the test's adapter passes in the same run. macOS and Linux pass the
step.

The step was skipped in every Windows run on `main` (an earlier step failed since run
43370169181 of 2026-11-13, and the last green run, 43357584659, has no MCP step), so a tool call
through the MCP stdio server has not been shown to work on Windows. No Windows binary is
released (`.github/workflows/release-binaries.yml:3`).

## Cause, read from code on 2026-11-23

Not yet shown by a run on Windows. `L` is `crates/connectors-host/src/local`. A story-scoper
read every platform gate on the path of one `tools/call`, in the order the call reaches them:

1. `apps/connectors/src/mcp/projection.rs:743` starts the read budget. `ticks()` under
   `cfg(not(unix))` returns `Failure::Unsupported` (`L/runtime/bounded.rs:97-102`), so
   `ReadBudget::start` (`bounded.rs:16`) fails, and `projection.rs:743-745` maps every
   failure of it to `refusal("timeout")`, before any owner call. `tools/list` builds no budget.
   The test sets `request_milliseconds` to 120000, so no deadline ends at 2 s.
2. The client's connect and greeting have Windows forms (`L/owner/transport.rs:302`, `:1427`).
3. The owner's `exchange_cancellable` reads `budget.until()` (`transport.rs:2435`), which calls
   `ticks()` again.
4. `watch_cancel` under `cfg(not(unix))` answers `Code::Unsupported`
   (`transport.rs:2527-2536`). The owner then writes no reply (`:2438`, `:2219-2224`), the
   client reads end of file (`L/runtime/channel.rs:218-220`) and answers `unavailable`
   (`projection.rs:95`). With `ticks()` alone fixed, the answer is `unavailable`.
5. `supervisor::Cancel::cancel` is `#[cfg(unix)]` (`L/owner/supervisor.rs:238`): a Windows
   `watch_cancel` that calls it does not compile until that gate goes.
6. The adapter invoke (`L/runtime/process.rs:895-956`), the child (`server.rs:143-154`) and
   `bounded.rs:159` use the channel and `ticks()` and have no other gate. The child is killed
   on deadline by `process.kill()` plus wait on Windows, under a kill-on-close job
   (`process.rs:1151-1197`).

`projection.rs:745` is the only catch-all that answers `timeout`; `projection.rs:100`, `:120`
and `stdio.rs:146` answer it for real deadlines. `unsupported` is a declared code
(`ess/domains/service_wire.yaml:13`, `:19`) that the projection already emits
(`projection.rs:92`, `:174`, `:183`).

The six points are the gates found by reading. Past them the owner's path of a read
(`governed::bounded::resolve`, the registry, the credential read from the platform store) has
no platform gate and no test that runs on Windows, so it has never run there. The first green
run of the step is the proof; a run may show another defect after these.

Reverses D1 of `approval-record:wave-20261022` (2026-10-23), "the bounded read is unix-only
by design", with the test `bounded::unsupported_tests::a_budget_cannot_start_off_unix` and the
contract sentence "Unix only, a budget elsewhere refuses as unsupported"
(`contracts/cli/v1alpha1/private-bounded-read.md:16-18`). story:mcp-stdio-on-macos-and-windows
(active, later) requires the opposite: "The macOS and Windows CI jobs run the MCP stdio tests
that run on Linux today"; this story follows the later one, and host-operator has that
(2026-11-23, 11:44 CET). D1 held the bounded read
Unix-only because a budget is this boot's `CLOCK_MONOTONIC` nanoseconds
(`L/runtime/bounded.rs:97-98`). Windows has a clock of the same kind, the unbiased interrupt
time, which is system-wide, so owner, client and child read one clock
(`contracts/service/clock.md:138-143`); D1's reason does not hold. The record is not edited.
Nothing else relies on the read refusing on Windows: the only assertion of it is
`bounded::unsupported_tests` (`bounded.rs:271-280`). This story unblocks
story:mcp-stdio-on-macos-and-windows.

## Design

- `ticks()` gets a `cfg(windows)` arm: `QueryUnbiasedInterruptTimePrecise`, 100 ns units times
  100 with `checked_mul`. The read exists twice already, both private (`fn read` in the
  private `mod timer`, `L/clock/timer.rs:41-47`, `clock.rs:11`; `L/owner/mutation.rs:259-271`
  inside `impl Deadline`), so the new arm holds the call itself, a few lines. `clock.rs`,
  `clock/timer.rs` and `owner/mutation.rs` are not edited.
  A `cfg(not(any(unix, windows)))` arm keeps `Failure::Unsupported`.
- `runtime/channel/windows.rs` gains `Pipe::try_clone`: the standard library's
  `OwnedHandle::try_clone` (a `DuplicateHandle` with the same access, not inherited) and a new
  event from `CreateEventW`, since each `Pipe` holds one event (`windows.rs:69`, `:174`); `peer`
  stays empty in the clone. Each operation has its own `OVERLAPPED` on the stack and is
  cancelled with `CancelIoEx(handle, &overlapped)`, which names that one operation
  (`windows.rs:165-220`); the file has no `CancelIo` and no `FlushFileBuffers`. Both pipe ends
  are opened overlapped (`:458`, `:497`).
- `watch_cancel` (`transport.rs:2479-2536`) becomes one function for Unix and Windows. It
  already reads through `channel::read_with_cancel` (`transport.rs:2500`), whose Windows form
  is the same loop over `read_within` with the 250 ms check (`L/runtime/channel.rs:188-223`),
  so the body does not change: a thread reads frames on the clone and cancels on a `cancel`
  naming its request id, on any other frame and on a failed or closed stream; a `cancel`
  naming another request is ignored; `finished` ends the thread. The original handle writes
  the reply meanwhile (`:2462-2471`), and every path joins the thread, so the clone is not
  dropped with a read pending. Read from the Windows API's rules and not shown by a run: a
  duplicate shares the file object, an overlapped pipe allows one pending read and one pending
  write at once, and a cancelled `ReadFile` that has not completed loses no bytes. Acceptance
  3 and 4 show it. Only if a run refutes it does the watcher poll the clone with
  `PeekNamedPipe` (`windows.rs:372-395`).
- `supervisor::Cancel::cancel` loses `#[cfg(unix)]` (`supervisor.rs:238-240`, one `AtomicBool`
  store).
- `projection.rs:743-745` answers `unsupported` for `Failure::Unsupported` and keeps
  `timeout` for every other failure of the start, through one function. A new `#[cfg(test)]`
  module in `projection.rs` that is not `cfg(unix)` tests it; `mod tests` at `mod.rs:97` and
  at `projection.rs:863` are `cfg(all(test, unix))` and stay.
- `bounded::unsupported_tests` goes; `runtime::bounded::tests` lose their `unix` gate
  (`bounded.rs:216`). The owner transport's `governed_client_tests` (`transport.rs:3635`,
  `UnixStream::pair`) and the Linux-only MCP integration tests keep their gates.
- The contract sentence at `contracts/cli/v1alpha1/private-bounded-read.md:16-18` becomes:
  `deadline_ticks` is nanoseconds of `CLOCK_MONOTONIC` on Unix and of the unbiased interrupt
  time on Windows; a budget on any other platform refuses as unsupported. No page under
  `website/docs` holds a copy of the sentence (`rg -l private-bounded-read` finds it only
  under `contracts/`); `generate --check` of acceptance 8 decides whether a generated page
  changes. The contract file is not an input of
  `inputs.json`. The doc comments at `bounded.rs:97` and `transport.rs:2526` go with their
  gates.
- No file under `ess/` changes. The comment at `ess/domains/transport.yaml:18` ("this Linux
  boot's CLOCK_MONOTONIC") stays for now: `crates/connectors-build/generated/metadata-emissions/inputs.json:140`
  holds the file's digest, and a changed byte costs a regeneration of three parts of up to 30
  minutes each (`docs/development.md:66-68`). The comment is corrected by the next change that
  regenerates (the standing-grant story on branch `story/standing-grant-local` names it).

## Decision D6, 2026-11-23

host-operator, about 11:40 CET: "D6 = A: merge #284 as it is, ship 0.76.0 right after, fix
Windows step 10 as its own story, named in the release's 'known' list." No
`continue-on-error` on the step: Timo, about 11:35 CET, "we dont build workarounds here". This
story comes next after 0.76.0.

## How it is built

Only `windows-latest` runs the code, about 25 minutes per push. Before each push the change
compiles for Windows on the workstation (acceptance 1). Each push is one attempt; the report
lists the attempts with their run ids. The runner queue rule holds: one push per attempt, no
re-run of a commit that has a run queued.

## Acceptance

"The Windows job log" is the log of the job "host and CLI on windows-latest" at the head of
the pull request of branch `story/windows-bounded-read`.

1. On the workstation, at the story's head,
   `SLOT_TIMEOUT=30m cargo check --locked --target x86_64-pc-windows-gnu -p connectors-host -p connectors --all-targets`
   and
   `SLOT_TIMEOUT=30m cargo clippy --locked --target x86_64-pc-windows-gnu -p connectors-host -p connectors --all-targets -- -D warnings`
   exit 0. The first exits 0 on the base (3d38f77b99f) too; the report gives both results and
   the wall time. The Windows job compiles for `x86_64-pc-windows-msvc`; the report names that
   difference.
2. A new `#[cfg(test)]` module (not `cfg(unix)`) in `apps/connectors/src/mcp/projection.rs`
   calls the function that maps a failed budget start: `Failure::Unsupported` gives
   `unsupported`, `Failure::Unavailable` gives `timeout`. Before the change the function does
   not exist and the inline `map_err` gives `timeout` for both. It passes on Linux with
   `cargo nextest run -p connectors -E 'test(<its name>)'`, and the Windows job log has it
   with `ok`. The report names the function and the test.
3. A `cfg(windows)` test in `L/runtime/channel/windows/tests.rs`: one end of a pipe pair is
   cloned with `try_clone`. A thread reads on the clone through `channel::read_with_cancel`
   and signals just before its read. The original, on its own thread, then writes 256 KiB
   while the peer does not read (the pipe buffer is 64 KiB per direction, `windows.rs:61`), so
   the write pends. The closure passed to `read_with_cancel` counts its calls, and the test
   asserts a count of at least 2 before the peer starts reading; the peer then reads
   everything and the test checks that the write completed whole and the bytes are the ones
   written. Then the peer writes one frame
   and the clone's read returns it. Each wait is bounded at 10 s. The Windows job log has the
   test's name with `ok`.
4. `cfg(windows)` tests of `watch_cancel` beside it in `L/owner/transport.rs`, over a pair of
   their own from `runtime::channel::windows::pair()` and `Pipe::from_owned`, one per case:
   (a) the peer writes a `cancel` naming the request id, and the `Cancel` reads cancelled
   within 10 s; (b) the peer writes a `cancel` naming another id, and after 600 ms the
   `Cancel` does not read cancelled and the watcher's `JoinHandle::is_finished()` is false; (c) the peer closes its end, and the `Cancel` reads cancelled within
   10 s; (d) with an `until` 20 s away, `finished` is set with no frame sent, the thread joins
   within 1 s, and the
   `Cancel` does not read cancelled. The Windows job log has each test's name with `ok`.
5. The Windows job log has every test of `runtime::bounded::tests` with `ok`, among them a
   new one that runs on every platform: two `ticks()` calls with a 100 ms sleep between them
   differ by at least 100 ms and less than 5 s. `rg -n 'unsupported_tests' crates apps` prints
   nothing.
6. At the pull request's head the job "host and CLI on windows-latest" ends `success` and its
   step "The MCP stdio server serves a session on the platform's own custody" ends `success`.
   The same step stays `success` in "host and CLI on macos-latest". `gh pr checks <n>` lists
   no failed check.
7. A guard of the Unix behaviour, not expected to fail before the change: on Linux
   `cargo nextest run -p connectors-host -E 'test(runtime::bounded::) | test(owner::transport::)'`,
   `cargo nextest run -p connectors -E 'test(mcp::)'` and
   `cargo nextest run -p connectors --test adversary_mcp_deadline --test security_mcp_deadline --test mcp_tool_call_shapes`
   exit 0, each test within 60 s and each run within 120 s; a test over 5 s is named with its
   time, and a run over 120 s is named with its time and not repeated.
8. `rg -n -U 'unbiased\s+interrupt\s+time' contracts/cli/v1alpha1/private-bounded-read.md`
   prints at least one line, and `rg -n 'Unix only' contracts/cli/v1alpha1/private-bounded-read.md`
   (which prints line 17 today) prints nothing.
   `rg -n -i 'unix only|only a unix owner' contracts/cli/v1alpha1/private-bounded-read.md crates/connectors-host/src/local/runtime/bounded.rs crates/connectors-host/src/local/owner/transport.rs`
   prints nothing. `slot cargo run --locked -p connectors-docs -- generate --check` exits 0.
   `git diff --stat origin/main...HEAD -- ess crates/connectors-build/generated` prints
   nothing, and the pull request's "repository gate" check passes.
9. `CHANGELOG.md` has a `### Fixed` entry under `## Unreleased` (the heading is added above
   `## 0.76.0` if no merged story has added it): a tool call through the MCP stdio server on
   Windows answered `timeout`; it names the release that introduced the bounded read (found
   with `git log -S'fn ticks' -- crates/connectors-host/src/local/runtime/bounded.rs` and
   `git tag --contains`), and says that no Windows binary was released.
10. The report names the cancel form built (a read on the clone, or `PeekNamedPipe` polling),
    the commit, and the run id in which the tests of acceptance 3 and 4 and the step of
    acceptance 6 ended `ok` with it. If the second form was built, it names the run id in
    which the first form failed and the failing test.

## Shares files with

- story:settled-read-uses-leave-the-store (branch `story/read-uses-in-memory`): `CHANGELOG.md`
  only. Its entry also goes under `## Unreleased`; whichever story merges first creates the
  heading and the later one rebases onto it.
- story:connections-list-answers-through-the-owner (branch `connections-through-owner`):
  `CHANGELOG.md` and `crates/connectors-host/src/local/owner/transport.rs`. Its hunks there are
  `Client::observe` (about 1094-1113), one match arm (about 2810) and one `action_value` arm
  (about 3137-3143); this story changes `watch_cancel` (2479-2536), the tests beside it and
  the `use` lines those tests need.
- The standing-grant story (branch `story/standing-grant-local`, a draft without code): it
  changes `ess/domains/*.yaml` and regenerates `inputs.json`; this story changes neither.
  `owner/transport.rs` is not known to be in its scope; checked again when its scope is set.
- Merge order with the least rework: 0.76.0, story:settled-read-uses-leave-the-store,
  story:connections-list-answers-through-the-owner, then this story.

## Out of scope

- The Linux-only MCP integration tests (cancel, deadline, large results) on Windows and macOS:
  story:mcp-stdio-on-macos-and-windows.
- The sentences that say serving on macOS and Windows is "not yet shown" or has "passed on
  Linux only so far" (`docs/local-mcp-stdio.md:10-16`, `docs/local-mcp-cli.md:3`,
  `website/docs/connectors/mcp.md:65-68`, `website/docs/status.md:50`): they stay with
  story:mcp-stdio-on-macos-and-windows, which changes them on the recorded serving runs.
- A Windows release binary: story:windows-host-job-releases-the-cli-binary.
- A Windows compile check in the repository gate. The report gives the wall time of
  acceptance 1 so that it can be decided.
- The 20 owner processes the Windows test step leaves behind:
  story:test-children-die-with-their-parent.
