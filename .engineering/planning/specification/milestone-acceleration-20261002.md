---
format: aep.planning-md/3
id: specification:milestone-acceleration-20261002
kind: specification
status: draft
title: 'Next Connectors milestones: sustained reads, provider acceptance, MCP'
relations:
- informed_by: initiative:complete-local-connectors
- informed_by: story:metadata-invoke-cost-flat-in-store-size
revision: 3
---
## Purpose and authority
Interactive planning on 2026-10-02 continuing Claude session 09b54d3f-8dac-49eb-9335-b0fa6e55f989. A scheduling and acceptance proposal under initiative:complete-local-connectors, not implementation approval or replacement scope. Existing stories remain draft. No approval bypass.

## Reconciled baseline and v0.24.0 receipt
Remote main and peeled v0.24.0: c7a9d5b1db1ff4e3b2af568db19c6eaa60d79d90. PR #77 merged with all four checks successful. Its Rust gate ran 02:03:10–02:25:42 UTC (22m32s); release published 02:27:20 UTC on 2026-10-02.
Verified through git ls-remote, gh pr view 77 and gh release view v0.24.0:
https://github.com/beyond10x/connectors/releases/tag/v0.24.0
https://github.com/beyond10x/connectors/pull/77
The previous session reported local MSRV verification: 1180 passed, zero failed; this session did not rerun it. The release ships catalog parameter withholding, missing-state setup and measurement tests. Growing-store performance remains broken.
https://github.com/beyond10x/entity-runtime/issues/51 is OPEN with no comments at inspection. Measured metadata time: 578ms/55 events, 12.4s/601, 40.2s/1203; execute_batch dominates, host-only work 0.4%. Optimizing unrelated host code cannot remove that bottleneck.
Primary has an unrelated untracked .agents/ directory, preserved. No prior linked Connectors trees remained.

## Milestones and exit evidence
| Milestone | Existing owners | Exit evidence |
| --- | --- | --- |
| M1: sustained reads | story:metadata-invoke-cost-flat-in-store-size and its upstream blocker; bridge/clock stories below | Exact published upstream adoption, bounded 600/6000-event measurements, about 700 fixture-backed invokes without admission timeout or unknown batch outcome, unchanged corruption/replay guarantees; then an explicitly authorized consumer Jira rerun. |
| M2: trusted first three providers | initiative:complete-local-connectors; story:catalog-cli-journeys; story:ignored-suites-have-a-runner; story:kubernetes-spec-service | Catalog-child approval/settlement/lost-response/crash/same-key/revocation journeys; dedicated Kubernetes/PostgreSQL sandbox evidence; all selected C06/C09/C12/C14/C15/C16 obligations accounted for, including open process-family/write cases. Fixtures never substitute for sandboxes. |
| M3: MCP runtime | epic:mcp-contracts and existing children | Reviewed outbound auth/results and inbound projection/replay/CLI/composition; model unresolved caller/process relations before runtime decomposition; prove selected versions, tools/resources/prompts, cancellation and no duplicate uncertain effects. Reconcile ../mcp actual HEAD before planning its changes. |
| Later | Existing initiative phases 6–7 and catalog/Google/HubSpot epics | Remaining providers, C20/C21/C22, compatibility and reproducible distribution stay required. No broad provider expansion before sustained reads work. |

M2 acceptance preparation may overlap the upstream wait; independent MCP contract work may overlap too. Runtime phase order and parent obligations remain unchanged. Inbound local stdio and outbound HTTP can progress independently of outbound child ownership; neither completes the whole MCP epic. Cloud caller isolation and outbound stdio remain decision-gated.

## Next batch proposal
Prepare three lanes together in separate managed trees:
1. story:bridge-drop-waits-for-dispatched-batch: metadata safety correction; record an upstream blocker if no bounded safe shutdown guarantee is available.
2. story:sql-fixture-accepts-stray-connections: reproduce and isolate interference, preserving rejection of extra SUT sessions.
3. story:ignored-suites-have-a-runner: expose acceptance coverage and missing prerequisites.

