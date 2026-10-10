---
format: aep.planning-md/3
id: story:parity-gitlab-ci-writes
kind: story
status: active
title: GitLab pipeline retry, cancel and create; environments
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T09:55:31Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T09:55:31Z", actor: "human:timo", revision: 3}
---
## Outcome

GitLab pipeline retry, cancel and create; environments — parity unit U15 of `docs/fluxplane-plugin-parity.md`.

## Operations

`gitlab.pipeline.retry`, `gitlab.pipeline.cancel`, `gitlab.pipeline.create`, `gitlab.environment.list`

36 calls since 2026-09-09 (declared and mapped undeclared names), provider `gitlab`, planned wave W6.

## Surface

catalog `operations.json` selection.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
