---
format: aep.planning-md/3
id: story:pending-connection-refusal-names-its-remedy
kind: story
status: active
title: A pending connection refuses with a next action that clears it
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T07:53:31Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T07:53:31Z", actor: "human:timo", revision: 3}
---
## Outcome

A saved connection that cannot become ready answers a refusal whose `next_action` resolves it. Today a
Confluence connection that is `pending` answers every invoke and `approvals key-status` with
`lifecycle_conflict` and `next_action = retry_status`, which repeats forever, while its revalidation
answers `not_granted` with `next_action = create_connection`.

## Source

Read-only CLI audit of 2026-10-07 and its re-run on 0.33.0 (2026-10-08 07:50Z): the Confluence
connection was `pending` before the audit began; on 0.33.0 revalidation refused `not_granted`
(`create_connection`), `pages.changed` and `approvals key-status` refused `lifecycle_conflict`
(`retry_status`). Jira on the same Atlassian profile revalidated and read. Cause not established.

## Acceptance

- The cause is named from a runtime observation (which admission step refuses, and why only Confluence).
- A test reproduces a pending connection whose revalidation cannot succeed; invoke and
  `approvals key-status` on it answer a refusal whose `next_action` is the step that clears it, never
  `retry_status` when no status change is pending.
- If the cause is a defect (state left by an interrupted publication, a wrong profile or a lost
  credential reference), the defect is fixed and the test covers it; the operator's connection is not
  edited by hand.
