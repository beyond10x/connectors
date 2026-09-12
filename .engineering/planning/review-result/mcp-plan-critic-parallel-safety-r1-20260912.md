---
format: aep.planning-md/1
id: review-result:mcp-plan-critic-parallel-safety-r1-20260912
kind: review-result
status: active
title: Parallel-safety critic, MCP decomposition, round 1
relations:
- reviews: story:mcp-specification-pin
- reviews: story:mcp-domain-model
- reviews: story:mcp-profile-selection-matrix
- reviews: story:mcp-outbound-connection-lifecycle
- reviews: story:mcp-outbound-auth-lifecycle
- reviews: story:mcp-outbound-invocation-results
- reviews: story:mcp-inbound-local-binding
- reviews: story:mcp-inbound-capability-projection
- reviews: story:mcp-inbound-mutation-replay
- reviews: story:mcp-inbound-cloud-profile
- reviews: story:mcp-composition-provenance
- reviews: story:mcp-cli-journey-discovery-contract
revision: 1
---
approve

**What I read:** All 12 `story:mcp-*` bodies in full (`aep plan artifact show story:<id>`, 12 calls) plus the store's own `aep plan artifact waves --kind story --status draft` (7 waves, 6 collisions, all inferred). Cross-checked the on-disk precedent for the pattern the store flagged: `find . -type d -name scenarios` and its contents, to confirm whether a shared `scenarios/` directory carries a manifest/index file that two writers could conflict on.

**Surface count:** 12 cited, 0 inferred, 0 unplaceable — every story's own `## Scope` section (rung 1 of the ladder) names its file paths in backticks directly in the body, which supersedes the tool's blanket `(inferred)` label on every wave/collision entry; the tool guessed where the text already answered.

**Collision check.** The store's 6 flagged collisions reduce to two triads sharing a directory, not a file:
- `adapters/mcp/contracts/server/v1alpha1/scenarios/` — `story:mcp-inbound-local-binding`, `story:mcp-inbound-capability-projection`, `story:mcp-inbound-mutation-replay`
- `adapters/mcp/contracts/client/v1alpha1/scenarios/` — `story:mcp-outbound-connection-lifecycle`, `story:mcp-outbound-auth-lifecycle`, `story:mcp-outbound-invocation-results`

Each of the six bodies names the other two sharers by id and states which distinct file category it adds ("session lifecycle" vs. "projection/error-mapping" vs. "mutation/lost-reply"; "lifecycle" vs. "credential-state" vs. "result/error"), and each says no story edits another's file. On-disk precedent (`contracts/sessions/v1alpha1/scenarios/`, `contracts/operations/v1alpha1/scenarios/`, etc.) confirms this repo's `scenarios/` directories hold only individual leaf files with no index/manifest — so concurrent creation of distinct files there is not a git merge conflict. This is a legitimately shared, admitted, named directory, not an unnamed file collision.

Two other shared-file cases (`apps/connectors/spec/compatibility.json`, `contracts/cli/v1alpha1/semantics.md`) are each touched by only one story in this 12-item set (`story:mcp-cli-journey-discovery-contract`); both bodies say so and name the outside owner. `adapters/README.md` is touched by exactly one story in the set (`story:mcp-specification-pin`), and `story:mcp-profile-selection-matrix` explicitly disclaims touching it. No two stories in this set target the same file without saying so, and no story's Scope names a surface broad enough to collide with everything.

**What I could not establish:** none — every one of the 12 stories carries an explicit `## Scope` section naming concrete new (or singly-owned) paths.

**Out of my lane, not affecting this verdict:** whether the three-way scenario-directory split is the *right* split (design critic's question), and whether `contracts/README.md`'s family index should be registered by one of these stories but isn't listed in any Scope section (scope critic's question — I did not treat it as a collision since no story claims to write it).

```findings
[]
```
