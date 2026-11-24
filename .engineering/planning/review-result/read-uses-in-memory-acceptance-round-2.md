---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

Round 2 is much closer. Seven of the eleven round-1 findings are answered, three are answered in part, and one is moot. Two lines still cannot be done as written.

**Findings**

1. **Line 7 cannot be produced.**
   - `deletion_allowed` (`use_and_retirement.rs:447`) refuses only when a use has `released=0 AND expires_at_ms>now`.
   - Deletion needs `now >= delete_not_before_ms`, which is the retire time plus `RETENTION_MS` (`registry.rs:55`, 86,400,000 ms; `lifecycle.rs:788`).
   - A use lasts at most `USE_MS` (`registry.rs:56`, 120,000 ms) and is captured while its version is still active.
   - So no use of a retired version is open when deletion is allowed, and the test "open use, then `Conflict`" has no setup through public calls.
   - An undispatched use is also released at retirement (`refresh.rs:422-425`).
   - `delete_retired` needs a `custody::Store`, and line 7 names no location. The pattern is `keyring/custody/tests.rs:466`.
   - Change: drop line 7 and state that the guard is unreachable by clock arithmetic, or seed the use by direct SQL in `Metadata::update`. Design difference 3 (`deletion_allowed` sees no open use in another process) is vacuous for the same reason.

2. **Line 3 fails on a legitimate rebase, and does not prove "no SQL statement changes".**
   - The base is fixed at `3d38f77b99f` and the pathspec includes `owner` and `apps/connectors/src`.
   - `story/connections-through-owner` lists `apps/connectors/src/local/connections.rs` and `owner/transport.rs` in its scope. If it merges first, this diff is non-empty after the rebase.
   - The pathspec also leaves `metadata.rs` and the other `registry/` files out.
   - Change: use `git diff --stat main...HEAD -- <paths>`, and add `git diff -U0 main...HEAD -- crates apps | grep '^[-+].*registry_uses'`, which may show only `er.rs` hunks and test files.

3. **Line 4 contradicts itself and has no command.**
   - "Pass with their files unchanged" lists tests in `registry/tests.rs`, but lines 2, 5, 7 and 10 edit that file.
   - `reuse_tests.rs` and `recipient_tests.rs` are not in line 3's diff.
   - Change: say "test bodies unchanged", add `git diff --stat main...HEAD -- reuse_tests.rs recipient_tests.rs` printing nothing, and name line 18's filter as the run.

4. **Line 10 names no switch.** The command `git grep -B1 <the switch's name>` cannot run. Name the switch in the Design.

5. **Line 11 has no pass or fail.**
   - Nothing observes "the measured invokes run with the switch off".
   - With the switch off, the growth loop (`store_cost_tests.rs:157`, `while recorded_events < events`) would never end, so the first half checks itself.
   - Change: on an empty `<dir>`, the run on this branch prints `events` above 17 and `appended_per_invoke=0`.

6. **Line 2 is vague, and the helper's output is unproven.**
   - It does not say which use gets which terminal call (cancel needs an undispatched use, refuse a dispatched one). A test with fewer paths would still pass.
   - Assign one use per path: read, reuse and revalidation each, then cancel, refuse and invalidate.
   - I could not tell whether the recorded wrapper blob carries the lifecycle state. `recorded_clock_floor_events` only matches the subject name. Line 8 is the only positive control, so state that the helper is non-empty with the switch on.

7. **Lines 5 and 6 leave out calls.**
   - Line 5 does not name the dispatch call. It is `dispatch_read` for the first case and `dispatch_revalidation` for the second.
   - Line 6 does not say which process dispatches the first use's id.

8. **Line 9 has an ambiguous line range.** The switch's pass-through near `metadata.rs:124-139` adds lines. Say "old-side ranges of `git diff -U0` lie outside 2455-2582".

9. **Lines 12 and 18 omit the slot limit.** Two release builds and a filtered nextest run each risk the 10-minute job limit. Name `SLOT_TIMEOUT=30m` or `slot --timeout`, as the repository's `AGENTS.md` requires.

10. **Lines 14 to 17 leave some changes unchecked.**
    - Line 16's `local-er-metadata.md` changes (`:79-82`, `:287`) are paraphrased, not quoted, so a reader's judgement decides. Quote the sentence or pin a `git grep` phrase.
    - Line 17's "the five differences" has no pinned wording.
    - `connectors-docs generate --check` is not a runnable form. Write the `cargo run -p connectors-docs -- generate --check` form.

