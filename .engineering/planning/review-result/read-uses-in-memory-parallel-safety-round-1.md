---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

Two stories can be worked beside this one. The set is not safe to run unchanged. The story's scope omits two files its Design needs, and its "Shares files with" section leaves out one collision and leaves two others without regions or an order. Two stories have no scope in this store.

Findings (path prefix `.engineering/planning/story/`; `L/` is `crates/connectors-host/src/local/`):

1. story:settled-read-uses-leave-the-store: The Design keys the table "by the simulated process in tests" (line 127-129). The only source of that key is `fn process()` in `L/metadata/er.rs:636-640`, which is private and `#[cfg(test)]`. `L/metadata.rs:124-125` re-exports only `simulate_process`, the setter. Reading the current id from the registry module needs a new accessor in `er.rs` and `metadata.rs`. Neither file is in the scope (cited, `settled-read-uses-leave-the-store.md:127-129`). No story in the set names `er.rs` or `metadata.rs`, so there is no collision today, but the scope and the "Shares files with" claims are incomplete.
   - Change: add both files to the scope with the region (a `cfg(test)` accessor next to `simulate_process`).
   - Also state whether `exit_process()` (`er.rs:2936`) clears the table, because acceptance 6 depends on it.

2. story:settled-read-uses-leave-the-store: The claim that nothing under `L/owner/` changes holds for production code but is worded two ways.
   - Production holds. `owner.rs:1182`, `owner/supervisor.rs:1355`, `:1484`, `:3444`, `owner/transport.rs:3427`, `:3582` and `owner/mutation/execution.rs:43-51`, `:203`, `:446` call `capture_*`, `dispatch_read`, `cancel_read` and `invalidate_read`. All keep their signatures under the Design, and `Registry::with_system_clock` is rebuilt per request. The static keyed by state directory is therefore the right shape.
   - Only two owner files read `registry_uses`: `owner/approval_issuance/tests.rs:769` and `owner/governed/tests/final_fence.rs:281`. Both are the two the story lists.
   - The wording conflicts. Design line 129 says "no file under `owner/` changes". Line 211 and acceptance 10 say two test files change.
   - Change: Design line 129 should say "no production file under `owner/`; two test files, listed in 'Shares files with'".
   - Check against the connections story: it edits `owner.rs`, `owner/transport.rs` and `owner/admission_tests.rs`. None of those hold use rows, so there is no overlap.

3. story:settled-read-uses-leave-the-store: "Shares files with" (lines 201-214) names the connections story's collisions without regions, an order or the generated mirror.
   - `website/docs/reference/contracts/cli.md` is missing from the section. The connections worktree diff edits it at `@@ -1393,3` (the mirror of `semantics.md:1378-1380`). Its scope omits it, but the file is edited. This story regenerates the same file (acceptance 13). A generated file cannot be merged by hand.
   - `CHANGELOG.md`: the connections worktree adds a new `## Unreleased` and `### Changed` at line 3. This story needs the same heading. The section says only "the second to merge rebases". Both add at the head of the file, so the second merge gets a text conflict or a duplicate heading.
   - Line numbers shift. The connections diff in `docs/local-er-metadata.md` is `-270 +270,3` (+2 lines), so this story's `:287-292` moves by 2 if it lands first. The connections diff keeps `semantics.md:1378-1380` at 3 lines, which is why the "no line above :1512" rule in acceptance 13 holds today.
   - Change: add the mirror, the `## Unreleased` heading and these line shifts. Replace "the second to merge rebases" with an order: the second to merge rebases, merges its entry under the existing `## Unreleased` / `### Changed`, and regenerates the mirror with `connectors-docs generate`.

4. story:store-tests-end-within-60s-under-load: This story runs after the settled-read story, and that order exists only in the settled-read body (line 204). The store-tests story has no `scope` and no `depends_on`. Its relations are `decomposes` and `serves` only. Its body does not mention the settled-read story (cited; `store-tests-end-within-60s-under-load.md:7-8`).
   - The shared surface is real. The store-tests story names four tests in `L/registry/tests.rs` (`:787`, `:1463`, `:2061`) and `L/registry/store_cost_tests.rs:529`. The fifth is in `crates/connectors-host/tests/metadata_reopen_adversary.rs:175`.
   - The settled-read story rewrites `grown_stores` (`store_cost_tests.rs:164-166`) and `seed_recorded_events` (`tests.rs:1746`, `:1799`). It also changes the read cost that the 60 s times were measured on.
   - Change on the store-tests story: add a `scope` of those three files and a `depends_on` edge to the settled-read story, with the shared files as its reason.
   - The settled-read story's own "Shares files with" should also name the regions (`tests.rs:1746-1799`, `store_cost_tests.rs:164-166` against the five tests). It currently names only the two files.

