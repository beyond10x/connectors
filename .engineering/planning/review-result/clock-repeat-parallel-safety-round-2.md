---
format: aep.planning-md/3
id: review-result:clock-repeat-parallel-safety-round-2
kind: review-result
status: active
title: 'clock repeat story: parallel-safety critic, round 2'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: needs-revision

Round 1 findings of my axis:
- ESS, `binding.json` and digest collision (blocker): left the story. Change 5 now says no ESS or CLI contract file is touched. The member change moved to `a-clock-refusal-of-issue-and-invoke-names-its-sources`, which has `depends_on` pattern-grant.
- `semantics.md` line shift: left the story. It no longer edits that file, and statement 15 guards it.
- `CHANGELOG.md` missing from scope: answered in scope and in "Shares files with". One text defect remains (finding 3).
- `execution.rs` overlap note: answered. The test files are in the scope list, and the pattern-grant tree has no change in `execution.rs`, `approval_issuance.rs`, `clock.rs` or either clock document. The only shared test file is `owner/mutation/tests.rs`, one line at `:577`.
- Windows lint ordering: answered by the "Shares files with" line and statement 19.
- Reader beside writer, statement 5 baseline: answered by the "base is the tree after that story's merge" sentence. There is still no `depends_on` edge, only a note, which I accept.

Surfaces: 4 of 4 items placed as cited, 0 inferred, 0 unplaced. Acceptance reads traced to a producer in the set: 1 (the `Site` fixture of `mutation/tests.rs`, which pattern-grant edits). 0 are recorded by an edge; the "Shares files with" section records the order as a note.

Thread question: the owner tests reach the acquisition on the thread that set the timeout. `query_timeout()` is read on the calling thread at `clock.rs:387`, and the worker threads only receive the `deadline` value (`:392`). `time()` is synchronous (`execution.rs:84-97`). `site()` sets the timeout at `mutation/tests.rs:718` and `approval_issuance/tests.rs:518` sets it on the test's own thread. See finding 4 for what this means for statement 11.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 110
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 5 and statement 15 say no generated contract page is touched and `git diff` lists no file under `website/docs/reference/contracts`. But `website/docs/reference/contracts/clock.md:114` is a committed rendering of `contracts/service/clock.md:99` and holds the sentence 'are no automatic network retries'. `website/publication.json:6` maps it to that source. The gate step `docs_check` (`crates/connectors-build/src/gate.rs:48-62`, `connectors-docs generate --check`) fails when the page differs from its source. Change 4 therefore forces a change to a file that statement 15 forbids, and the story's scope lists neither the page nor the regeneration. No other story in the set regenerates this page (pattern-grant and connections-through-owner regenerate other pages of the same directory, not `clock.md`), so this is a missing surface, not a collision. Fix: add `website/docs/reference/contracts/clock.md` to the scope (cited); in statement 15 exempt that one file; add a statement that `connectors-docs generate --check` passes."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 102
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 4 replaces one sentence at `contracts/service/clock.md:94-99` with several, so every later line of that file shifts. `ess/domains/cli.yaml:189` cites `clock.md:163`, and `docs/design/approval-clock-quorum.md:17-23,:108,:124,:148,:155,:235,:333,:349` cite its lines. The story edits no ESS file, so the `cli.yaml:189` citation goes stale. Re-pointing it edits the ESS source and may change the contract digest, which pattern-grant also regenerates (`ess/domains/standing_approvals.yaml`, digest in the model pages, `binding.json`). The connections story avoids this by editing `semantics.md` net-zero (its acceptance on `cli.yaml` citations). Fix: add a sentence naming this. Either the `clock.md` text is net-zero in lines, or the stale comment is left and listed as known, or the `cli.yaml:189` comment is re-pointed and `ess/domains/cli.yaml` joins 'Shares files with' as a pattern-grant collision with a regeneration-only rebase step or an ordering edge. Do not choose for the operator."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 109
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'one entry under the next heading' and 'both add lines under the top heading' (`:228`) do not match the base. The top heading of `CHANGELOG.md` is the released `## 0.77.1` (`CHANGELOG.md:3`). pattern-grant creates `## Unreleased` / `### Added` at line 3 (staged hunk `@@ -4,6 +4,31 @@`). `story/connections-through-owner` creates `## Unreleased` / `### Changed` / `### Fixed` at the same line. A third story adding that heading makes the conflict an add/add on the heading itself, not only on entries. Fix: say which section the entry goes under (`### Fixed`), that the story creates `## Unreleased` only if absent, and that the second and third to merge keep one heading and reorder their entries."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 171
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statement 11 names `owner/mutation/tests.rs`, but the only fixture there is `site()` (`mutation/tests.rs:716`). It takes no argument, builds both sources with `Server::fixture` that always answer (`:733-738`), and sets `set_test_query_timeout(WAIT)` with WAIT = 50 s (`:707`, `:718`). A source that drops its first request would then make the first attempt wait 50 s, and 'at least one query deadline remains before `until`' cannot hold inside a 60 s test. The same function (`recheck` module, `#[cfg(unix)]`) is what pattern-grant's tests use (it edits `:577` in the same file). Fix: state that the test calls `set_test_query_timeout` with at most 500 ms after `site()` on the test thread. State that the sources of 11 are built with `Server::shared` (`server.rs:86`) or `Server::spawn` with a request-count offset taken after the proof is issued, since `n` counts cumulatively (`server.rs:98`), and that `site()` gets a parameter or a sibling constructor. Say in 'Shares files with' that this edits `site()`, which pattern-grant's tests use."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 177
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statement 13 names no file. `rg recover_observation` finds no recovery test in `crates/connectors-host/src/local/owner`; the only callers are `owner/supervisor.rs:1188` and `service.rs:944` (`state.recover_observations()`). 'The module that holds recovery's tests' does not exist, and the test may land in `service.rs` or `supervisor.rs`, which are in neither the scope list nor 'Shares files with'. Fix: name the file (either `owner/mutation/tests.rs`, inside the `site()` module, or a new test module of `recovery.rs`) and add it to the scope; drop the fallback sentence."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 230
  category: parallel-safety
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The no-shared-change claim holds against the pattern-grant tree read now: it has no change in `execution.rs`, `approval_issuance.rs`, `clock.rs`, `clock.md`, `local-clock.md` or `local_cli.rs`. Its `mutation/tests.rs` change is the single line at `:577`. The tree is moving: `apps/connectors/src/local/approvals.rs` (a `listed()` call for `list-standing`) appeared in it during this review. 'As of that tree on 2026-11-24' carries no time and no commit. Fix: add the head commit (`d45310985`) and the time, and name the pattern-grant story's own shared-files list (its 'Order and shared files' section) as the source to recheck. The connections branch is based on 2e5ddabe9 and touches `owner.rs`, `owner/admission_tests.rs`, `owner/transport.rs` and `semantics.md`, none of them in this story's scope, so 'CHANGELOG.md' is the only shared file, as stated."
```

Out of my lane:
- Change 3 and the "Decided" section say clock-check's contract says "once" at `contracts/cli/v1alpha1/semantics.md:131`. That line is the `below_quorum` next-action text. The words "asks every source once" are in `docs/local-clock.md:91`, and `semantics.md:123` only refers to the clock binding. This is a citation error for the design or acceptance critic.
- I did not judge the 12 s budget or whether the statement 4 timing is checkable.
