---
format: aep.planning-md/3
id: review-result:wave-d275-ua-adversary
kind: review-result
status: active
title: Adversary pass on admitting a set of service accounts
summary: 1 warning and 2 notes, all fixed in the correction round
relations:
- reviews: story:served-admission-admits-a-set-of-service-accounts
revision: 1
---
# Adversary pass on admitting a set of service accounts

```findings
[
{"file":"crates/connectors-host/src/server.rs","line":80,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a presence member written as null counted as present for the v3 schema but absent for the reader; fixed: schema and reader refuse null for all six presence members"},
{"file":"crates/connectors-host/src/server/token_review.rs","line":234,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"principal() returned the first entry or the pattern text; fixed: documented, and accounts() describes the configured form"},
{"file":"crates/connectors-host/src/server/token_review.rs","line":397,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"patterns no valid name can match passed start; fixed: refused at start naming the key"}
]
```
