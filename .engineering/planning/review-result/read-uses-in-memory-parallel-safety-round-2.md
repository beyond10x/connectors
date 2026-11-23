---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
Recorded by the session that ran the panel: the store reads the severities blocker, warning and note, so `severity: nit` in the findings block below is written `note`. The words of the review are unchanged.

verdict: needs-revision

Paths below are under `.engineering/planning/story/`.

1. **Acceptance 3 fails if the connections story merges first.** It diffs `owner` and `apps/connectors/src` against the fixed base `3d38f77b99f` and expects no output. The connections story's working tree edits `owner.rs`, `owner/transport.rs`, `apps/connectors/src/local/connections.rs` and `owner/admission_tests.rs`. After a rebase onto main with that story merged, the check prints its diff. "Shares files with" allows either merge order, so the check must hold in both.
   - Change: make acceptances 3, 9 (the metadata.rs hunk check) and 13 diff against the merge-base with the target branch. Or add "minus the hunks of story:connections-list-answers-through-the-owner" to the check. Cited, acceptance 3 at line 202.

2. **Acceptances 5, 6 and 7 name no file for their new tests.** Acceptance 4 holds `registry/reuse_tests.rs` unchanged, and any other file is outside the scope. `simulate_process` is `pub(super)` in `metadata.rs`, so `registry/tests.rs` can call it. A test in a file outside the scope would also need a `mod` line in `registry.rs`, which acceptance 3 forbids touching.
   - Change: name `L/registry/tests.rs` as the home of the three tests, or say that each test lives there. Cited, lines 219-228.

3. **The connections story's own copy does not name this story's files.** Its "Shares files with" (worktree line 204) lists only other stories. It lists no `owner.md`, `local-er-metadata.md` or generated mirror clash with this story. The mirror and `semantics.md` hunks both edit; this story's `CHANGELOG.md` entry joins the heading that story creates. The collision is therefore admitted only in this story. That is acceptable for the pair as long as this story states the order, which it does ("the second to merge"). I did not touch the other worktree.
   - Change: have the connections story's next planning commit add one line naming this story for those five files. Warning only.

4. **Nit.** `crates/connectors-host/src/local/metadata.rs` is still marked `inferred` in the scope. The Design now cites it (`:124-139`), so it can be marked `cited`.

Round-1 findings:
- **1 (`er.rs` and `metadata.rs` missing from the scope):** answered. Both files are in the scope, and `process()` and `exit_process` are in `er.rs`.
- **2 (wording on `owner/`):** answered. The Design says "no production file", and the owner test files left the scope.
- **3 (shared files, regions, order):** answered, apart from finding 1 above. The mirror, the CHANGELOG heading, the regions and the +2 line shift are named. Regenerating the mirror falls to the second to merge. I checked these against the connections diff: `owner.md` `:239`+ versus `:207-211`, `semantics.md` net 0, `local-er-metadata.md` +2.
- **4 (the store-tests story):** answered. It has a three-file scope and a `depends_on` edge to this story.

Scope against the body: every file the Design and acceptances 1-18 need is in the scope.

The connections working-tree diff touches none of `metadata/er.rs`, `metadata.rs` or the four test files this story edits (`store_cost_tests.rs`, `registry/tests.rs`, `expiry_batch_tests.rs`, `metamorphic_tests.rs`). It does not grep for `ReadUse`, `registry_uses`, `simulate_process` or `exit_process`.

Counts: 4 items placed (3 cited, plus `metadata.rs` inferred), 0 unplaced. Acceptance reads traced to a producer: 2 (the 0.32.0 fixture, the bytes-per-read report), both recorded by a `depends_on` edge.

What I read: the story in full, the store-tests story, the round-1 review, and the connections worktree's `git diff` with `-U0` per file. I also read its "Shares files with" section and ran `git grep` for the tests that count events, blobs or `registry_uses`.

What I could not establish: whether the 5 slow store tests still fail after this story (a timing question). Whether `observe` in the owner changes the Design's claim that the CLI builds the list projections (design lane). The connections story in the main store is revision 1 and has no scope.

```findings
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 202
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: Acceptance 3 expects an empty diff of owner/ and apps/connectors/src against base 3d38f77b99f, but story:connections-list-answers-through-the-owner edits owner.rs, owner/transport.rs and apps/connectors/src/local/connections.rs, so the check fails if that story merges first. Diff against the merge-base with the target branch, or exclude that story's hunks (cited).
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 219
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: Acceptances 5, 6 and 7 name no file for their new tests. Name crates/connectors-host/src/local/registry/tests.rs, which is in scope; reuse_tests.rs is held unchanged by acceptance 4 and a new module needs a registry.rs edit that acceptance 3 forbids (cited).
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 204
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: The connections story's Shares files with section does not name story:settled-read-uses-leave-the-store or the five shared files (CHANGELOG.md, cli.md mirror, semantics.md, owner.md, local-er-metadata.md); only this story admits the collision (cited, worktree copy).
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 1
  category: parallel-safety
  severity: note
  verdict: needs-revision
  origin: introduced
  message: The scope marks crates/connectors-host/src/local/metadata.rs inferred, but the Design now cites it at metadata.rs:124-139; mark it cited.
```
