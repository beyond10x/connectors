---
format: aep.planning-md/3
id: review-result:wave-d265-c4-security
kind: review-result
status: active
title: Security review of the git relay (C4)
summary: 6 findings, all fixed
relations:
- reviews: story:git-smart-http-relay-with-quarantine
revision: 1
---
# Security review of the git relay (C4)

```findings
[
{"file":"adapters/catalog/src/served/git.rs","line":1049,"category":"security","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"mirror fetch followed a redirect and sent the forge token header to another host; fixed: http.followRedirects=false"},
{"file":"adapters/catalog/src/served/git.rs","line":1146,"category":"security","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"commit_identity bypassed by 0x1e/0x1f in an ident; fixed: NUL-separated fields and control characters refused"},
{"file":"adapters/catalog/src/served/git.rs","line":1208,"category":"security","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"pre-push timeout killed only the direct child; fixed: own process group killed whole"},
{"file":"adapters/catalog/src/served/git.rs","line":286,"category":"security","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"work_directory itself not made private; fixed: set to 0700 at start"},
{"file":"adapters/catalog/src/served/git.rs","line":1233,"category":"security","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"check stderr collected without bound before the log cut; fixed: drained with the first 4 KiB kept"},
{"file":"adapters/catalog/src/served/git.rs","line":1060,"category":"security","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"ca_file did not replace a compiled-in CA path; fixed: http.sslCAPath set to an empty private directory"}
]
```
