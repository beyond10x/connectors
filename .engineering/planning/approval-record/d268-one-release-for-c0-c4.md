---
format: aep.planning-md/3
id: approval-record:d268-one-release-for-c0-c4
kind: approval-record
status: approved
title: One release, 0.62.0, for units C0-C4 (D268)
relations:
- decides: story:definition-lowerer-reads-published-features
- decides: story:catalog-github-app-installation-token
- decides: story:served-instance-admits-a-service-account-token
- decides: story:served-keyed-writes-under-a-standing-approval
- decides: story:git-smart-http-relay-with-quarantine
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-30T09:51:49Z", actor: "human:timo", revision: 2, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
---
## Decision D268 (2026-10-30)

Ship C0-C4 as one release, 0.62.0, cut from `wave/d265-c4`; no separate 0.63.0. The coordinator
of wave D243-C1 had been idle since about 08:40Z with C0-C3 integrated and gated on
`wave/d243-c1` (14e6ce0c7), and that wave closes into this one. Decided by the orchestrator under
Timo's grant ("push through !", 2026-10-30).

| unit | story |
|---|---|
| C0 | `story:definition-lowerer-reads-published-features` |
| C1 | `story:catalog-github-app-installation-token` |
| C2 | `story:served-instance-admits-a-service-account-token` |
| C3 | `story:served-keyed-writes-under-a-standing-approval` |
| C4 | `story:git-smart-http-relay-with-quarantine` |

Gate: the C-wave gate on 14e6ce0c7 was green (touched targets in 11 crates, clippy, drift checks,
ESS validate, Gates check). The 0.62.0 gate adds what C4 and the version bump change; CI runs the
full workspace gate on the pull request.
