---
format: aep.planning-md/3
id: story:migrate-tavily-entries
kind: story
status: draft
title: setup migrate moves a Tavily entry, or names why it stays pinned
tags:
- d298
relations:
- decomposes: epic:connector-vocabulary
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`setup migrate` moves a legacy Tavily entry (today kept as `implementation_unmapped`) to an
instance of a `tavily` connector without re-consent, or the operator decides it stays pinned.

## Context

Split from `story:migrate-writes-the-short-instance-form` in wave D298. Tavily is its own
implementation (`adapters/tavily`, binary `connectors-tavily`, its own native format and
revision), not a catalog provider, and derivation runs only the installed catalog provider. Either
Tavily becomes a catalog provider (its revision changes, so connections are not kept) or
non-catalog implementations learn to derive. That choice needs a decision first.

## Acceptance

- The decision is recorded, and a legacy Tavily entry migrates or is named as staying pinned with
  that reason.
