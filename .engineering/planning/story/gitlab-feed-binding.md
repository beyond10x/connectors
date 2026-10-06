---
format: aep.planning-md/3
id: story:gitlab-feed-binding
kind: story
status: active
title: GitLab binds the feed family
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
- depends_on: story:feed-bindings-discoverable
- depends_on: story:catalog-feed-engine
- depends_on: story:catalog-feed-engine-extensions
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T19:55:10Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-06T19:55:10Z", actor: "agent:claude", revision: 3}
---
## Outcome

The GitLab adapter binds `datasource.feed/v1alpha1`: its containers and items are readable through the family, with the binding's own profile, under `adapters/catalog/contracts/feed/v1alpha1/`.

## Acceptance

The family's conformance suite passes against the binding with recorded provider fixtures, and the binding's semantics state: what a container is (a project (merge requests and issues as items)), how the watermark is formed, how far a first read reaches, whether deletions are observed, and how direct or private material is excluded.

## Depends on

`story:feed-contract`, `story:feed-bindings-discoverable`.

## Decided 2026-10-06 (wave 20261006d)

## Decided 2026-10-06 (wave 20261006d)

- Items are merge requests only: one connection carries one binding with the fixed ids, so one profile yields one item kind.
- Visibility: every project maps to `private` until visibility can be read from two fields (a public project with members-only merge requests must not be listed public).
- `url` stays null: a project rename changes `web_url` without changing the merge request revision.
- The unit's binding and engine patch are kept in the worktree archive `connectors-w4-gitlab`.
