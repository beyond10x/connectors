---
format: aep.planning-md/1
id: review-result:mcp-plan-critic-scope-r2-20260912
kind: review-result
status: active
title: Scope critic, MCP decomposition, round 2
relations:
- reviews: story:mcp-inbound-local-binding
- reviews: story:mcp-inbound-capability-projection
- reviews: story:mcp-inbound-mutation-replay
- reviews: story:mcp-profile-selection-matrix
- reviews: story:mcp-inbound-cloud-profile
revision: 1
---
`approve`

**What I read:** Parent `epic:mcp-contracts` in full (`aep plan artifact show epic:mcp-contracts`) before touching any story. All 12 stories under it (`aep plan artifact show story:mcp-{cli-journey-discovery-contract,composition-provenance,domain-model,inbound-capability-projection,inbound-cloud-profile,inbound-local-binding,inbound-mutation-replay,outbound-auth-lifecycle,outbound-connection-lifecycle,outbound-invocation-results,profile-selection-matrix,specification-pin}`). `aep plan artifact graph --root epic:mcp-contracts`, grepped for `mcp`, to confirm the 12 `decomposes` edges are the whole set and no other artifact claims part of this epic. `git diff` on the two touched files to isolate exactly what round 2 changed. The round-1 scope critic's own record (`.engineering/planning/review-result/mcp-plan-critic-scope-r1-20260912.md`) to confirm the blocker's original wording and my baseline count.

**Promise count:** 14 promises extracted (8 "Required contract deliverables" bullets + 6 numbered "Acceptance" criteria). 14/14 now trace to a claiming story: 12 traced cleanly on round 1 and unchanged here; 2 (AC3's caller-isolation half, deliverable 3's outbound-stdio half) remain honestly narrowed against `decision-blocker:mcp-caller-connection-assignment` and `decision-blocker:mcp-outbound-stdio-process-ownership` respectively, cited by name; the 15th tracked item from round 1 — deliverable 3's and AC5's framing/streaming/cancellation/progress/session-loss/version-mismatch requirement for the inbound local-binding profile — is now claimed directly in `story:mcp-inbound-local-binding`'s revision-3 Acceptance (`.engineering/planning/story/mcp-inbound-local-binding.md`): "the six protocol behaviours `epic:mcp-contracts` deliverable 3 requires... message framing, streaming, cancellation, progress, connection and session loss, and version and capability mismatch," plus a second paragraph naming scenario files for "cancellation, version and capability mismatch, malformed input, partial output and session loss," with lost-replies/mutation-uncertainty explicitly left to `story:mcp-inbound-mutation-replay` and error-mapping scenarios to `story:mcp-inbound-capability-projection`. The round-1 blocker is closed.

Checked specifically for what the revision could have broken: `story:mcp-inbound-capability-projection`'s own Acceptance and Scope sections still name only "projection and error-mapping scenario files," not cancellation/malformed-input/mismatch/partial-output, so no two stories claim the same outcome; `story:mcp-inbound-cloud-profile` claims the same six-behaviour list for the *cloud* profile, which the epic asks for separately ("selected profiles," plural) and is a different binding, not a duplicate; `story:mcp-profile-selection-matrix`'s own Acceptance still confines itself to the disposition matrix and explicitly disclaims owning behaviour ("How a selected transport actually frames, streams, cancels or loses a session... `story:mcp-inbound-local-binding` own[s] that per direction"), so no reach was introduced there either. The other change in this round (`story:mcp-cli-journey-discovery-contract` gained four more `depends_on` edges) touched no Acceptance or Scope text, so it carries no scope implication.

**What I could not establish:** `story:mcp-inbound-mutation-replay`'s prose still attributes "cancellation, malformed input, mismatch and partial output" jointly to *both* `story:mcp-inbound-local-binding` and `story:mcp-inbound-capability-projection`, but only `mcp-inbound-local-binding`'s Acceptance actually names all four; `mcp-inbound-capability-projection` never uses those four words and its Scope confines its own file additions to "projection and error-mapping." This reads as a stale/imprecise cross-reference rather than a duplicate claim (no second acceptance section repeats the same outcome), so it does not change my verdict, but I flag it as a cross-reference-accuracy question that is design-critic's lane, not mine, to weigh.

```findings
[]
```
