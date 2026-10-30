---
format: aep.planning-md/3
id: review-result:wave-d275-ub-adversary
kind: review-result
status: active
title: Adversary pass on the review token under fsGroup
summary: 2 notes, both fixed in the correction round
relations:
- reviews: story:review-token-file-admits-fsgroup-read
- reviews: story:review-token-path-in-docs-opens
revision: 1
---
# Adversary pass on the review token under fsGroup

```findings
[
{"file":"crates/connectors-host/src/credentials.rs","line":177,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the group-read path masked only 0o077, so setuid, setgid and sticky bits on a 0640 review token were admitted; fixed: the group-read path admits exactly 0640 or 0440"},
{"file":"crates/connectors-host/src/credentials.rs","line":183,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"owner bits were unchecked under group read, so 0740 and 0540 were admitted; fixed: the group-read path admits exactly 0640 or 0440"}
]
```
