---
format: aep.planning-md/3
id: review-result:wave-d275-ud-adversary
kind: review-result
status: active
title: Adversary pass on bounding catalog reads to the instance repositories
summary: 1 warning and 3 notes; 3 fixed, 1 accepted
relations:
- reviews: story:catalog-reads-stay-inside-the-instance-repositories
revision: 1
---
# Adversary pass on bounding catalog reads to the instance repositories

```findings
[
{"file":"adapters/catalog/src/served.rs","line":175,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"catalog.repositories: null was read as absent and failed open; fixed: null is an empty list and refuses start"},
{"file":"adapters/catalog/src/served.rs","line":115,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a .GIT suffix in another case was not stripped; fixed: stripped in any case for catalog.repositories and git.repositories"},
{"file":"contracts/service/compatibility.md","line":59,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the list of refusals before the anchor lacked the adapter admission; fixed at merge by the coordinator"},
{"file":"crates/connectors-host/src/server/v1alpha2.rs","line":1,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"an unlisted repository answers not_found with audit_ref null like every refusal before the anchor; accepted"}
]
```
