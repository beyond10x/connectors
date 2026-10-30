---
format: aep.planning-md/3
id: release-plan:connectors-v0621-rehearsal-findings
kind: release-plan
status: active
title: 'Release 0.62.1: findings from the served forge access rehearsal'
summary: 'D275: five rehearsal findings on 0.62.0'
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T15:16:30Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "approved", at: "2026-10-30T15:16:31Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "active", at: "2026-10-30T15:16:32Z", actor: "human:timo", revision: 4}
---
## Outcome

Patch release 0.62.1, the closing release of wave D275 under
`approval-record:wave-d275-0621`, with the findings of a cluster rehearsal of 0.62.0:

- A served host admits a list of service accounts, or a namespace and a name pattern.
- The TokenReview `credential_file` is read at `0640` under a group the process holds.
- The documented review-token path opens (`..data/token`).
- A refused `state` names the directory and the reason.
- A served catalog instance's reads stay inside `catalog.repositories`.

No breaking changes.

## Scope

| story | status |
|---|---|
| story:served-admission-admits-a-set-of-service-accounts | implemented |
| story:review-token-file-admits-fsgroup-read | implemented |
| story:review-token-path-in-docs-opens | implemented |
| story:served-state-on-a-shared-volume-root | implemented |
| story:catalog-reads-stay-inside-the-instance-repositories | implemented |
