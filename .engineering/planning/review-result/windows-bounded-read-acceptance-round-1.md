---
format: aep.planning-md/3
id: review-result:windows-bounded-read-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:windows-mcp-tool-call-answers-timeout
relations:
- reviews: story:windows-mcp-tool-call-answers-timeout
revision: 1
---
Recorded by the session that ran the panel: the brief asked for the members severity, where, finding and fix; the store reads file, category, severity, verdict, origin and message. Each entry below keeps its words in `message`; file, category, verdict and origin were set at recording.

verdict: needs-revision

```findings
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "where: Design (watch_cancel on Windows) / acceptance 3, 5, 9 finding: The story rewrites the cancel path for Windows, but no line shows a cancel firing there. Line 3 tests only the pipe clone. Line 5 shows a call finishing, which exercises only the `finished` exit of the thread. The cancel-on-own-id, cancel-on-other-frame and cancel-on-EOF arms, and `finished` ending a blocked read within 250 ms, are never run. Out of scope sends the Windows MCP cancel integration tests elsewhere, and `governed_client_tests` stays Unix-gated. fix: \"Add acceptance 3b: a `cfg(windows)` test next to `watch_cancel` in `owner/transport.rs`, on a Windows pipe pair. (a) The peer writes a `cancel` naming the request id, and `Cancel::cancelled()` is true within 10 s. (b) The peer writes a `cancel` naming another id, and `cancelled()` stays false. (c) `finished` is set with no frame sent, the watcher thread joins within 1 s, and `cancelled()` stays false. The log of 'host and CLI on windows-latest' shows each test `ok`.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 2 finding: No non-gated test location exists in `apps/connectors/src/mcp/`. `mod tests;` at `mod.rs:97` and `mod tests` at `projection.rs:863` are both `cfg(all(test, unix))`. The implementor must add a new `#[cfg(test)]` module, and the line does not say so. \"Fails before the change\" is also loose: the function does not exist yet, so the test fails to compile rather than returning `timeout`. The Windows job's `mcp::` step (`windows-host.yml:79`) would run the test there. fix: \"A new `#[cfg(test)]` module (not `cfg(unix)`) in `apps/connectors/src/mcp/projection.rs` calls `<function>`: `Failure::Unsupported` gives `unsupported`, `Failure::Unavailable` gives `timeout`. Before the change the function does not exist, and the old inline `map_err` gave `timeout` for both. It passes after, on Linux, with `cargo nextest run -p connectors -E 'test(<its name>)'`. The report names the function and the test.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 6 finding: The filters match real paths (`local::runtime::bounded::tests::*`, `local::owner::transport::*`). `test(mcp::)` matches only the bin unit tests. The Linux integration tests of the changed Unix `watch_cancel` (`adversary_mcp_deadline`, `security_mcp_deadline`, `mcp_tool_call_shapes`) run in no line. Line 6 is a regression guard and does not fail before the change; it should say so. fix: \"Append: `cargo nextest run -p connectors --test adversary_mcp_deadline --test security_mcp_deadline --test mcp_tool_call_shapes` exits 0 under the same bounds (each test within 60 s, the run within 120 s). Line 6 guards Unix behaviour after the `watch_cancel` merge and is not expected to fail before the change.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 7 finding: \"'The sentence of the Design' is a paraphrase, so it cannot be matched. `rg 'Unix only'` prints today only because of line 17 of the contract. Other stale text is not covered: the doc comments at `bounded.rs:97` and `transport.rs:2526`, and the `ess/domains/transport.yaml:18` comment, which the Design says it updates and which says 'this Linux boot's'. The docs sentences that actually exist say 'passed on Linux only so far' and 'not yet shown' (`docs/local-mcp-stdio.md:10-16`, `website/docs/connectors/mcp.md:65-68` and `:204`). None of them says a call 'does not work on Windows', so the clause as written matches nothing.\" fix: \"Replace with: `rg -n -F 'unbiased interrupt time' contracts/cli/v1alpha1/private-bounded-read.md` prints one line, and `rg -n -i 'unix only|only a unix owner' contracts/cli/v1alpha1/private-bounded-read.md crates/connectors-host/src/local/runtime/bounded.rs crates/connectors-host/src/local/owner/transport.rs` prints nothing. Add: the report quotes before and after each sentence in the two docs that says serving or a tool call is shown on Linux only or not yet shown on Windows, or says it was changed after the green run.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 9 finding: A run id does not show which form was built. The code at the head does. fix: \"Replace with: 'The report names the form built (a read on the clone, or `PeekNamedPipe` polling), the commit, and the run id whose acceptance 3 test and step 10 ended `ok` with it. If the second form was built, it also names the run id where the first form failed.'\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 3 finding: The test lives in `channel/windows/tests.rs` (`mod tests;` at `windows.rs:639`), not in `windows.rs`. Writing to the peer before the reader thread is blocked makes the test pass vacuously. fix: \"Say 'in `channel/windows/tests.rs`'. The thread signals just before its read, and the original writes only after a 200 ms wait.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 1 finding: Cross-building two packages with `--all-targets` may pass 10 minutes in the pool. fix: \"Prefix both commands with `SLOT_TIMEOUT=30m`.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 4 / Design (`ticks()`) finding: Nothing checks the unit or monotonicity of the Windows `ticks()`, nor that other platforms keep `Unsupported`. fix: \"Add to line 4: a `runtime::bounded::tests` case where two `ticks()` calls 100 ms apart differ by 100 ms to 5 s. Drop the 'other platform keeps Unsupported' sentence from the Design, or add a line for it.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: acceptance 5 finding: macOS passes the step today, so that half is a regression guard, not a check on this change. fix: \"Say 'the macOS step stays `success`'. Name the pull request of branch `story/windows-bounded-read`.\""
```

I read the story (whole body), the cited code, `windows-host.yml` and `macos-host.yml`. I did not build or run anything. I could not establish how long the `owner::transport::` tests take on Linux against the 120 s budget, or whether `try_clone` on a read and write at the same time works on a Windows overlapped pipe.
