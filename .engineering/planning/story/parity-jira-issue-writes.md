---
format: aep.planning-md/3
id: story:parity-jira-issue-writes
kind: story
status: draft
title: Jira issue create, edit, comment, link and delete
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Jira issue create, edit, comment, link and delete — parity unit U06 of `docs/fluxplane-plugin-parity.md`.

## Operations

`jira.issue.comment.add`, `jira.issue.create`, `jira.issue.link.add`, `jira.issue.edit`, `jira.issue.delete`

326 calls since 2026-09-09 (declared and mapped undeclared names), provider `jira`, planned wave W2.

## Surface

catalog `operations.json` selection: `createIssue`, `editIssue`, `addComment`, `linkIssues`, `deleteIssue`, guards to decide; Markdown-to-ADF would be an engine change.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
