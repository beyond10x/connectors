---
format: aep.planning-md/3
id: decision-blocker:v1-remote-branch-retirement
kind: decision-blocker
status: open
title: Nobody has decided which of 32 unmerged remote branches may be deleted
relations:
- blocks: story:retire-unmerged-remote-branches
revision: 2
---
# Nobody has decided which of 32 unmerged remote branches may be deleted

## The branches (read 2026-10-07)

`git fetch origin --prune`, then per branch: the tip's committer date, the tip, the commits not
reachable from `origin/main` (`git rev-list --count origin/main..<branch>`), and the commits
reachable from neither `origin/main` nor any tag (`git rev-list --count <branch> --not origin/main
<every tag>`). A branch with 0 in the last column loses nothing on deletion: every commit it holds
stays reachable from `main` or a tag. `main` is the v2 lineage and the v1 history is reachable
only through the v1 tags, which is why most counts in the third column are in the hundreds.

### 15 branches whose every commit stays reachable from `main` or a tag

| Branch | Last commit | Tip | Not in main | In neither main nor a tag |
|---|---|---|---:|---:|
| `bot/git-fetch-sessions-0.6.1` | 2026-09-05 | `7033b6b43` | 364 | 0 |
| `docs/gitlab-provisioning-gap` | 2026-09-05 | `4e0eb26af` | 377 | 0 |
| `feat/identity-0-5-6` | 2026-09-03 | `36dbc38b3` | 308 | 0 |
| `feat/repository-workspace-datasources` | 2026-09-01 | `dbf4957ee` | 272 | 0 |
| `feat/repository-workspaces` | 2026-09-01 | `6b70b7f99` | 275 | 0 |
| `fix/claude-oauth-recovery` | 2026-09-07 | `34fabba52` | 452 | 0 |
| `fix/generated-service-remediation-0-7` | 2026-09-07 | `1de85e9d4` | 453 | 0 |
| `fix/git-v2-smart-http-preamble` | 2026-09-06 | `db6d7a930` | 381 | 0 |
| `impl/rate-limit-in-the-protocol` | 2026-09-06 | `014ed09fc` | 439 | 0 |
| `publish/cli-auth-oauth` | 2026-09-07 | `6e9c26716` | 449 | 0 |
| `release/0.7.0` | 2026-09-07 | `f7b0b08aa` | 451 | 0 |
| `release/bounded-source-reads-0.7.1` | 2026-09-07 | `52c01370d` | 457 | 0 |
| `wave/cli-ten-slack-first` | 2026-09-06 | `9a254a50a` | 404 | 0 |
| `wave/cli-ten-slack-first-resumed` | 2026-09-06 | `756b209d1` | 416 | 0 |
| `wave/cli-verified-first-batch` | 2026-09-06 | `9dcdb5863` | 420 | 0 |

### 17 branches holding commits that neither `main` nor a tag reaches

| Branch | Last commit | Tip | Not in main | In neither main nor a tag | Tip subject |
|---|---|---|---:|---:|---|
| `archive/wt-38354a193753` | 2026-09-15 | `ed8966bcf` | 535 | 96 | chore(archive): uncommitted state of wt-38354a193753 at retirement |
| `archive/wt-4f1de73d0685` | 2026-09-06 | `bf7f2049c` | 497 | 58 | test: retain final OAuth authoring shutdown and expiry regressions |
| `archive/wt-87a5e72befe7` | 2026-09-15 | `2bb2ce151` | 507 | 38 | chore(archive): uncommitted state of wt-87a5e72befe7 at retirement |
| `feat/generic-endpoint-discovery` | 2026-09-07 | `5a432fc64` | 497 | 28 | plan: record independent review round one and its outcomes |
| `request/channel-scoped-slack-replies` | 2026-09-08 | `fe19e60fc` | 388 | 10 | plan: request bounded discovery of recent channel replies |
| `request/nested-file-admission-20260909` | 2026-09-09 | `330bc1e6b` | 388 | 10 | plan: report nested repository file admission refusal |
| `archive/wt-2a4683b1bc2e` | 2026-09-07 | `cd275bf6a` | 477 | 8 | monitoring: separate endpoint inventory regression fixtures |
| `archive/wt-7b3b2b2ffd88` | 2026-09-07 | `92b36b2f1` | 477 | 8 | endpoint: compose declared websocket channels against admitted routes |
| `fix/hosted-catalog-endpoint-bindings` | 2026-09-07 | `2b4343551` | 460 | 7 | catalog: preserve declared legacy output schemas |
| `pre-contracts` | 2026-09-15 | `b0cbd9d64` | 476 | 7 | docs: design 21 names a reachable pin and counts its consumers once |
| `feat/hosted-integration-admin` | 2026-09-02 | `5405e7c60` | 295 | 4 | release: cut connectors 0.4.3 |
| `compat/claude-oauth-recovery` | 2026-09-07 | `43e553cd1` | 383 | 2 | chore: retain reviewed public source digest scan baseline |
| `integrate/ess-evolution-phase-e-20260923` | 2026-09-23 | `d3ad422c6` | 2 | 2 | fix(core): read and digest JSON numbers alike in both serde_json builds |
| `compat/claude-oauth-0-7-source` | 2026-09-07 | `976c4f134` | 451 | 1 | fix: distinguish subscription OAuth failures and expired flows |
| `next` | 2026-09-14 | `fa3105c8a` | 195 | 1 | Close the v0.11.0 release plan against the page it shipped |
| `plan/ready-platform-cli-waves` | 2026-09-06 | `05fcd317e` | 378 | 1 | plan: mark platform and cli stories ready |
| `recovery/cli-review-link-transaction-20260906` | 2026-09-06 | `7e8703193` | 409 | 1 | plan: retain failed review-link recording transaction |

15 + 17 = 32. Counts in the last column are per branch; two branches can share the same unique
commits, so the columns are not summed.

### Not part of the 32

Branches created after 2026-09-30 are current work and are not in this question:
`feat/cli-json-answers-as-json`, `unit/mcp-local-runtime-20261003`,
`unit/mcp-local-runtime-20261003-rebased`, `wave/20261006b` and `wave/20261006d`.

## Options

| Option | What it does | Cost |
|---|---|---|
| A | Delete the 15 branches with 0 unique commits through the bot App. For the 17 with unique commits, push an annotated tag `archive/<branch>` at each tip, then delete the branch. | 17 tags added to the tag namespace; nothing becomes unreachable on GitHub |
| B | Delete the 15 with 0 unique commits. Keep the 17 as branches. | 17 stale branches stay; nothing is lost |
| C | Write one `git bundle` per branch outside GitHub, then delete all 32. | The unique commits survive only where the bundles are kept |

## What would clear this

An operator decision naming one of the options, or a keep/archive/delete call per branch.
