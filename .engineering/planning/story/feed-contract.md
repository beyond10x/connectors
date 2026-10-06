---
format: aep.planning-md/3
id: story:feed-contract
kind: story
status: implemented
title: The datasource.feed/v1alpha1 family is stated and modelled
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: contracts/datasources/feed/v1alpha1
- confidence: inferred
  path: ess
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T09:58:15Z", actor: "agent:claude", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-06T09:58:16Z", actor: "agent:claude", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-06T11:28:25Z", actor: "agent:claude", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`datasource.feed/v1alpha1` is stated under `contracts/datasources/feed/v1alpha1/` and modelled in ESS, with conformance scenarios any binding runs.

## Work

Two read operations every binding provides:
- **containers**: what the connection can be read from. Each container has a stable `id`, a `name`, a `kind` (provider vocabulary, open) and a `visibility` the binding states (public, private, direct). Direct conversations are never listed unless the binding's profile declares them, and a binding states how it excludes them.
- **items**: one container's items since a `watermark` the consumer passes back unchanged (opaque; absent on a first read), bounded by `limit`. An item has `id` (stable within the container), `revision` (changes when the content changes), `created_at`, `updated_at`, `author` (`id`, optional `display_name`), `body` (a `datasource.records/v1alpha1` envelope), `url`, an optional `parent` (thread or containing item), and `deleted: true` for an item the provider reports removed. A page has `items`, `next_watermark` and `complete`.

Rules the semantics state:
- The watermark makes reads resumable: reading again with the last `next_watermark` returns every item created or changed since, possibly again (at-least-once). A consumer deduplicates by `(container, id, revision)`.
- A binding states how far back a first read reaches, how deletions are observed (or that they are not), and its time precision.
- Bounds, errors and admission follow `datasource.records/v1alpha1` and the service contract.

## Acceptance

- `contracts/datasources/feed/v1alpha1/semantics.md` and its ESS model validate (`ess specify validate`).
- A conformance suite (synthesized from the model plus authored scenarios) runs against a fixture binding: a first read, a resumed read that returns a changed item with a new revision and a new item, a deleted item, and a container listing that omits a direct conversation.
