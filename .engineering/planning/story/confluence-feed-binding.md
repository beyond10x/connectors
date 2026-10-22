---
format: aep.planning-md/3
id: story:confluence-feed-binding
kind: story
status: active
title: Confluence binds the feed family
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
- depends_on: story:feed-bindings-discoverable
- depends_on: story:catalog-feed-engine
- depends_on: story:catalog-feed-engine-extensions
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md
- confidence: cited
  path: adapters/catalog/providers/confluence-v1/operations.json
- confidence: inferred
  path: adapters/catalog/tests/confluence_v1_feed.rs
- confidence: inferred
  path: adapters/catalog/tests/feed/confluence/
- confidence: inferred
  path: crates/connectors-build/src/confluence_feed_conformance.rs
- confidence: inferred
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/confluence-v1.yaml
- confidence: inferred
  path: crates/connectors-docs/src/status.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T19:55:10Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-06T19:55:10Z", actor: "agent:claude", revision: 3}
---
## Outcome

The Confluence adapter binds `datasource.feed/v1alpha1`: its containers and items are readable through the family, with the binding's own profile, under `adapters/catalog/contracts/feed/v1alpha1/`.

## Acceptance

The family's conformance suite passes against the binding with recorded provider fixtures, and the binding's semantics state: what a container is (a space (pages as items)), how the watermark is formed, how far a first read reaches, whether deletions are observed, and how direct or private material is excluded.

## Depends on

`story:feed-contract`, `story:feed-bindings-discoverable`.

## Scope

Derived 2026-10-21 by `story-scoper` at e26481df09. Every line is **cited** or **inferred**.

- **Primary surface:** `adapters/catalog/providers/confluence-v1/operations.json` (the `feed` block) — cited (pattern: `providers/gitlab/operations.json`)
- **Not this binding:** `confluence` (v2): `getPages` and `getPagesInSpace` take no time parameter and there is no change cursor (`adapters/catalog/src/feed.rs:6-10`) — cited. A v2 feed needs a descending-walk shape and its own story.
- **Binding files (new):** `adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md`, `adapters/catalog/tests/confluence_v1_feed.rs`, `adapters/catalog/tests/feed/confluence/{space,page}.json`, `crates/connectors-build/src/confluence_feed_conformance.rs`, its mod line in `crates/connectors-build/src/main.rs` — inferred (GitLab pattern)
- **Definitions copy:** `crates/connectors-catalog/tests/fixtures/definitions/connectors/confluence-v1.yaml` (GitLab's feed sits at `gitlab.yaml:312-314`) — inferred
- **Engine items needed:** the next-URL parameter cursor (`getSpaces` `start`, `searchContentByCQL` `cursor`; `adapters/catalog/tests/confluence_v1.rs:421,451,736-739`), the CQL template, relative item URLs if `url` maps `_links.webui` — cited, from `story:catalog-feed-engine-extensions`
- **Revision pins that move:** none for `confluence-v1` (grep of `adapters/catalog/tests`) — cited absence
- **Also likely:** `CHANGELOG.md`, `crates/connectors-docs/src/status.rs:336-338` and its regenerated pages — inferred
- **Would collide with:** `story:catalog-feed-engine-extensions` (lands first); `story:confluence-token-kind-binding` on `providers/confluence-v1/operations.json`, `docs/catalog-confluence.md`, `CHANGELOG.md`, `status.rs` (lands first); `epic:catalog-connectors-only-in-definitions` step 4 moves `adapters/catalog/providers/` out
- **Safety fact:** a `feed` key is read only where declared (`crates/connectors-build/src/catalog_feed_conformance.rs:599-616`) — cited

### Decisions taken by the wave coordinator (2026-10-21)

| question | decision | reason |
|---|---|---|
| placement | in-tree, in `providers/confluence-v1/operations.json` and the definitions fixture copy, as GitLab's feed is | the definitions repository is a scaffold with no `connectors/` directory; epic step 4 moves every provider file, this block with them |
| token kind | the feed lives in the `confluence-v1` selection only | a classic-token connection bound to v1 by `story:confluence-token-kind-binding` reads the v1 feed |
| containers | global and personal spaces; every space reads as private | no live read checks space permissions |

Not established: CQL `lastmodified` precision and whether `order by lastmodified asc` is accepted; the fixtures declare one form, the status row says fixtures only.
