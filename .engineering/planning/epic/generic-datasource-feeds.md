---
format: aep.planning-md/3
id: epic:generic-datasource-feeds
kind: epic
status: draft
title: Incremental reads of any source in one shape, whatever provider answers
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A provider-independent feed family, `datasource.feed/v1alpha1`: a consumer lists what a connection can be read from (channels, projects, spaces) and reads the items of each since a watermark it keeps. Every item has a stable identity, a revision that changes when its content changes, its times, its author, its text and its place in a thread, in one shape whatever provider answers. A consumer reads any connection whose adapter binds the family without code for that provider: adding or upgrading an adapter in Connectors makes a new source available to the consumer with no rebuild of the consumer.

Requested by the operator on 2026-10-06 for a consumer that keeps knowledge stores current from many sources, as `datasource.websearch/v1alpha1` already does for web search (`epic:generic-websearch`).

## Findings (2026-10-06)

| Fact | Source |
|---|---|
| Shared families live under `contracts/datasources/` (`logs`, `records`, `series`, `websearch`); adapters bind them under `adapters/<id>/contracts/<family>/` | `contracts/datasources/`, `contracts/README.md` |
| `datasource.records/v1alpha1` defines bounded document envelopes and states that no generic list codec promises a snapshot, deduplication or stable membership, and that it grants no automatic refresh | `contracts/datasources/records/v1alpha1/semantics.md` |
| The catalog providers today are confluence, gitlab, google (4), hubspot, jira, zendesk; Slack is `story:catalog-slack-reads` (draft) | `adapters/catalog/providers/` at `80bee2f` |

So incremental reading (where a consumer stands, what changed, what was removed) is a family of its own, built on the records envelope for each item's body.

## Decomposition

1. `story:feed-contract`: the shared `datasource.feed/v1alpha1` semantics, ESS model and conformance scenarios.
2. `story:feed-bindings-discoverable`: a consumer finds, for one saved connection, the operations that bind the family and their profile, at run time.
3. `story:slack-feed-binding`, `story:jira-feed-binding`, `story:gitlab-feed-binding`, `story:confluence-feed-binding`: the first bindings.

Out of scope: writes, push delivery (webhooks), and any change to `datasource.records/v1alpha1`.
