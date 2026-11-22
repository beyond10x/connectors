---
format: aep.planning-md/3
id: story:slack-socket-mode-stream
kind: story
status: draft
title: Socket Mode events on an app-level token (out of the first wave)
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-four-profile-definition-generated
- depends_on: story:slack-user-writes-by-policy
- depends_on: story:admission-checks-token-scopes
scope:
- confidence: inferred
  path: adapters/catalog/src
revision: 2
---
## Outcome

An app-level token (`xapp`, profile `slack.app`) receives Slack's Socket Mode events through
Connectors. Out of the first wave (Timo's direction, 2026-11-23, relayed by `bf-runbook`).

## Today

`apps.connections.open` is an HTTP method and lands with
`story:slack-four-profile-definition-generated`. The stream it opens is a WebSocket. Connectors
operations are request and answer; no operation holds a socket open or delivers events.

## Open

- Where a stream lives: a feed (`epic:generic-datasource-feeds`), a served process, or outside
  Connectors. Decided before any design.
- Acknowledging each envelope within Slack's time limit while a caller reads.

## Acceptance

Written once the open items are decided.

## Scope

Not assessed.
