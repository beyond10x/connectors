---
format: aep.planning-md/3
id: review-result:standing-grant-local-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-Safety critic, round 1, story:local-invoke-admits-a-keyed-write-under-a-standing-grant
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
revision: 1
---
verdict: needs-revision

**1. Overlap with other units** (line ranges taken from `git diff` in each checkout, all cited):
- `CHANGELOG.md`: all three in-work units and the 0.76.x releases insert at line 3 (read-uses +32, connections +16, windows +34 inherited from the pin). This story's base has no Unreleased section. The conflict is certain.
- `contracts/cli/v1alpha1/owner.md`: read-uses touches line 210 and connections touches 239-283. This story touches 147-149. No overlap.
- `contracts/cli/v1alpha1/semantics.md`: read-uses touches 2094 and connections touches 1378. This story touches 416-417 and 815-820. No overlap.
- `website/docs/reference/contracts/cli.md` (generated): connections changed ~1393. Read-uses will regenerate it later. Rebase and regenerate rather than merge.
- `crates/connectors-host/src/local/owner/transport.rs`:
  - Connections adds lines at 1094, 2810 and 3137.
  - Windows changes `watch_cancel`, around 2479-2536.
  - This story's Design reads 2640 and changes nothing there.
  - The scope line is "inferred", so it should say read-only or be dropped.
- `ess/domains/transport.yaml` comment: only the "Follows another story" section covers it.
- The windows checkout's own diff so far is only `projection.rs` +21, which this story does not touch.

**2. Generated files.** `inputs.json` digests `ess/domains/*.yaml`, `ess/components.yaml` and `ess/system.yaml`, plus the Entity Runtime crate versions. Read-uses, connections and windows change no `ess/` file, so none of them regenerates it. The pin series does: 0.76.0 rewrote its `digest` and `ess_crates` lines, and 0.76.1 will again. Order: implement code first, rebase on `main` after 0.76.1 (and after the Windows story, for the `transport.yaml` comment), then run the three-part regeneration once. A regeneration before that rebase is discarded.

**3. "Shares files with"**
- Windows "none, unless `transport.rs` moves": wrong. `CHANGELOG.md` is shared, and `transport.rs` does move.
- Connections-through-owner (active, in work) is missing. It shares `CHANGELOG.md`, `owner.md`, `semantics.md` and `cli.md`.
- The 0.76.0/0.76.1 releases are missing. They share `CHANGELOG.md` and `inputs.json`.
- Status of the six "Found by scope" stories:
  - `invoke-admission-queues-under-load`: shown as active, but it shipped in 0.75.0, the base of this tree. Nothing is in work on it.
  - `guarded-write-credential-refusal-says-repair`: draft. It shares `execution.rs` and `owner/mutation.rs`, so the overlap is real if it is ever started.
  - `timeout-refusal-says-how-long-it-waited`: draft, all scope lines inferred. It depends on the first story above and shares `ess/domains/cli.yaml`.
  - `definition-files-in-user-config` and `openrouter-inference-key-profile`: draft. They share `host/config.rs`, and the second shares `migrate.rs`.
  - `standing-grant-slot-released-before-dispatch`: draft, no scope. Its cited site is `server/v1alpha2.rs:654-664`, not `host/standing.rs`, so the "inferred" claim is unsupported.

**4. Unassessed files** (the Design changes them, no scope line names them):
- `local/service.rs`: the served-host `covering` call at 466 breaks if `covering` or `Grant` changes.
- `server/v1alpha2.rs`.
- `crates/connectors-host/tests/http_standing_grant_adversary.rs`: it calls `covering` and `parse`.
- `owner/mutation/tests.rs`: it matches `ApprovalCode`, which gains a variant.
- `local/owner.rs`: inferred as the place the owner reaches the grants path. Connections adds 358 lines there.

**5. Split.** I recommend one implementor, or the order C, then A, then B.
- C: `input_keys` in `standing.rs` plus `http_standing_grant.rs`.
- A: grant file, signature, the two commands, the config member.
- B: owner admission and the answer member.
- Collisions:
  - `standing.rs` is shared by all three.
  - `proof.rs` is shared by A (sign) and B (verify).
  - `config.rs` is shared by A and B.
  - `cli.yaml` is shared by A (commands) and B (`rate_limited`).
- B's acceptance 1 needs A's `grant-standing`, so B cannot go green before A. The story should record that order.

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Shares files with says none for story:windows-mcp-tool-call-answers-timeout and omits story:connections-list-answers-through-the-owner and the 0.76.0/0.76.1 releases; replace with: CHANGELOG.md is shared with the Windows story, connections-through-owner (also owner.md, semantics.md, cli.md) and releases 0.76.0/0.76.1 (also inputs.json); the Windows story changes transport.rs watch_cancel 2479-2536, not read here"
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no order is stated for the generated files; add: metadata-emissions inputs.json digests ess/domains and the Entity Runtime pin, so regenerate once, after rebasing on main with release 0.76.1 and the Windows story, never before"
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Found by scope lists story:invoke-admission-queues-under-load as a sharer but it shipped in 0.75.0 and is not in work, and the other five are drafts; say which are drafts, and drop story:standing-grant-slot-released-before-dispatch or change it to its cited server/v1alpha2.rs"
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design changes files no scope line names (unassessed): crates/connectors-host/src/local/service.rs, crates/connectors-host/src/server/v1alpha2.rs, crates/connectors-host/tests/http_standing_grant_adversary.rs, crates/connectors-host/src/local/owner/mutation/tests.rs, and inferred crates/connectors-host/src/local/owner.rs; add each as cited or inferred, or say it is not changed"
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope lists crates/connectors-host/src/local/owner/transport.rs as inferred while the Design says no transport change; write: read-only, not changed"
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "add a sentence that the story is one implementor's: standing.rs, proof.rs, config.rs and cli.yaml are shared by the grant-file, owner-admission and input_keys parts, and acceptance 1 reads the grant file the commands write, so the order is input_keys, then commands, then owner admission"
```

**What I read:** 11 artifacts (`aep plan artifact show` ×11), `git diff` in four checkouts, `rg` over `standing.rs` callers. Acceptance reads traced to a producer: 1 (acceptance 1 reads what `grant-standing` writes). It is inside this story, so no edge is needed. Placed: 36 scope lines, 26 cited, 10 inferred, 0 unplaceable.

**Could not establish:** the owner's path to the config (`owner.rs` or elsewhere), and the windows checkout's final changed ranges, since only `projection.rs` is modified so far.
