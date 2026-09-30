---
format: aep.planning-md/3
id: review-result:adversary-catalog-parameter-types-pass-1
kind: review-result
status: active
title: Adversary pass 1 on catalog parameter types
relations:
- reviews: story:catalog-parameters-declare-their-type
revision: 1
---
unit: story:catalog-parameters-declare-their-type, uncommitted tree wave0930b-params on 62aec7648
verdict: NEEDS-CHANGE
cases: executed 203→206, red 2
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: scratch/adversary (base copy and build dir, 841M)
needs-coordinator: numeric project ids on string-typed parameters; the CHANGELOG entry

Cases in adapters/catalog/tests/parameter_types_adversary.rs: numeric project id still accepted (red: refused on
pipelines.list, pipeline.get, pipeline.jobs, file.get; green at base), a declared integer never sent as a float (red,
also red at base), 18 malformed integer and boolean forms refused before any request (green; red at base).
Could not break: bundles equal to base apart from the new `type` field; per-type counts match the documents; bounds
order; no parameter name in both path and query.

Coordinator routing: 1 fixed (string parameters take a JSON integer as decimal text); 3 fixed (integer parameters
refuse non-integer JSON numbers); 2 is the coordinator's CHANGELOG entry (descriptor revision moves, approval
policies bound to the old revision must be set again).

```findings
[
  {"file": "adapters/catalog/src/lib.rs", "line": 673, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A string-typed parameter now refuses a JSON integer, so a numeric GitLab project id accepted at the base is refused on pipelines.list, pipeline.get, pipeline.jobs and file.get while oneOf-typed reads still accept it."},
  {"file": "CHANGELOG.md", "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The acceptance requires the bundle and descriptor-revision change to be stated in the CHANGELOG, and no entry exists, although the revision change stops existing approval policies from matching."},
  {"file": "adapters/catalog/src/lib.rs", "line": 132, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A declared integer given 2.0 passes the schema and is sent as the text 2.0 (also as a path segment)."}
]
```
