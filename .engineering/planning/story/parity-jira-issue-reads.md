---
format: aep.planning-md/3
id: story:parity-jira-issue-reads
kind: story
status: draft
title: Jira single-issue, create-metadata and user reads
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

Jira single-issue, create-metadata and user reads — parity unit U03 of `docs/fluxplane-plugin-parity.md`.

## Operations

`jira.issue.show`, `jira.issue.create_meta`, `jira.user.search`

398 calls since 2026-09-09 (declared and mapped undeclared names), provider `jira`, planned wave W1.

## Surface

catalog `operations.json` selection: `getIssue`, `getCreateIssueMetaIssueTypes`, `findUsers`.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Wave 20261009a result

Written on wave/20261009a (unit commit b23ca437b, merged d5ba74512): catalog Jira selections `issue.get` (getIssue), `issue.create_meta` (getCreateIssueMetaIssueTypes) and `users.search` (findUsers), with tests against hand-written fixtures in the pinned document's shapes. The bundle already carried all three.

Limits: `getIssue`'s `updateHistory` (records a project view) is accepted like every declared parameter, since every Jira selection is pinned to the same parameter policy; the fields of one issue type (getCreateIssueMetaIssueTypeId) are not selected.

Not yet compiled or run: package gates of connectors-catalog and the status page regeneration are owed to the build.
