---
format: aep.planning-md/1
id: review-result:fresh-plan-parallel-safety-1
kind: review-result
status: active
title: parallel-safety critic round 1
relations:
- reviews: epic:incremental-source-reads
- reviews: story:brain-source-read-operations
revision: 1
---
needs-revision

story:typed-source-health — Supporting list-valued source-unit bindings requires adapting the scalar role-authority checks in crates/brain-rules/src/extract.rs (inferred from current consumers), which story:fair-receipted-extraction owns, but neither body assigns that shared-file adaptation or fixes its role-membership interface for the first parallel wave; explicitly assign the adaptation and agree the interface before dispatch. — crates/brain-rules/src/extract.rs:201

Read ten artifacts through aep plan artifact show, all three checkout graphs through aep plan artifact graph, and role consumers through rg and sed; seven implementation-story surfaces are cited, zero rely solely on inference, and zero are unplaced; three epics are containers, and the additional shared extractor surface above is inferred.

Could not establish: a compatible multi-binding role-membership contract or owner for adapting the extraction authority guards; acceptance completeness and decomposition quality are outside this lane.

```findings
- file: crates/brain-rules/src/extract.rs
  line: 201
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: Supporting list-valued source-unit bindings requires adapting the scalar role-authority checks in crates/brain-rules/src/extract.rs (inferred from current consumers), which story:fair-receipted-extraction owns, but neither body assigns that shared-file adaptation or fixes its role-membership interface for the first parallel wave; explicitly assign the adaptation and agree the interface before dispatch.
```
