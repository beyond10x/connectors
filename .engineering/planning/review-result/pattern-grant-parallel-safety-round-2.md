---
format: aep.planning-md/3
id: review-result:pattern-grant-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


Round 1 findings:

1. **Scope: resolved.** Frontmatter lines 10-42 now list 16 cited paths. `ess/domains/cli.yaml` and `apps/connectors/spec/cli.yaml` are left out as "not changed, to confirm" (line 221), which is consistent.
2. **Order and shared files: resolved.** Lines 341-352 name the files shared with the two 0.77.0 stories, and `Cargo.lock` and `inputs.json` with `er-0331-pin`. The second to land rebases, regenerates last and adds its CHANGELOG line afterwards. The line saying it shares no file with `story/read-uses-in-memory` matches what I found.
3. **Stale line citations: resolved.** The Design header says the lines are re-read at the merged 0.77.0 head, and `standing.rs` now carries lines in Files.
4. **Imprecise Files entries: resolved.** `inputs.json` is cited as digesting `ess/domains`. Both test files are listed, and `standing_grant.rs` is marked as created by 0.77.0. `apps/connectors/spec/cli.yaml` is now "not changed, to confirm", and nothing changes in the result type (line 132).

New files:

- **Work in flight:** neither the 0.77.0 tree nor `read-uses-in-memory` nor the pin touches `crates/connectors-core/Cargo.toml`, `crates/connectors-core/src` or `crates/connectors-catalog/src/inventory.rs`.
- **Unnamed collision (new finding):** the unmerged branch `fix/258-gmail-control` (tree `fix-258c`) edits `crates/connectors-core/src/lib.rs` (a doc comment near line 50) and `reason.rs`. The story's new function needs a `pub mod` line in `lib.rs`, which neither Files (lines 203-204) nor the scope names. The scope instead names the whole directory `crates/connectors-core/src` (line 32), which collides with everything under it. The hunks differ, so the textual merge is likely clean, but the surface is unnamed.
- **Fix:** name the new file, name `crates/connectors-core/src/lib.rs`, and narrow the scope to those two. Say that `fix-258c` shares `lib.rs`, or that the two are ordered.

Not in your list of in-flight work, so it is not part of the verdict's premise.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 203
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new function in crates/connectors-core/src needs a pub mod line in crates/connectors-core/src/lib.rs, which Files does not name, and the scope names the whole directory crates/connectors-core/src (line 32); the unmerged branch fix/258-gmail-control (tree fix-258c) edits lib.rs (doc comment near line 50) and reason.rs; name the new file and lib.rs in Files and the scope, drop the directory path, and say that fix-258c shares lib.rs (different hunks) or order the two"
```
