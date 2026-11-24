---
format: aep.planning-md/3
id: review-result:windows-bounded-read-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, story:windows-mcp-tool-call-answers-timeout
relations:
- reviews: story:windows-mcp-tool-call-answers-timeout
revision: 1
---
Recorded by the session that ran the panel: the brief asked for the members severity, where, finding and fix; the store reads file, category, severity, verdict, origin and message. Each entry below keeps its words in `message`; file, category, verdict and origin were set at recording.

verdict: needs-revision

The story can be worked beside the other three, with one blocker: the `ess/domains/transport.yaml` edit changes a committed digest that no scope line names.

Overlap by file. Line numbers for `connections-through-owner` are the new numbering from its `git diff`:

- **`owner/transport.rs`, `connections-through-owner`:** its hunks are 1094-1113 (`Client::observe`), 2810 (one match arm) and 3137-3143 (one `action_value` arm). `watch_cancel` is at 2479-2536 in the base and 2499-2556 there, so the regions do not overlap. That checkout adds no `use` lines at the top. The rebase is clean.
- **`CHANGELOG.md`:** `read-uses-in-memory` and `connections-through-owner` each add a `## Unreleased` block at line 3, so three blocks will collide there. The conflict is textual and easy to resolve. Acceptance 8 assumes `## Unreleased` exists, but the 0.76.0 base has none.
- **`mcp/projection.rs`, `mcp/mod.rs`, `runtime/bounded.rs`, `runtime/channel/windows.rs`, `owner/supervisor.rs`, `private-bounded-read.md`, `docs/local-mcp-stdio.md`, `website/docs/connectors/mcp.md`:** none of the other three checkouts has changes in them.
- **`standing-grant-local`:** it has no diff yet. Its named files, `execution.rs`, `owner/mutation.rs`, `standing.rs`, `invoke.rs` and `operations.rs`, are not in this story's scope. This story only reads `mutation.rs:259-271` and does not edit it. Its possible edits to `owner/transport.rs` (a new request kind or `action_value` arm) and to `ess/domains/transport.yaml` are unassessed.

"Shares files with":
- `story:settled-read-uses-leave-the-store` is correct. "No other file" is true.
- `story:connections-list-answers-through-the-owner` is correct for `transport.rs` and `CHANGELOG.md`.
- `standing-grant-local` is missing. It is a draft with no story id I could check.

```findings
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "where: Design, last bullet (\"The comment at ess/domains/transport.yaml:18 says the same. No declaration in ESS changes.\"), and scope finding: crates/connectors-build/generated/metadata-emissions/inputs.json records the sha256 of every file under ess/ (line 140 for transport.yaml). docs/development.md:66-68 says the_committed_metadata_emissions_are_current fails when it differs. A comment-only edit therefore changes the digest, and regenerating takes three parts of up to 30 minutes each. No scope line names inputs.json. The claim that nothing in ESS changes is true for declarations but false for the digest. standing-grant-local also edits ESS domains, so it will rewrite the same file. fix: Either drop the transport.yaml comment edit and the sentence about it (the contract carries the rule), or add `crates/connectors-build/generated/metadata-emissions/inputs.json` and the regenerated part files to the scope. In that case add an acceptance line \"the_committed_metadata_emissions_are_current passes\" and name the `slot --timeout 30m` regeneration."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Shares files with finding: standing-grant-local is not mentioned. If it keeps `ess/domains/transport.yaml` or `owner/transport.rs` in its scope, the pair collides on that file or on inputs.json. fix: Add \"standing-grant-local (draft, no story id yet): may change ess/domains/*.yaml, so inputs.json collides with this story if the transport.yaml edit stays; `owner/transport.rs` is not known to be touched. Re-check when its scope is set.\""
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Scope finding: Unassessed files. Acceptance 2 needs a test in apps/connectors/src/mcp/ and the scope names only mod.rs (inferred), not tests.rs. Design also edits the `use` block and `governed_client_tests` gating in transport.rs. These cannot be called safe. fix: Add `apps/connectors/src/mcp/tests.rs` (or the real test file) to the scope as cited. Name the transport.rs `use` lines as a region."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: parallel-safety
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "where: Shares files with, CHANGELOG.md finding: Both implemented stories add a `## Unreleased` heading at line 3. The base has only `## 0.76.0` there. fix: Add \"the entry goes under `## Unreleased`, created by whichever story merges first; the later one rebases onto it.\""
```

Merge order with the least rework: 0.76.0, then `read-uses-in-memory`, then `connections-through-owner`, then this story. Each of the first two already has uncommitted work, and this story is last in the order because it is the slowest to verify (about 25 minutes per Windows push). If the `transport.yaml` comment edit is dropped, this story and `standing-grant-local` do not collide.
