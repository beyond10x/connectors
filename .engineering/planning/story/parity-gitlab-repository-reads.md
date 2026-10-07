---
format: aep.planning-md/3
id: story:parity-gitlab-repository-reads
kind: story
status: draft
title: GitLab code search and repository tree
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

GitLab code search and repository tree — parity unit U10 of `docs/fluxplane-plugin-parity.md`.

## Operations

`gitlab.search.blobs`, `gitlab.repository.tree`

125 calls since 2026-09-09 (declared and mapped undeclared names), provider `gitlab`, planned wave W3.

## Surface

catalog `operations.json` selection: code search, tree, single commit, commit diff, branch list.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
