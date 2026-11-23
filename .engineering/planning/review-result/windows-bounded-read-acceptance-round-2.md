---
format: aep.planning-md/3
id: review-result:windows-bounded-read-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:windows-mcp-tool-call-answers-timeout
relations:
- reviews: story:windows-mcp-tool-call-answers-timeout
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 213
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 8 cannot pass as written. The contract wraps at about 80 columns (lines 12-20 are 59-83 wide), so the phrase 'unbiased interrupt time' can split across two lines, and a fixed-string `-F` search then prints nothing. 'Prints one line' is also wrong for a phrase that spans two. Fix: replace with: `rg -n -U 'unbiased\\s+interrupt\\s+time' contracts/cli/v1alpha1/private-bounded-read.md` prints at least one line, and `rg -n 'Unix only' contracts/cli/v1alpha1/private-bounded-read.md` (which prints line 17 today) prints nothing."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 186
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 3 depends on a write that pends, but the pipe buffer is exactly 64 KiB per direction (`windows.rs:61`, `BUFFER = 64 * 1024`). 'More than 64 KiB' may be only a few bytes over, and the write may complete or pend depending on the pipe's quota. The test also does not say how 'passes its 250 ms check at least twice' is observed. Fix: write 'The original, on its own thread, writes 256 KiB while the peer does not read'. Then add: 'the closure passed to `read_with_cancel` counts its calls and the test asserts the count is at least 2 before the peer starts reading'."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 143
  category: acceptance
  severity: note
  verdict: approve
  origin: introduced
  message: "The Design says the mirror `website/docs/reference/contracts/cli.md` is regenerated, but that file does not mention `private-bounded-read` or `CLOCK_MONOTONIC`. No acceptance line can check a regeneration that produces no diff. Fix: write 'its mirror `website/docs/reference/contracts/cli.md` holds no copy of the sentence; `generate --check` of acceptance 8 decides it'. The contract file is not an input of `inputs.json`, so editing it does not change the digest."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  line: 192
  category: acceptance
  severity: note
  verdict: approve
  origin: introduced
  message: "Acceptance 4 can be built from the names it uses. `windows::pair()` is `pub(crate)` and returns `(Pipe, OwnedHandle)`, and `Pipe::from_owned` is `pub(crate)`. `Cancel` derives `Default`, so a test can construct it. For case (b), 'the thread still runs' needs the handle's `is_finished()`. Fix: write 'the watcher's `JoinHandle::is_finished()` is false'. The `ends()` helper lives only in `channel/windows/tests.rs`, so the transport tests build their own pair."
```

I checked these in the tree:
- Acceptance 2 (the new non-gated test module in `projection.rs`) holds: the existing test modules at `mod.rs:97` and `projection.rs:863` are both `cfg(all(test, unix))`.
- Acceptance 3 uses `read_with_cancel`, which is `pub(crate)` with the signature `(&mut Stream, until, bool, usize, Option<&dyn Fn() -> Result<()>>)`. `channel/windows/tests.rs` already imports it.
- All three `--test` binaries in acceptance 7 exist and are Linux-only.
- The ESS comment is dropped, so no file under `ess/` or `crates/connectors-build/generated` changes. That is consistent with acceptance 8's `git diff --stat` guard.
- `rg -n -i 'unix only|only a unix owner'` over the three files named in acceptance 8 prints `bounded.rs:97` and `contract:17` today, so it turns on the change.
- Acceptance 5, 6, 9 and 10 are single-outcome and decidable.

I could not check the behaviour of a concurrent read and write on a cloned overlapped pipe. Only the Windows job decides that.
