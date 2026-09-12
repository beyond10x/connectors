---
format: aep.planning-md/1
id: review-result:mcp-plan-critic-acceptance-r2-20260912
kind: review-result
status: active
title: Acceptance critic, MCP decomposition, round 2
relations:
- reviews: story:mcp-inbound-local-binding
revision: 1
---
needs-revision

story:mcp-inbound-local-binding — the six-behaviours paragraph joins three independently-checkable claims with "and" (each of the six behaviours stated as an observable outcome with a state transition; a malformed/partially-delivered message names its outcome; a behaviour the transport doesn't offer is recorded as refused with reason), so one can hold while another fails and there is no single check that says the acceptance is met — .engineering/planning/story/mcp-inbound-local-binding.md:27-37

story:mcp-inbound-local-binding — the scenario-file coverage list names "malformed input" and "partial output" where the behaviours paragraph names "message framing" and "streaming," and omits "progress" outright, so a reviewer cannot mechanically tell whether scenario coverage for all six protocol behaviours the acceptance requires is satisfied — .engineering/planning/story/mcp-inbound-local-binding.md:39-44

What I read: `story:mcp-inbound-local-binding` in full (`aep plan artifact show`), plus `git diff HEAD -- .engineering/planning/story/mcp-inbound-local-binding.md` to isolate exactly what revision 3 added, and the approved sibling `## Acceptance` in `.engineering/planning/story/mcp-outbound-connection-lifecycle.md` for comparison (it states the same six-behaviour-style requirement as one clean for-all conjunction, which the rewritten section here does not). 1 artifact read, as given.

What I could not establish: whether "message framing" and "streaming" are meant to map onto "malformed input" and "partial output" in the scenario list — the document doesn't say, and I did not chase it further since the naming mismatch is itself the finding. The `story:mcp-cli-journey-discovery-contract` depends_on edges were out of scope for this pass and untouched by me.

Out of my lane: whether the scenario-file gap for "progress" leaves epic-deliverable-3 coverage incomplete is `plan-critic-scope`'s question, not mine — I flag it here only as an acceptance-mapping ambiguity, not a coverage gap.

```findings
- file: .engineering/planning/story/mcp-inbound-local-binding.md
  line: 27
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the six-behaviours paragraph joins three independently-checkable claims with "and" (each of the six behaviours stated as an observable outcome with a state transition; a malformed/partially-delivered message names its outcome; a behaviour the transport doesn't offer is recorded as refused with reason), so one can hold while another fails and there is no single check that says the acceptance is met
- file: .engineering/planning/story/mcp-inbound-local-binding.md
  line: 39
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the scenario-file coverage list names "malformed input" and "partial output" where the behaviours paragraph names "message framing" and "streaming," and omits "progress" outright, so a reviewer cannot mechanically tell whether scenario coverage for all six protocol behaviours the acceptance requires is satisfied
```