Then story:registry-clock-outside-shared-batches, after bridge cleanup and serialized against upstream metadata adoption. One bounded experiment, adopted only against its numeric and invariant checks, otherwise rejected with evidence. A mitigation never silently clears the upstream release/pin requirement.
These four are the review set, not bulk promotion or approved dispatch. Typed scope and depends_on drive aep plan artifact waves --status draft; other stories in its global output are not selected here. Its exact-string scopes also need manual directory/file-intersection review.
story:catalog-cli-journeys is the next M2 unit; scope it against the current tree before concurrent scheduling. MCP siblings share contract/scenario paths; different titles do not prove independence.

## Decisions and prerequisites
Keep the three decision blockers open pending answers: inbound caller connection allowlists; ownership of configured outbound MCP children; admission of a reusable external-process family for Helm/exec/copy/tunnels. Record exact ownership, cardinality and revocation in ESS before dependent runtime work. An allowlist preference alone does not settle every relation.
Google OAuth-client and Jira/Confluence/HubSpot sandbox blockers remain: configured integration access does not prove dedicated sandbox access. Recheck the Kubernetes protocol-loading refusal using the selected AEP before treating an upgrade as its resolution.
No cross-repository source edits, consumer repins, deployment or messages to other sessions are included.

## Faster delivery with the same checks
For each usable implementation batch, include intended version, changelog, affected guides and pre-release AEP state in its PR before final checks. Run one final local gate with --msrv and affected website checks while required PR checks run. Merge only after both pass and verify the merge tree matches the checked candidate. Then tag, publish and verify the source release.
This removes a separate version-only PR: one observed approximately 22-minute CI interval saved, not a measured future guarantee. Record release success only after publication, carrying the receipt into the next already-needed change; never pre-record success or silently drop a pending receipt.
Use one integration build lane, task-owned output, two Cargo jobs and disk inspection. Targeted red/green tests belong in unit trees; preserve a warm integration target for corrections and rerun full checks when changed inputs/failures require them. Do not share mutable target directories among incompatible concurrent builds.
The earlier sccache hang is historical, not a current diagnosis. Check health once; use task-local settings without the wrapper if it hangs, never kill another session's server.
Keep toolchain upgrades, branch retirement and review cleanup off this critical path. Project pins remain ESS 0.45.0 / AEP 0.65.0; ambient upgrades do not change them.

## Review and next action

Two review rounds are complete. aep:plan-critic-acceptance raised four acceptance-shape findings in round 1; all four were fixed and have review_outcome records. Round 2: acceptance, design, scope and parallel-safety all approve, zero remaining findings. Records: review-result:milestone-{acceptance,design,scope,parallel-safety}-r{1,2}-20261002.

Host adaptation: three independent read-only subagents ran the named acceptance/design/scope procedures; the coordinator ran parallel-safety concurrently because only four total agent slots were available. Thus the fourth perspective was not independent of the author. The skill's Sonnet model pin is unavailable in this host; critics inherited the session model. No claim of four independent Sonnet reviewers is made.

The global draft wave calculation places the first three selected stories in wave 1 and the clock experiment in wave 5 because unrelated backlog stories also compete for its paths. This is not four extra required batches: within this selected set only bridge-to-clock ordering and its two declared metadata collisions apply; do not dispatch the unrelated global-wave items automatically.

Interactive run; no approval bypass records. The three requested product decisions received no answer by handoff and remain open. Next owner: Connectors coordinator, beginning with this batch proposal and upstream issue #51. No runtime code changed or provider effects invoked.

Planning changes remain local on plan/milestones-20261002 in managed tree connectors-plan-20261002; publication is not inferred from this planning request. The tree is archived for recovery and retained for review. Carry its changes into the next authorized source/planning PR; no separate release or gate wait is needed merely to read the plan.

## Delivery authorization and active work — 2026-10-02

approval-record:milestones-delivery-20261002 records the operator's instruction to implement all selected work and cut a new version. The previous planning-only boundary is superseded. The bridge, SQL fixture and ignored-suite stories are active; clock experimentation remains sequenced afterward. The initial unit scopes remain unchanged. The full objective includes first-three-provider acceptance and MCP and is not complete merely when this first batch ships.
