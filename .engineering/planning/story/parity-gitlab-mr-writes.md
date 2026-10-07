---
format: aep.planning-md/3
id: story:parity-gitlab-mr-writes
kind: story
status: draft
title: GitLab merge-request notes, discussions, auto-merge and reopen
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

GitLab merge-request notes, discussions, auto-merge and reopen — parity unit U09 of `docs/fluxplane-plugin-parity.md`.

## Operations

`gitlab.mr.merge`, `gitlab.mr.update`, `gitlab.mr.note.create`, `gitlab.mr.discussion.reply`, `gitlab.mr.discussion.resolve`

159 calls since 2026-09-09 (declared and mapped undeclared names), provider `gitlab`, planned wave W2.

## Surface

catalog `operations.json` selection: notes, discussion reply and resolve, guarded merge-when-pipeline-succeeds and reopen.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
