---
format: aep.planning-md/3
id: review-result:pattern-grant-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


Findings, each also in the block below:

1. `story:standing-grant-binds-a-pattern-or-a-value-set` carries no machine-readable scope. Its frontmatter (lines 1-11) has no `scope:`. `aep plan artifact scope story:standing-grant-binds-a-pattern-or-a-value-set` answers "nothing to scope; name a path" and offers no read mode. Both 0.77.0 stories carry scope, so a wave over the set cannot see this one. The fix is to record the paths. Cited: `CHANGELOG.md`, `Cargo.lock`, `crates/connectors-host/Cargo.toml`, `crates/connectors-host/src/local/standing.rs`, `crates/connectors-host/tests/http_standing_grant_adversary.rs`, `apps/connectors/tests/standing_grant.rs`, both `semantics.md`, `contracts/service/compatibility.md`, `contracts/service/local-mutations.md`, `docs/running-services.md`, `docs/local-approvals.md`, `ess/domains/standing_approvals.yaml` and `ess/domains/cli.yaml`. Inferred: `crates/connectors-build/generated/metadata-emissions` and `apps/connectors/spec/cli.yaml`.

2. The story orders itself only after 0.77.0 (lines 219-224). It does not name the shared files, and the sibling stories do. The 0.77.0 stories share these with it, and it depends on them: `standing.rs`, `standing_approvals.yaml`, `ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml`, both `semantics.md`, `compatibility.md`, `local-mutations.md`, `docs/running-services.md`, `docs/local-approvals.md`, `CHANGELOG.md`. It also does not name the other in-flight work. The Entity Runtime pin shares `Cargo.lock`, because the pin's `entity-core` edits sit in the same `connectors-host` block where this story adds `regex` (line 153). The pin and the story's own `ess/domains` edits both feed `inputs.json`. `CHANGELOG.md` is touched by every release. `read-uses-in-memory` does not touch the story's files. I could not find a `cli.yaml` change on that branch and only took it from your brief; its story only cites `ess/domains/cli.yaml` at line 310. The fix is an "Order and shared files" section. It should say `Cargo.lock` and `inputs.json` are regenerated last on a rebased `main`, and that the `CHANGELOG.md` line goes in after the rebase. For the pin, the choice is an ordering edge naming the shared files or leaving the two apart. I did not choose.

3. The `S:` and `C:` line citations are against 43bda33b08d, and they are already stale (lines 62-64). The 0.77.0 work adds about 1,159 lines to `standing.rs`, and `TargetMember` has moved from `:131-136` to `:140`. The 0.76.0 and pin changes also shift `semantics.md` and `docs/running-services.md:251`. The fix is to say the lines are re-read at the merged 0.77.0 head. The Files entry for `standing.rs` (line 152) should give lines.

4. Some Files entries are not precise enough to assess (lines 142-158):
   - `inputs.json` is "inferred". It should be cited as digesting `ess/domains` plus the pin.
   - `apps/connectors/spec/cli.yaml` "does not change" is inferred, and the 0.77.0 tree is editing it now.
   - The Tests section names `apps/connectors/tests/standing_grant.rs`, which does not exist at 43bda33b08d and is untracked in the 0.77.0 tree. It also names `http_standing_grant_adversary.rs`. Neither is in Files, and the first is not marked as created by 0.77.0.

What I read: 3 stories and the `read-uses-in-memory` branch diff, from `standing-grant-local` via `git status`, `git diff`, `git grep` and `aep plan artifact scope`.

Surfaces, counted over the files in the story's Files list: 11 cited, 2 inferred and 0 unplaceable. I traced 4 acceptance reads to a producer: 4, 5 and 9 to `local-invoke`, 8 to `closes-the-input`. Three are recorded by a direct `depends_on` edge. The fourth is only transitive, through `local-invoke`, and I raised no finding on it.

Out of my lane: the claim that `apps/connectors/spec/cli.yaml` does not change is a design question.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 11
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "frontmatter has no scope: list, and aep plan artifact scope reports nothing to scope, so the story is unassessed for waves; record the cited paths (CHANGELOG.md, Cargo.lock, crates/connectors-host/Cargo.toml, standing.rs, the two test files, both semantics.md, compatibility.md, local-mutations.md, docs/running-services.md, docs/local-approvals.md, ess/domains/standing_approvals.yaml, ess/domains/cli.yaml) and the inferred ones (metadata-emissions, apps/connectors/spec/cli.yaml)"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 224
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "orders itself after 0.77.0 only and names none of the shared files (standing.rs, standing_approvals.yaml, ess/domains/cli.yaml, semantics.md files, compatibility.md, CHANGELOG.md) nor the Entity Runtime pin's Cargo.lock and inputs.json; add an Order and shared files section: regenerate Cargo.lock and inputs.json last on a rebased main, add the CHANGELOG line after the rebase, and for the pin choose an ordering edge naming the shared files or a split"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "S: and C: line citations are against 43bda33b08d and 0.77.0 plus the pin move them (TargetMember is already at standing.rs:140, not :131-136); say the lines are re-read at the merged 0.77.0 head and give lines for the standing.rs entry in Files (line 152)"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 156
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inputs.json and apps/connectors/spec/cli.yaml are inferred (finding rests on inferred surfaces), and apps/connectors/tests/standing_grant.rs (created by 0.77.0, absent at 43bda33b08d) and http_standing_grant_adversary.rs are in Tests but not in Files; cite inputs.json as digesting ess/domains plus the pin, confirm spec/cli.yaml after 0.77.0 merges, and list the test files marking the first as created by 0.77.0"
```
