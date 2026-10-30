---
format: aep.planning-md/3
id: story:catalog-operation-declares-its-walk
kind: story
status: active
title: A catalog read declares how its pages are walked and when the walk ends
tags:
- d240
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T11:46:44Z", actor: "human:timo", revision: 2, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
- {from: "proposed", to: "active", at: "2026-10-30T11:46:45Z", actor: "human:timo", revision: 3, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
---
## Problem

An agent that reads all of a ticket's audits or comments writes its own loop: send `page[size]`,
read `meta.after_cursor`, send it as `page[after]`, stop on the right signal. The end rule lives
only in prose (`docs/catalog-zendesk.md`, the operation descriptions in
`adapters/catalog/providers/zendesk/operations.json`), so each agent re-derives it and some get it
wrong.

## Outcome

A read in a `connectors-catalog-operations/1` selection may declare a `walk` block, typed from
the feed vocabulary (`adapters/catalog/spec/ess/domains/feed.yaml`, `connectors_catalog.feed.Condition`):

- `parameter`: the query parameter that carries the position (`page[after]`);
- `next`: a JSON pointer into the response body to the next position (`/meta/after_cursor`);
- `records`: a JSON pointer to the page's records (`/audits`);
- `query`: fixed query values every page sends (`include_boundary_indicators: true`);
- `ends`: a list of conditions, the first that holds ends the walk, each named so a walk's
  summary can say which one ended it.

The engine refuses a malformed `walk` when the selection loads (on a write, a pointer that does
not parse, a `parameter` that is not a query parameter of the operation, an empty `ends`).

The declaration reaches the CLI through the operation's admitted description, so
`connectors invoke <connector>/<instance> <operation> --describe` shows it.

Declared for Zendesk `ticket.comments` and `ticket.audits` (end on `meta.has_more` false, on
`meta.after_cursor` absent or null, or on no records, as the descriptions state; `ticket.audits`
sends `include_boundary_indicators: true`), the three incremental exports (end on
`end_of_stream: true`; `organizations.incremental` passes `end_time` as `start_time`), and the
cursor reads of the other catalog connectors whose doc names an end condition.

## Contract

- New type in `adapters/catalog/spec/ess/domains/` for the walk; `docs/local-catalog-provider.md`
  states it; the catalog bundle and generated outputs regenerated.

## Acceptance

Named tests in `adapters/catalog/tests/walk_declarations.rs`: the Zendesk declarations load, a
malformed walk is refused by name at load, a write with a walk is refused, and `--describe` (or
the description the owner admits) carries the walk.

## Out of scope

Carrying `walk` through the definition lowerer into `connectors-definition/1`: that format's
schema is owned by the definitions repository, so it is a follow-up story there
(`story:definition-lowerer-carries-walk`). Running the walk is `story:invoke-walks-declared-pages`.
