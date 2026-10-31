---
format: aep.planning-md/3
id: approval-record:wave-d265-c4
kind: approval-record
status: approved
title: 'Wave D265-C4: git smart-HTTP relay'
summary: One unit, C4, released as v0.63.0 after the 0.62.0 wave
relations:
- decides: story:git-smart-http-relay-with-quarantine
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-30T07:34:10Z", actor: "human:timo", revision: 2, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
---
## Wave D265-C4

One unit on the integration branch `wave/d265-c4` (tree `conn-d265-c4`), started from the head of
`wave/d243-c1` (`9e28c7347`, C0 to C2 merged), one full gate at the end, one pull request, release
v0.63.0. The 0.62.0 wave (C0 to C3) reaches `main` first; this branch then takes `main` before its
gate.

| unit | story | depends on |
|---|---|---|
| C4 | `story:git-smart-http-relay-with-quarantine` | C1, C2 |

## Approval

The orchestrator session, 2026-10-30, item D265(a), under the grant of the workspace instructions
(class C: a wave, its commits, pull request, merge and release): "Item D265(a): connectors unit C4,
Git smart-HTTP relay with quarantine, ref policy, identity check and pre-push check. Release it as
connectors 0.63.0." The operator's words relayed with it, 2026-10-30: "push through !"

## Decided in the wave

- The relay is served by `connectors-catalog-provider --config` under a `git` member, beside the
  instance's operations, and admits its caller under the host's own admission profile.
- The pre-push check is a configured command with no credential in its environment; the commit
  identity check is optional configuration (`commit_identity`).
- The forge token is the instance profile's: a `token` profile's stored token or a `github_app`
  profile's installation token, sent as HTTP basic authentication.
