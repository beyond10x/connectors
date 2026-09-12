---
format: aep.planning-md/1
id: review-result:mcp-plan-critic-design-r2-20260912
kind: review-result
status: active
title: Design critic, MCP decomposition, round 2
relations:
- reviews: story:mcp-cli-journey-discovery-contract
- reviews: story:mcp-domain-model
- reviews: story:mcp-specification-pin
- reviews: story:mcp-inbound-local-binding
revision: 1
---
`approve`

No findings — zero blockers, zero warnings.

**Round-1 finding, closed.** `story:mcp-cli-journey-discovery-contract` now carries all four `depends_on` edges (`mcp-inbound-capability-projection`, `mcp-outbound-invocation-results`, `mcp-inbound-mutation-replay`, `mcp-outbound-auth-lifecycle`), confirmed via `aep plan artifact show story:mcp-cli-journey-discovery-contract`. Each is grounded in its acceptance text: "per selected capability, the identifier an agent reads" → capability-projection; "the exhaustive error set it may receive" → invocation-results, mutation-replay, auth-lifecycle (the three sources of the closed `ErrorCode` surface it enumerates). No edge is excess and none is still missing.

**Depth check — real, not sequencing preference.** Walked all 21 `depends_on` edges and all 24 `decomposes`/`serves` edges among the 12 stories (`aep plan artifact graph`, cross-checked against 12 individual `show` calls), plus the two external `decision-blocker → epic:mcp-contracts` `blocks` edges. No cycle: every edge points strictly backward through one topological order (`pin → domain-model → profile-selection-matrix → {local-binding, connection-lifecycle} → {capability-projection, cloud-profile, auth-lifecycle} → {mutation-replay, invocation-results} → {cli-journey, composition-provenance}`). The front is a genuine 3-long single-file chain, but its content is real, not preference: `mcp-domain-model`'s acceptance validates its nouns against "the pinned specification," and `mcp-profile-selection-matrix`'s body explicitly cites `mcp-domain-model`'s `UNMAPPED:` markers row-by-row (`aep plan artifact show story:mcp-profile-selection-matrix`, `## Domain relations`). Past that chain the graph fans out to width 2–3, so the set is not a queue in disguise — only its first three nodes are serial, and that seriality is textually justified.

**Domain-model → specification-pin, settled: real content need.** `story:mcp-domain-model`'s acceptance ties every relation not readable from an existing `ess/1` document "or the pinned specification" to an explicit `UNMAPPED:` marker — i.e. it must read the pinned, hashed specification to know which relations are answerable at all. This mirrors the repo's own established discipline for `kubernetes`/`loki` (pin exact source hashes, then model against them). Not a process preference.

**Concurrency answer for the coordinator** (`aep plan artifact waves --kind story --status draft`, filtered to this epic's 12):

| wave | stories | count |
|---|---|---|
| 1 | mcp-specification-pin | 1 |
| 2 | mcp-domain-model | 1 |
| 3 | mcp-profile-selection-matrix | 1 |
| 4 | mcp-inbound-local-binding, mcp-outbound-connection-lifecycle | 2 |
| 5 | mcp-inbound-capability-projection, mcp-inbound-cloud-profile, mcp-outbound-auth-lifecycle | 3 |
| 6 | mcp-inbound-mutation-replay, mcp-outbound-invocation-results | 2 |
| 7 | mcp-cli-journey-discovery-contract, mcp-composition-provenance | 2 |

Peak concurrency is 3 (wave 5); the first three waves are strictly serial (1 item each, content-grounded, not artificial). `story:kubernetes-spec-service` shares wave 1 by coincidence (different epic, no edge to this set) — not part of this count.

**Inbound-local-binding rewrite:** checked for split-abstraction against its sibling `mcp-outbound-connection-lifecycle`. The six deliverable-3 behaviours are specified once per direction (inbound in one, outbound in the other), each citing the same shared `connectors.sessions.Session` vocabulary rather than restating the other's half. No overlap with `mcp-inbound-capability-projection` or `mcp-inbound-cloud-profile`, both of which are named in its own "What it does not cover." No design defect introduced.

What I read: all 12 story bodies in full (`aep plan artifact show story:<id>`), `epic:mcp-contracts` in full, `aep plan artifact relations`, `aep plan artifact graph` (full store, ~21 `depends_on` + 24 `decomposes`/`serves` edges walked for this set, plus the 2 external `blocks` edges into the epic — no edges walked outside this set turned up a back-reference), `aep plan artifact waves --kind story --status draft`, `aep plan artifact validate` (reports only pre-existing prose-only review-result formatting, unrelated to this set — not restated as a finding).

What I could not establish: whether `mcp-cli-journey-discovery-contract`'s omission of `mcp-inbound-cloud-profile` and `mcp-composition-provenance` from its `depends_on` is deliberate scope (its doc is explicitly the *local* CLI journey, per `docs/local-mcp-cli.md` naming) or a gap — I found no citation either way strong enough to call it a finding, so I leave it unflagged rather than guessed. The parallel-safety collisions the waves command reports (shared scenario/scenario-file directories in waves 5–6) are that critic's lane, not mine, and do not affect this verdict.

```findings
[]
```
