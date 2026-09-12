---
format: aep.planning-md/1
id: review-result:mcp-plan-critic-scope-r1-20260912
kind: review-result
status: active
title: Scope critic, MCP decomposition, round 1
relations:
- reviews: story:mcp-inbound-local-binding
- reviews: story:mcp-inbound-capability-projection
- reviews: story:mcp-profile-selection-matrix
- reviews: story:mcp-inbound-mutation-replay
revision: 1
---
`needs-revision`

epic:mcp-contracts — deliverable 3 ("specify framing, streaming, cancellation, progress, connection/session loss and version mismatch for the selected profiles") and acceptance criterion 5 ("Cancellation, malformed input, version/capability mismatch, partial output, lost replies and mutation uncertainty have reviewed scenarios") are not claimed for the inbound local-binding profile by any drafted story: `story:mcp-profile-selection-matrix` and `story:mcp-inbound-mutation-replay` both say cancellation/mismatch/malformed-input/partial-output for this direction belong to `story:mcp-inbound-local-binding` and `story:mcp-inbound-capability-projection`, but neither of those two stories' Acceptance sections names framing, streaming, cancellation, progress or version/capability mismatch anywhere in their bodies — .engineering/planning/epic/mcp-contracts.md:23,42

## What I read

Parent epic (`aep plan artifact show epic:mcp-contracts`) — 8 "Required contract deliverables" bullets, 6 numbered "Acceptance" criteria, one "Scope and status" exclusion list. All 12 stories whose id starts `story:mcp-` (`aep plan artifact list --kind story --status draft`, `aep plan artifact show story:<id>` for each). Both decision-blockers (`decision-blocker:mcp-caller-connection-assignment`, `decision-blocker:mcp-outbound-stdio-process-ownership`). Full planning graph (`aep plan artifact graph`, grepped for `mcp`) to confirm no other artifact decomposes this epic. The critic rubric at `/home/timo/.claude/plugins/marketplaces/beyond10x/plugins/aep-plan/skills/planning/references/critic-rubric.md`. Targeted greps across all 12 story bodies for `cancel`, `progress`, `framing`, `streaming`, `mismatch`, `malformed` to verify the gap rather than infer it.

**Promise count:** 8 deliverable bullets + 6 acceptance criteria = 14 promises extracted. 12 traced cleanly to a claiming story. 1 (AC3's caller-isolation half) and 1 (deliverable 3's outbound-stdio evaluation half) are honestly narrowed by the two named decision-blockers — both cite the exact `UNMAPPED:` shared-model lines that make the gap real rather than convenient, and both are scoped no wider than the acceptance text they hold open, so I count them as satisfied-with-named-exclusion. 1 promise (inbound cancellation/framing/streaming/progress/version-mismatch scenarios, deliverable 3 + AC5) traces to no story's acceptance text at all, despite two other stories' prose pointing at a claimant — that is the finding above.

I found no artifact outside the 12-story set decomposing this epic (graph checked), no reach into implementation/deployment/credential-migration/provider-operation (every story explicitly disclaims runtime action), and no two stories claiming the same outcome.

## What I could not establish

Whether the missing inbound cancellation/framing/streaming/progress/mismatch coverage was an intentional, unstated narrowing the decomposer meant to fold into a later revision of `story:mcp-inbound-local-binding` — nothing in the store says so, so I treat it as unstated rather than assume good intent.

```findings
- file: .engineering/planning/epic/mcp-contracts.md
  line: 23
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: deliverable 3's and acceptance criterion 5's cancellation/framing/streaming/progress/version-mismatch requirement for the inbound local-binding profile is attributed by story:mcp-profile-selection-matrix and story:mcp-inbound-mutation-replay to story:mcp-inbound-local-binding and story:mcp-inbound-capability-projection, but neither story's acceptance claims it
```