Counts:
- **Surfaces:** all 5 items are placed as cited. The store-tests story is placed by test names found with `git grep`.
- **Unplaced:** none.
- **Reads traced to a producer in the set:** 2 of 2 recorded by a `depends_on` edge.
  - Acceptance 7 reads the `metadata-store-er-0.32.0` fixture, produced by `story:entity-runtime-0330-records-name-definition-by-digest`.
  - Acceptance 11 reads the `blob_bytes_per_invoke` print, produced by that same story and by `story:read-cost-test-reports-bytes-per-read`.

What you read: 5 artifacts, by `aep plan artifact show` on the settled-read, store-tests, windows-host, entity-runtime-pin and read-cost stories, plus the connections story in both worktrees. I also ran `aep plan artifact relations`, `graph` and `waves` (output scanned only for this story). I compared the connections worktree's `git diff` and `git diff -U0` (hunks) per file, and ran `git grep` over `registry_uses`, `ReadUse`, `capture_*` and `simulate_process`. The branch is on 3d38f77b99f.

What I could not establish:
- **Connections story in this store:** story:connections-list-answers-through-the-owner is a draft here with no scope (revision 1). In its own worktree it is active with a scope (revision 27). Any wave plan built in this store treats it as unassessed until it merges.
- **Windows story:** story:windows-host-job-releases-the-cli-binary has no scope field. Its body cites `.github/workflows/windows-host.yml`. `apps/connectors/tests/mcp_stdio_serves.rs` comes only from your brief (not cited in the body). Neither path is in this story's scope, so I found no collision.
- **Connections story, own scope:** its worktree diff edits `website/docs/reference/contracts/cli.md`, which its scope omits. That is its own defect and is outside my lane.
- **Read-cost story:** story:read-cost-test-reports-bytes-per-read is outside the set. It also edits `read_invoke_cost_by_store_size` in `store_cost_tests.rs`. This story's `depends_on` edge already orders them, and I did not assess it further.
- **Owner process count:** two owner processes on one state directory, and whether the mutation executor runs outside the owner, are open questions the story already lists. Both are design-lane questions.

```findings
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 127
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Design keys the table by the simulated process, but process() in crates/connectors-host/src/local/metadata/er.rs:636-640 is private and metadata.rs:124 re-exports only the setter, so reading the key needs a new accessor in er.rs and metadata.rs, neither of which is in the scope; add both files with the region and say whether exit_process clears the table (cited)"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 129
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design says no file under owner/ changes while Shares files with line 211 and acceptance 10 change two owner test files; production owner code keeps its Registry signatures (supervisor.rs, transport.rs, execution.rs, owner.rs), so reword to no production file under owner/ plus the two test files (cited)"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 210
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Shares section leaves out website/docs/reference/contracts/cli.md, which story:connections-list-answers-through-the-owner edits (hunk at 1393) and this story regenerates; both add a new Unreleased and Changed heading at CHANGELOG.md line 3; docs/local-er-metadata.md numbers after :270 shift by 2 if that story lands first; 'the second to merge rebases' is no order for a generated file, so name the mirror and the CHANGELOG heading and say the second merger regenerates with connectors-docs generate (cited)"
- file: .engineering/planning/story/store-tests-end-within-60s-under-load.md
  line: 7
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the store-tests story has no scope and no depends_on edge, its body does not mention story:settled-read-uses-leave-the-store, yet it names tests in registry/tests.rs (:787, :1463, :2061), registry/store_cost_tests.rs:529 and tests/metadata_reopen_adversary.rs:175 that the settled-read story rewrites (grown_stores, seed_recorded_events) and whose read cost it changes; the order 'runs after this story' exists only in the settled-read body at line 204, so add the three files as scope and a depends_on edge to the settled-read story with the shared files as its reason, and give the regions in the settled-read Shares entry (cited)"
```
