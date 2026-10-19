---
format: aep.planning-md/3
id: story:connection-next-actions-no-cycle
kind: story
status: draft
title: Every next action a connection command answers is accepted by the state it leads to
summary: 'issue #181: revalidate, reconnect and connect answer each other''s next action in a loop'
refs:
- provider: github
  reference: beyond10x/connectors#181
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:confluence-token-kind-binding
revision: 1
---
## Outcome

Every next action a connection command answers (`revalidate`, `reconnect`, `connect`) is one the state
it leads to accepts: following the answers ends in a usable connection or in a refusal that names the
failed probe, never in a cycle.

## Why now

https://github.com/beyond10x/connectors/issues/181 (seen on 0.53.1): on `atlassian.basic`, revalidate
answers `create_connection`, reconnect answers `create_connection`, connect answers
`repair_connection`, and the caller loops. The Confluence half (v2 probe 401 "scope does not match",
v1 200) is `story:confluence-token-kind-binding`; the loop itself has no story. Code:
`apps/connectors/src/local/connections.rs:318-335`. Whether 0.54.0 still loops is not known.

## Acceptance

- From fixtures: an Atlassian site whose v2 probe answers 401 "scope does not match" and v1 answers
  200. Starting from a pending connection, following each command's `next_action` reaches a
  connection or a refusal naming the probe within three commands; a test asserts no answer repeats.
- A Jira connection on the same profile reports why it is pending (the issue saw it pending with no
  reason).
