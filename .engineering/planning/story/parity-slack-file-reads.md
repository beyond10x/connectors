---
format: aep.planning-md/3
id: story:parity-slack-file-reads
kind: story
status: implemented
title: Slack file list, info and download
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-binary-responses
- depends_on: story:catalog-slack-reads
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T14:23:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T14:23:54Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T16:33:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

Slack file list, info and download through the catalog provider: the read part of parity unit U13 of `docs/fluxplane-plugin-parity.md`. Upload and delete stay in story:parity-slack-files with the Slack writes.

## Operations

`slack.file.download` (11 calls), `slack.file.info` (7), `slack.file.list` (3): 21 calls since 2026-09-09.

## Surface

The Slack catalog selection (`files.list`, `files.info`) and the file download of `url_private`, which needs the binary responses of story:catalog-binary-responses and a host the Slack connection admits for file content.

## Acceptance

- Spec first: the selection and the file record (name, type, size) are modelled in the Slack specification and validated with the newest `ess` before implementation.
- Each operation answers through `connectors operations invoke` on a saved connection against a recorded provider fixture, with the same capability the fluxplane operation gives.
- A download URL on a host the connection does not admit is refused by name.
- The parity page rows move to covered, with the Connectors operation named.
