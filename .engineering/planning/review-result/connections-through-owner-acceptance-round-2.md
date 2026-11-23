---
format: aep.planning-md/3
id: review-result:connections-through-owner-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: needs-revision

Eight of my nine round-1 findings are resolved, one is partly resolved, and I have two new findings, so one line is left to revise and one is partly open.

**Round-1 findings**

1. Resolved. `connections-describe` leaves `SELECTING` and the arm asks the owner first. Line 1 now lists describe without `--connection` and requires no `open` and no `snapshot` line.
2. Resolved. The log must hold no `open` and no `snapshot` line at all, after an owner started by a cold read and drained with `passes()`. That no longer depends on telling the CLI pid from the owner pid.
3. Resolved. The acquisition id comes from the `connect` result in `connected()`, with a stated fallback to a pending acquisition.
4. Resolved. The two time keys are removed at every depth, and the order is stated: no-owner answers kept first, after `owner-stop`, then the owner runs.
5. Resolved. A refusal now falls back to the direct read, so the wrapper returns `None` for it as well. The fixtures are named: `admission_tests.rs:140-183` for another build and `answer_once` at `:191` for a refusal or a dropped stream.
6. Partly resolved. The harness (the existing ignored test), the statistic (median of three), the `uptime` line and the release build are now named, and the bar is gone. The fallback clause still cannot be checked (new finding 2).
7. Resolved. The prose edits, the `owner.md` change and the CHANGELOG entry sit on separate lines 7-9, each checkable by `git grep` or reading the entry. The `semantics.md` line count is still 2334 and the edit can keep it.
8. Resolved apart from "the generated directories" in line 10 (new finding 3). The drift commands are cited at `AGENTS.md:149-153` and the how moved to the notes.
9. Resolved. The filter expression is given and `SLOT_TIMEOUT=30m` is in the notes. The `<the new host test>` placeholder is acceptable until the test is written.

**New findings**

1. Line 2 (`.engineering/planning/story/connections-list-answers-through-the-owner.md:156-158`) still says "a host test compares the two builders' inputs for that case". The owner now returns data and the CLI builds the document once, so there are no two builders. The clause does not say what the host test compares, so it cannot pass or fail. Name the comparison (the registry value against the same value after the wire round trip, for the pending, no-external-identity and binding cases), or drop the clause.
2. Line 6 (`:170-175`) says the report "says whether any owner run fell back", but the run is on a release build. `cold_owner_start.rs:320-323` and `:376` say a release build logs no verification, so there is no signal that shows a fallback. The line also does not say how the read invoke is held in flight. Name the signal, for example the same three commands once on a test build with the log, or an owner-side marker. Say how the invoke is held in flight.
3. Line 10 (`:184-185`) says "no change under `ess/`, `apps/connectors/spec/` and the generated directories", but the generated directories are not named, so two people could check different paths. The tree has `crates/connectors-build/generated`, `crates/connectors-core/generated` and `adapters/*/generated`. Give a pathspec for `git diff --stat 5983a6b9502 --`.

Out of my lane: the choice of one `observe` kind with a selector, and the Shares-files list.

I read 1 of 1 ids in full, plus the round-1 review, the tree at base 5983a6b9502 (`connections.rs:97-170`, `cold_owner_start.rs` in full, `mounted.rs`, `semantics.md:1374-1384`, `local-er-metadata.md:266-273`, `owner.md:250-262`, `.config/nextest.toml`) and `AGENTS.md:140-160`. I could not establish whether the owner logs `snapshot` for these commands, or whether the mounted fixture can hold the pending, no-identity and binding connections of line 2. Line 2's conditional wording covers the second.

```findings
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 156
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 2: "a host test compares the two builders'' inputs for that case" is stale; the CLI is the only builder now, so the line does not say what the host test compares (the registry value against the value after the wire round trip) (.engineering/planning/story/connections-list-answers-through-the-owner.md:156-158)'
- file: apps/connectors/tests/cold_owner_start.rs
  line: 376
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 6: "says whether any owner run fell back" has no signal on a release build, which logs no verification (cold_owner_start.rs:320-323, :376); name the signal (a test-build run with the log, or an owner-side marker) and how the read invoke is held in flight'
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 184
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 10: "the generated directories" is not a path list; name crates/connectors-build/generated, crates/connectors-core/generated and adapters/*/generated as the git diff --stat pathspec (.engineering/planning/story/connections-list-answers-through-the-owner.md:184-185)'
```
