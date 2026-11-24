---
format: aep.planning-md/3
id: epic:review-20261123-findings
kind: epic
status: draft
title: Findings of the 2026-11-23 connectors review (0.74.1 with Definitions 0.8.0)
summary: '11 findings: catalog ownership, pagination, tool contracts, two broken callers, setup, inventory, docs'
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

Every finding of the connectors review of 2026-11-23 is fixed, shown already fixed, or carried
to the repository that owns it.

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Stories

One story per finding (11), plus the timing test that failed once on pull request #276.

Also under this epic, from issue #267 (2026-11-16, the store grows by 0.9 MB per read; consumer
dev-operator), which the review did not list: story:invoke-admission-queues-under-load,
story:entity-runtime-0330-records-name-definition-by-digest,
story:settled-read-uses-leave-the-store, story:connections-list-answers-through-the-owner and
story:owner-observe-request-is-modelled-in-ess.

## Out of scope

Releases. Production deployments (outside the review).
