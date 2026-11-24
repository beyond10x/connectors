---
format: aep.planning-md/3
id: review-result:windows-bounded-read-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:windows-mcp-tool-call-answers-timeout
relations:
- reviews: story:windows-mcp-tool-call-answers-timeout
revision: 1
---
Recorded by the session that ran the panel: the brief asked for the members severity, where, finding and fix; the store reads file, category, severity, verdict, origin and message. Each entry below keeps its words in `message`; file, category, verdict and origin were set at recording.

verdict: needs-revision

I found no blocker. The core Design holds against the code. The warnings below should change before code is written.

Read (file:line):
- `windows.rs:165-220` gives every operation its own `OVERLAPPED` on the stack. The event is the only per-`Pipe` shared state (`:69`, `:174`). Cancellation is `CancelIoEx(handle, &overlapped)`, which names one operation.
- `GetOverlappedResult` is called with that same `OVERLAPPED`.
- The file has no `CancelIo`, no `FlushFileBuffers` and no handle close during a wait.
- `channel.rs:188-223` already has a Windows `read_exact` with the 250 ms `cancel` check, built on `read_within(buf, wait)`.
- `watch_cancel` joins the watcher on every path (`transport.rs:2461-2471`), so the clone is never dropped with a read pending.

Inferred about Windows:
- A duplicate shares the file object.
- Overlapped named pipes allow one pending read and one pending write at once.
- A cancelled `ReadFile` that has not completed loses no bytes.

```findings
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Design, watch_cancel and PeekNamedPipe fallback finding: The Unix watcher does not use a socket read timeout of its own. It calls read_with_cancel (transport.rs:2500), whose loop checks `ended` and uses a 250 ms wait. channel.rs:188-223 already has that loop for Windows over read_within. The same loop works unchanged on the clone, so the PeekNamedPipe fallback is not needed. fix: Say the Windows watcher uses channel::read_with_cancel unchanged. Drop the fallback and acceptance 9's two-form choice, or keep the fallback only as a contingency for a failed run."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Acceptance 3 finding: The test reads once with read_within on the clone. The real consumer is read_with_cancel: a 250 ms CancelIoEx, then a new ReadFile, repeated while the original writes. The hazard is that the clone's cancel disturbs the original's pending write. A short write never pends, so the test would not show it. fix: Use read_with_cancel on the clone. Have the original write more than 64 KiB (BUFFER) so the write pends while the peer does not read. Let the clone cycle at least twice through its 250 ms timeout, then have the peer drain and check that the write completes whole."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Acceptance 5, 9; transport.rs:3635 finding: governed_client_tests stays unix-only, so nothing on Windows checks the watcher's three exits: a cancel frame naming its own id, an end of file from the peer's close, and `finished`. The step shows only that a call answers. Each CI push costs about 25 minutes. fix: Add a cfg(windows) test of watch_cancel over runtime::channel::windows::pair(). It checks: own id cancels; another id does not; peer close cancels; `finished` ends the thread within about 300 ms without a cancel."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Design, ess/domains/transport.yaml:18; scope list finding: crates/connectors-build/generated/metadata-emissions/inputs.json:140-141 holds a sha256 of ess/domains/transport.yaml. Editing the comment changes the file bytes and the digest. That file is not in scope and \"No declaration in ESS changes\" hides the regeneration. fix: Add inputs.json (regenerated) to the scope. Name the generator step in acceptance 7."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: Design, ticks() finding: This would be the third copy of the Windows read (timer.rs:~45-50, and mutation.rs:259-271, which is private to `impl Deadline`). The unit and overflow are fine: 100 ns units times 100 with checked_mul, and a u64 of nanoseconds lasts centuries. The interrupt time is system-wide, so the owner, client and child share one clock. Every tick reader is in bounded.rs (start, until, constrain; serve_read reaches it through budget.until()). mutation::Deadline has its own `now()`. fix: Prefer one pub(crate) helper over a third copy. Keep a cfg(not(any(unix, windows))) arm returning Unsupported."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: Design, try_clone finding: std's OwnedHandle::try_clone does DuplicateHandle with same access and no inherit. Pipe::try_clone then only adds a CreateEventW. peer stays an empty OnceLock in the clone. fix: Say so, to avoid a second unsafe block."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: Cause 5 and \"ordered list of six\" finding: The six points are compile and refusal gates. Past them, the owner path (governed::bounded::resolve, lock_before, the registry read-use records, the credential read) has no cfg gate (grep). Tests of it are all cfg(unix), so it has never run on Windows. A later run may show another defect after this one. fix: Say the six are the known gates, not a proof of the path, and that the first green run is the proof."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: Coupling (item 6) finding: Nothing else relies on the read refusing on Windows. The only assertion of it is bounded::unsupported_tests (bounded.rs:271-280); the other cfg(not(unix)) tests under apps/connectors/tests are helper stubs. The unix-gated tests (governed, supervisor, owner transport) call ReadBudget::start().unwrap() and stay gated. The unified watch_cancel leaves macOS unchanged. fix: Say that D1 of approval-record:wave-20261022 is reversed, and that story:mcp-stdio-on-macos-and-windows is unblocked by this story."
```

Item 4: `Cancel::cancel` (`supervisor.rs:238-240`) is one `AtomicBool` store with no signals or pidfd, so removing its gate is safe. The Windows `terminate` (`process.rs:1170-1177`) already exists. windows-host.yml:52 already runs clippy `-D warnings` on Windows, so the base compiles clean. Once `watch_cancel` is unified, nothing becomes dead code, as far as I could see.

Files: crates/connectors-host/src/local/runtime/channel/windows.rs, crates/connectors-host/src/local/owner/transport.rs, crates/connectors-build/generated/metadata-emissions/inputs.json
