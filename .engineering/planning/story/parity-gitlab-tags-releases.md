---
format: aep.planning-md/3
id: story:parity-gitlab-tags-releases
kind: story
status: implemented
title: GitLab tags and releases
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T04:32:49Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T04:32:50Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T09:10:39Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

GitLab tags and releases — parity unit U14 of `docs/fluxplane-plugin-parity.md`.

## Operations

`gitlab.release.create`, `gitlab.repository.tag.create`, `gitlab.repository.tag.show`, `gitlab.release.show`, `gitlab.release.link.list`, `gitlab.release.update`, `gitlab.repository.tag.delete`

45 calls since 2026-09-09 (declared and mapped undeclared names), provider `gitlab`, planned wave W5.

## Surface

catalog `operations.json` selection.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
