---
format: aep.planning-md/3
id: review-result:windows-bounded-read-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:windows-mcp-tool-call-answers-timeout
relations:
- reviews: story:windows-mcp-tool-call-answers-timeout
revision: 1
---
verdict: needs-revision

Nothing here is a blocker. Three wording corrections remain.

(1) `clock/timer.rs` exposes no nanosecond read that `bounded.rs` can call, so the implementor must hold the call (a small copy).
- Read: `timer.rs:41-47` has the Windows `fn read(clock: InterruptTime)`. It returns units times 100 via `checked_mul`, but it is private. `SUSPEND_EXCLUSIVE` (`timer.rs:38`) is private too, and `mod timer;` is private in `clock.rs:11`.
- The only `pub(super)` item is `Stamp::read()`. It samples three clocks, fails on a 1 ms bracket, and is visible only inside `clock`.
- Widening the visibility edits `clock.rs` and `timer.rs`, which no scope line covers.

(2) Acceptance 3 and 4 can be written with what exists.
- Read: `pair()` (`windows.rs:515`) returns `(Pipe, OwnedHandle)`, and `tests.rs:22` already wraps the client as `Pipe::from_owned`.
- `channel::Stream` is `Pipe` on Windows (`channel.rs:15`), so `watch_cancel(&server, …)` takes the server end directly.
- A peer frame is `channel::write(&mut client, &CancellableRequest::Cancel{request_id}, None, &[], until)`. The enum is private to `transport.rs`, so a test module there sees it, as `say()` at `transport.rs:3668` does. `supervisor::Cancel::cancelled` is visible there too.
- Buffers: `BUFFER = 64 * 1024` (`windows.rs:62`) is passed as both the out and in size to `CreateNamedPipeW` (`:471-472`).
- Inferred: a byte-mode overlapped write larger than that quota pends until the peer reads. Use 256 KiB, not 64 KiB plus one, for margin.
- The write blocks its caller in `write_within` (up to 10 s), so the test needs a separate thread for the write. Otherwise "the test waits 600 ms, then the peer reads" cannot run.

(3) In case (d), `finished` ends the blocked read well within 1 s.
- Read: `read_exact` (`channel.rs:195-205`) runs `check()` at the top of each loop and then waits `min(remaining, 250 ms)`. `ended` then returns `Interrupted`. `watch_cancel` returns without cancelling because `finished` is set (`transport.rs:2515-2519`).
- The join takes about 250 ms or less. The test must pass a far `until` (the Unix helper uses 20 s). A near `until` would end as `Timeout` and cancel.

(4) The Design says the contract's mirror `website/docs/reference/contracts/cli.md` is regenerated, but no such mirror exists.
- Read: `cli.md` has no match for `deadline_ticks`, `invoke_bounded` or `CLOCK_MONOTONIC`. The only website page with those terms is `connectors-transport.md`, generated from the ESS file you now leave alone.
- Read: `rg -l` finds `private-bounded-read.md` only under `contracts/`.

```findings
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 1
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design, ticks(): it says the Windows arm calls the read in clock/timer.rs, but that read is `fn read` at timer.rs:41-47, private, inside private `mod timer` (clock.rs:11), and Stamp::read is pub(super) and does a 3-sample bracket check. bounded.rs cannot call either. Say the implementor holds the call: a few lines of QueryUnbiasedInterruptTimePrecise times 100 with checked_mul, behind cfg(windows), and no edit to clock.rs, timer.rs or mutation.rs."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 1
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design, contract paragraph: it says the mirror website/docs/reference/contracts/cli.md is regenerated, but cli.md holds no text of private-bounded-read.md, and no website page does. Drop the mirror sentence and let `connectors-docs generate --check` (acceptance 8) say whether anything changes."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 1
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 3: say the original's write runs on its own thread (write_within blocks its caller while it pends, so the test thread could not start the peer's read after 600 ms), and make the write 256 KiB or more. The pipe buffer is 64 KiB per direction (windows.rs:62, :471-472), and that a larger write pends is inferred from the Windows API's rules, not shown. Acceptance 4(d): name a far `until` (at least 20 s) so a Timeout path cannot cancel."
```
