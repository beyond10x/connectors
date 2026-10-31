---
format: aep.planning-md/3
id: story:migrate-refuses-the-whole-run
kind: story
status: draft
title: setup migrate refuses the whole run when an entry cannot move
tags:
- d298
relations:
- decomposes: epic:connector-vocabulary
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`setup migrate` refuses the whole run, naming every entry it cannot move and why, and writes
nothing, as `story:migrate-writes-the-short-instance-form` first asked.

## Context

Split from that story in wave D298. Today an entry migrate cannot move is kept and the others are
written. Refusing the whole run before gateway entries, Tavily and Slack's user selection can move
would stop every configuration holding one of them from migrating at all, so this follows
`story:migrate-gateway-entries`, `story:migrate-tavily-entries` and
`story:slack-user-connector-declaration`.

## Acceptance

- A configuration with one entry migrate cannot move is refused as a whole, naming that entry and
  its reason, and the file is unchanged.
