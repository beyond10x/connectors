---
format: aep.planning-md/1
id: review-result:mcp-plan-critic-design-r1-20260912
kind: review-result
status: active
title: Design critic, MCP decomposition, round 1
relations:
- reviews: story:mcp-cli-journey-discovery-contract
- reviews: story:mcp-domain-model
- reviews: story:mcp-specification-pin
revision: 1
---
**needs-revision**

story:mcp-cli-journey-discovery-contract — its own acceptance requires the discovery contract to state, "per selected capability, the identifier an agent reads and the exhaustive error set it may receive, mapped onto `connectors.service_wire.ErrorCode`" — content that only `story:mcp-inbound-capability-projection` (the capability identifiers) and `story:mcp-outbound-invocation-results` / `story:mcp-inbound-mutation-replay` / `story:mcp-outbound-auth-lifecycle` (the per-direction distinct failure states) establish, yet none of those four is in its `depends_on` list (only `mcp-outbound-connection-lifecycle`, `mcp-inbound-local-binding`, `mcp-profile-selection-matrix` are) — `aep plan artifact show story:mcp-cli-journey-discovery-contract`, section "The error contract is the agent-facing half."

**What I read:** the epic (`epic:mcp-contracts`) and all 12 `story:mcp-*` bodies in full via `aep plan artifact show`; `aep plan artifact relations` for edge vocabulary; `aep plan artifact graph --format json` filtered to these 12 stories plus the epic, the two `decision-blocker:mcp-*` artifacts and `vision:independent-contract-adapters` (every `relations` block on each, 17 `depends_on` edges among the 12 stories, plus their `decomposes`/`serves`/`informed_by` edges, and the 2 `blocks` edges from the decision-blockers into the epic — I walked one hop outside the named set in each direction and no further); `aep plan artifact waves --kind story --status draft` (7 waves, 6 collisions, exit 0 — no cycle reported); `aep plan artifact validate` (reports only pre-existing prose-only review-results elsewhere in the store, nothing about this set, so relayed and not repeated as a finding).

**What I could not establish:** whether `story:mcp-domain-model`'s `depends_on story:mcp-specification-pin` reflects a real content need or a process preference — its body cites `ess/1` documents and the initiative, never the pinned evidence file's contents directly — I judged it defensible given the epic's own "pin, then author entities" framing but did not find a citation proving domain-model's assertions actually rest on the pinned revision's bytes; undecided, not filed as a finding. I also did not check whether the six directory-level collisions `aep plan artifact waves` reports (the inbound trio sharing `.../server/v1alpha1/scenarios`, the outbound trio sharing `.../client/v1alpha1/scenarios`) reflect a genuine split-abstraction problem or a legitimate one-directory-many-files pattern with clearly disjoint scopes as each story's body claims — that determination of whether the two can be worked at once is `plan-critic-parallel-safety`'s lane, out of mine, and I note it here rather than let it set my verdict.

```findings
- file: aep plan artifact show story:mcp-cli-journey-discovery-contract
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: its discovery contract must state, per capability, the identifier an agent reads and the exhaustive error set it may receive — content that story:mcp-inbound-capability-projection, story:mcp-outbound-invocation-results, story:mcp-inbound-mutation-replay and story:mcp-outbound-auth-lifecycle establish, and none of them appears in its depends_on list
```
