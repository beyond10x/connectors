---
format: aep.planning-md/3
id: story:gitlab-feed-binding
kind: story
status: draft
title: GitLab binds the feed family
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
- depends_on: story:feed-bindings-discoverable
revision: 1
---
## Outcome

The GitLab adapter binds `datasource.feed/v1alpha1`: its containers and items are readable through the family, with the binding's own profile, under `adapters/catalog/contracts/feed/v1alpha1/`.

## Acceptance

The family's conformance suite passes against the binding with recorded provider fixtures, and the binding's semantics state: what a container is (a project (merge requests and issues as items)), how the watermark is formed, how far a first read reaches, whether deletions are observed, and how direct or private material is excluded.

## Depends on

`story:feed-contract`, `story:feed-bindings-discoverable`. 
