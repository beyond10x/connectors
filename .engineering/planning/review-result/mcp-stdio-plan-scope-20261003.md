---
format: aep.planning-md/3
id: review-result:mcp-stdio-plan-scope-20261003
kind: review-result
status: active
title: Outbound stdio draft scope review (coordinator fallback)
relations:
- reviews: story:mcp-outbound-stdio-runtime
revision: 1
---
approve
Read epic:mcp-contracts in full, then story:mcp-outbound-stdio-runtime in full and the graph. The parent's eight contract deliverables remain the scope baseline: this incremental story takes the selected outbound stdio share of discovery/invocation, transport, capability/result, auth, shared mapping and CLI delivery; inbound/cloud and composition retain their existing owners. It explicitly excludes any claim of full HTTP, cloud, Helm or parent completion. The operator's recorded process decision supplies authority for the new native association, not deployment authority.
Coordinator fallback following aep:plan-critic-scope; not independent review. Six parent deliverables intersect this stdio slice and two remain wholly with existing siblings; neither this review nor a future stdio completion proves the parent's full acceptance.
```findings
[]
```

## Re-created 2026-10-22

Re-created through the planning CLI by wave 20261022 (story:mcp-stdio-runtime-branch) from
branch `unit/mcp-local-runtime-20261003-rebased` head `5140636d9`, where it was `active` at revision 1.
Its transitions and its 2026-10-03 evidence records were not carried: they describe that
branch's tree, not the re-landed one.
