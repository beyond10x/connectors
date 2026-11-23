---
format: aep.planning-md/3
id: story:connections-list-answers-through-the-owner
kind: story
status: draft
title: connections list, describe and status answer through a running owner
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

While an owner of the same build runs for the state directory, `connections list`, `connections
describe` and `connections status` answer from the owner's held handle. The CLI process opens and
replays the store for them only when no such owner runs.

## Observed

Issue #267 (wanted item 3), 2026-11-16, connectors 0.73.0: `connections list --connector slack
--instance <name>` took 0.26 s on a fresh store and 35 s real, 26 s user on a 133 MB copy, all in
the CLI process. `origin/main` at 56f5b9db642 still opens the store in the CLI:
`apps/connectors/src/local/connections.rs:15`, `:434` and `:458` call
`Registry::with_system_clock(&paths.state)`. `invoke` already lists through the owner
(`apps/connectors/src/local/invoke.rs:355`, `owner::listed_through_owner`).

Consumer: dev-operator (session bf-runbook), whose Jira store held 650 MB on 2026-11-23.

## Acceptance

- With a running owner and a store of the size the read-cost test uses, `connections list`,
  `describe` and `status` each answer without the CLI process opening the store: a test counts
  the CLI's store opens and reads 0.
- With no owner running the three commands answer as today.
- The time of `connections list` on that store is reported before and after.

## Out of scope

Store size itself (retention and the record format): issue #267 items 1 and 2, which wait for
Timo's choice of path (question D3, 2026-11-23).
