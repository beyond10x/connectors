---
format: aep.planning-md/1
id: review-result:fresh-plan-design-1
kind: review-result
status: active
title: design critic round 1
relations:
- reviews: epic:incremental-source-reads
- reviews: story:brain-source-read-operations
revision: 1
---
needs-revision

story:typed-source-health — Pending extraction counts and ages require the queue interface owned by story:fair-receipted-extraction, so add a depends_on edge to that story or move the consuming health projection into the already dependent supervisor story — .engineering/planning/story/typed-source-health.md:36; .engineering/planning/story/fair-receipted-extraction.md:27; aep plan artifact graph

Read 10 artifacts with `aep plan artifact show`, all three stores’ `relations`, `graph`, and `validate`, and relevant engine exports; examined 227 graph edges, following all 21 ordering edges including outside the drafted set, with no ordering cycle found.

Could not establish cross-repository completion from local graphs; the instance body explicitly records those external prerequisites.

```findings
- file: .engineering/planning/story/typed-source-health.md
  line: 36
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: Pending extraction counts and ages require the queue interface owned by story:fair-receipted-extraction, so add a depends_on edge to that story or move the consuming health projection into the already dependent supervisor story
```
