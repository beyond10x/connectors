---
format: aep.planning-md/3
id: story:parity-gitlab-mr-reads
kind: story
status: implemented
title: GitLab merge-request diffs and discussions
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T02:00:38Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T02:00:38Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T03:47:23Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

GitLab merge-request diffs and discussions — parity unit U11 of `docs/fluxplane-plugin-parity.md`.

## Operations

`gitlab.mr.changes`, `gitlab.mr.discussion.list`

106 calls since 2026-09-09 (declared and mapped undeclared names), provider `gitlab`, planned wave W4.

## Surface

catalog `operations.json` selection: diffs, discussions, notes.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
