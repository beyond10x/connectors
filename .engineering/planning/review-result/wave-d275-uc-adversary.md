---
format: aep.planning-md/3
id: review-result:wave-d275-uc-adversary
kind: review-result
status: active
title: Adversary pass on naming the refused state directory
summary: 2 warnings and 1 note, all fixed in the correction round
relations:
- reviews: story:served-state-on-a-shared-volume-root
revision: 1
---
# Adversary pass on naming the refused state directory

```findings
[
{"file":"docs/running-services.md","line":174,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the subPath bullet admitted a host-owned 0755 directory mounted as state, which start refuses as not private; fixed: split into mounted-as-state (0700) and mounted-as-parent"},
{"file":"crates/connectors-host/src/server.rs","line":451,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a 0777 or foreign-owned state was told only the first failing reason, never to set 0700; fixed: the state directory's refusal states the whole target"},
{"file":"docs/running-services.md","line":176,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a root the host cannot search refuses with the closed text, unnamed; fixed in docs, the case pins the closed text"}
]
```
