---
format: aep.planning-md/3
id: decision-blocker:wave-20261014b-scope
kind: decision-blocker
status: cleared
title: 'What wave 20261014b delivers: incident call lookup and traces'
relations:
- blocks: story:incident-call-lookup
- blocks: story:incident-call-traces
- serves: vision:independent-contract-adapters
- blocks: story:parity-slack-message-writes
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T05:12:41Z", actor: "human:timo", revision: 2}
---
## Question

What wave 20261014b delivers: the two incident call reads a deterministic troubleshooting workflow
needs, a call looked up by its number and a call's traces followed by its identifier.

## Options

| option | what | cost |
|---|---|---|
| A | Two units by adapter: the SQL read profile (story:incident-call-lookup, SQL half) and the Loki line and trace profiles (story:incident-call-lookup, Loki half, and story:incident-call-traces). The cross-source join test is written at integration. | Two trees of package gates, about 4-6G of target each. |
| B | One unit per story: the lookup first (SQL and Loki), the traces after it merges. | Serial on the Loki adapter; the traces wait a round. |

Recommended: A. The SQL and Loki adapters share no source file; both stories' Loki work lands in one
tree, so no two units edit the same adapter.

Default on silence: A.

## Decided

Option A, 2026-10-14: wave 20261014b delivers both stories on the integration branch
`wave/20261014b`, one pull request, then a release.

Width: two units. Every other ready unit is outside this wave's order: Jira writes stay held while
the operator's own Jira branches exist in this repository; story:registry-passive-approval-test-flake
needs 100 full host runs and waits for disk headroom; story:mounted-version-released-state depends on
mounted custody, which merges in wave 20261014a.

## Units

| unit | tree | branch | base | stage |
|---|---|---|---|---|
| story:incident-call-lookup (SQL read profile) | conn-u-lookup-sql | impl/incident-call-lookup-sql | 362d99320 | dispatched |
| story:incident-call-lookup (Loki line profile), story:incident-call-traces | conn-u-loki-profiles | impl/incident-call-loki-profiles | 362d99320 | dispatched |

| story:incident-call-lookup (cross-source join test) | conn-u-call-join | impl/incident-call-join | fedbfd5ca | dispatched |
| story:parity-slack-message-writes | conn-u-slack-writes | impl/parity-slack-message-writes | fedbfd5ca | dispatched |
| docs: moving a local install from `slack-credentials` to the catalog provider | conn-u-slack-migration | docs/slack-local-migration | fedbfd5ca | dispatched |

Widened 2026-10-14: Slack message edit and delete (U08) joins the wave; it shares no source file
with the SQL and Loki units. docs/catalog-slack.md is edited by both Slack units in different
sections.

## Shared files, kept by the coordinator at integration

- CHANGELOG.md
- crates/connectors-docs/src/status.rs and the generated status page and data, regenerated once on
  the integrated head
- the cross-source join test of story:incident-call-lookup's acceptance, written from both units'
  fixtures

## Commits this wave makes

One commit per unit (or a few on the unit's branch), the merges of each unit and of released `main`
into `wave/20261014b`, the closing planning-store commit, the pull request into `main` and its
merge, and the release commit and tag. Nothing else.
