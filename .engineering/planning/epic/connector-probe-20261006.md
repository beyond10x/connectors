---
format: aep.planning-md/3
id: epic:connector-probe-20261006
kind: epic
status: draft
title: Defects from the 2026-10-06 probe of five live connections
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Source

A probe of every permitted read on an operator's five live connections on 2026-10-06 (Zendesk 7,
GitLab 12, Jira 3, Confluence 4, Tavily 3; connectors 0.28.0 to 0.30.0), and the defects filed from
it: beyond10x/connectors#101, #102 (fixed in 0.30.0), #103, #104 (fixed in 0.30.0), #105, the open
#81, and upstream beyond10x/entity-runtime#55.

## Outcome

The operator's connections stay fast and usable as they are used: per-command metadata cost and
owner memory stop growing with the store, every refusal names an action that can help, and the
toolchain and dependencies are the newest releases.

## In scope

- Store growth per command (the registry clock floor) and the owner's memory (#101, #103).
- Refusals that name the wrong next action: expired evidence (`not_granted` advising repair), an
  adapter-reported identity mismatch during a configuration upgrade, a dispatch `unauthorized`
  without its upstream reason.
- Rate-limited catalog reads (`429`) honouring `Retry-After`.
- `operations describe` and `invoke` answering JSON as JSON (#105).
- GitLab issue creation in the forge selection (#81).
- Dependencies: compatible crates.io updates, ESS 0.53.0, AEP 0.68.0.

## Not in scope

The native Tavily adapter's `429` (mapped to `RateLimited` at `adapters/tavily/src/lib.rs:148`, no
`Retry-After`): `story:catalog-honours-retry-after` covers the catalog engine only.
The Entity Runtime open that does not verify the whole history (beyond10x/entity-runtime#55):
an upstream change; `story:metadata-invoke-cost-flat-in-store-size` stays blocked on it. New
providers.
