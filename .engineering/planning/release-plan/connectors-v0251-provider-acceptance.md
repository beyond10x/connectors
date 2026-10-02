---
format: aep.planning-md/3
id: release-plan:connectors-v0251-provider-acceptance
kind: release-plan
status: active
title: 'Release 0.25.1: verified provider acceptance and precise SQL refusals'
relations:
- informed_by: specification:wave-20261002d-provider-acceptance
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-02T16:50:11Z", actor: "human:timo", revision: 2}
---
## Outcome and standing authorization

Prepare patch release0.25.1 for the verified provider-acceptance handoff under
approval-record:milestones-delivery-20261002. Remote main was verified at
b1f432766057fb8b7db60736363926cc44fe0db8 and the0.25tag namespace contains only
v0.25.0 on2026-10-02. Recheck before tagging. Patch scope reflects the exact0A000
SQL refusal correction and acceptance/tooling/documentation changes; it introduces
no new provider capability or protocol version.

## Candidate scope

Reviewed PostgreSQL sourcec3e20e1df82ccd4bc33b25c2943d3059ae0b51e1 and Kubernetes
source8ce90cb6d0f765a78f4273ca0ccade1ed7253a35 are integrated locally. Include current
GitLab18-read evidence and explicit initial revalidation refusal, exact ignored
runner classifications/prerequisites, and catalog lifecycle/mutation acceptance
only after its remaining helper/recovery/fault assertions and adversary review
complete. The rejected bounded clock experiment remains a recorded decision;
production clock source is unchanged and Entity Runtime51/M1 remains open.

Catalog is still executing. This plan is not release readiness. Do not describe
unexecuted catalog variants or failed fault injection as passing. If a requested
obligation proves infeasible, record its specific blocker and narrow the released
scope explicitly while retaining the unmet goal; never silently close it.

## Owned delivery surfaces and checks

Root owns Cargo.toml workspace version, generated Cargo.lock workspace entries,
README.md, CHANGELOG.md, docs/development.md, website/docs/adapters/{gitlab,kubernetes,sql}.mdx,
public evidence, generated owning projections/reference data as required, and AEP.
Provider workers retain their test/native guide scope until frozen. No source
version changes until the release candidate's final scope is established.

Required verification: pinned ESS/AEP validation and projection/conformance checks,
repository gate with MSRV, selected provider evidence with unchanged-input reuse
explicit, compiled ignored inventory with no unknown entries, affected website
checks after npm ci. Test/docs-only changes do not justify unrelated live reruns.
Capture full AEP output including historical warnings. Preserve exact candidate
source/tree identity and bot author+committer for every direct commit.

Publish branch through b10x-gates bot; create/merge PR through bot App API, required
checks green; verify exact remote main merge/tree before immutable annotated tag.
Verify tag workflow, GitHub release author/latest identity and required source
archives. Report queued until all source-release requirements succeed. Docs
publication remains asynchronous; no Atlas/Website delivery/consumer pin work.
Managed worker commits must be published before finish; cleanup uses reviewed
exact worktree ids and recovery evidence. No force/manual removal.

## Completion boundary

This source release cannot close the parent initiative, remaining Kubernetes/Helm
semantics, sustained ER workload, the unexplained isolated GitLab revalidation
observation or MCP delivery. The three product decisions stay open. This is one
release artifact over already-reviewed work, not a new multi-story decomposition;
a new decomposition critic panel is not applicable.
