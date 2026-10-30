---
format: aep.planning-md/3
id: review-result:wave-d265-c4-adversary
kind: review-result
status: active
title: Adversary pass on the git relay (C4)
summary: '4 findings: 3 fixed, 1 documented and filed as a story'
relations:
- reviews: story:git-smart-http-relay-with-quarantine
revision: 1
---
# Adversary pass on the git relay (C4)

```findings
[
{"file":"adapters/catalog/src/served/git.rs","line":1126,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"pack objects no pushed tip reaches skip identity and pre-push checks and are stored on the forge; fixed: every pack object is reached from a pushed tip or already in the mirror"},
{"file":"adapters/catalog/src/served/git.rs","line":772,"category":"judgement","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"caller push options are relayed to the forge unvetted; fixed: a push with push options is refused"},
{"file":"adapters/catalog/src/served/git.rs","line":1339,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"caller hang-up drops relay_push mid-send so a forge-applied push gets no outcome log; fixed: the push runs in its own task"},
{"file":"adapters/catalog/src/served/git.rs","line":1208,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"pre-push check runs with instance file access and reads the mounted forge token; documented in running-services.md, sandbox is story:git-relay-pre-push-check-sandbox"}
]
```