**Round 1 mapping**
- Finding 1: answered by `recorded_read_uses` and by line 1 sitting in `store_cost_tests.rs`. The table accessor is moot, because the SQL still answers.
- Findings 2, 3, 4, 5, 8 and 11: answered (lines 1, 4, 5, 9, 13, 17).
- Finding 6: the cancel-versus-never-captured part is moot, since the rows stay in SQL. The switch source is answered but unnamed (finding 4 here), and `grown_stores` is still weak (finding 5).
- Finding 7: answered on the store and the command, but the slot time is missing (finding 9).
- Finding 9: answered by lines 14 to 16, apart from finding 10 here.
- Finding 10: answered by line 18.

**Read**
- The story artifact and the round-1 review, in full.
- At `3d38f77b99f`, these cites check out:
  - `measured_invoke`, `recorded_events` (`tests.rs:1839`, imported), `blob_bytes`, `grown_stores`, `recorded_clock_floor_events`, and the fixture test at `store_cost_tests.rs:470`, which asserts `recorded > loaded` at `NOW+10_000`.
  - `use_and_retirement.rs:294-295` and `revalidation.rs:166-171`: the released check precedes the revoked check in both, so the answer moves from `Conflict` to `Revoked`.
  - `lifecycle.rs:469`, `refresh.rs:422`, and the `expiry_batch_tests`, `metamorphic_tests` and `reuse_tests` cites.
  - `semantics.md:2094` and `website/docs/reference/contracts/cli.md:2109`.
  - `owner.md:210`, and the `local-connection-registry.md` lines `:220`, `:233` and `:282`.
  - `CHANGELOG.md:3`, `the_committed_metadata_emissions_are_current`, `cli_spec_mapping_adversary.rs:202`, and the `local_cli` binary.
- Line 16's grep matches exactly the five lines to be changed, so it is a real before/after check. Line 12's output keys exist (`store_cost_tests.rs:385`).
- The line 18 filters are sound.
- I opened no fixture and ran nothing.

**Could not establish**
- Whether the fixture test's invoke adds exactly 1 event. That depends on the clock row, and the fixture was not opened.
- Whether the wrapper blob carries state for `recorded_read_uses`.
- Whether the checkpoint write at `er.rs:709` adds blob bytes.
- Design questions (static versus handle visibility across processes) are out of my lane.

```findings
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 171
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 cannot be produced: deletion needs now >= retire+RETENTION_MS (86,400,000 ms, registry.rs:55) while a use ends within USE_MS (120,000 ms), so deletion_allowed's open-use check (use_and_retirement.rs:447) never holds through public calls; drop the line or seed by direct SQL, name the location (custody Store needed, keyring/custody/tests.rs:466), and drop or restate Design difference 3"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 158
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 diffs against fixed base 3d38f77b99f over owner and apps/connectors/src, which story:connections-list-answers-through-the-owner edits (connections.rs, owner/transport.rs), so it fails after a legitimate rebase, and it does not prove no registry_uses statement changes; use main...HEAD and add git diff -U0 | grep registry_uses showing only er.rs and tests"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 162
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4 says tests pass 'with their files unchanged' but registry/tests.rs is edited by lines 2, 5, 7 and 10, and reuse_tests.rs and recipient_tests.rs are in no diff command; say test bodies unchanged, add the diff command and point to line 18's filter as the run"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 173
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 10 names no switch: 'git grep -n -B1 <the switch's name>' cannot be run; name the switch in the Design and in the command"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 178
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 11 has no pass/fail for 'the invokes that read_invoke_cost_by_store_size measures run with the switch off'; add an observation, e.g. a run on an empty <dir> on this branch prints events above 17 and appended_per_invoke=0"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 154
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 does not say which use gets which terminal call (cancel needs an undispatched use, refuse a dispatched one) so a test with fewer paths passes, and nothing shows recorded_read_uses is non-empty with the switch on"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 164
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptances 5 and 6 do not name the dispatching call (dispatch_read, dispatch_revalidation) or the process that dispatches the first use's id"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 171
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 9: the switch's pass-through at metadata.rs:124-139 shifts later lines, so 'no hunk in lines 2455-2582' is ambiguous; say the old-side ranges of git diff -U0 lie outside 2455-2582"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 179
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptances 12 and 18 run release builds and a filtered nextest without naming the slot time (SLOT_TIMEOUT=30m or slot --timeout), which the 10-minute job limit requires"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 190
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptances 14 to 17: local-er-metadata.md :79-82 and :287 are paraphrased, not quoted, and 'the five differences' has no pinned wording, so a reader's judgement decides; quote the sentences or pin grep phrases, and write the runnable form of connectors-docs generate --check"
```
