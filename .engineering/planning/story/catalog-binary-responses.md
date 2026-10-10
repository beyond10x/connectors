---
format: aep.planning-md/3
id: story:catalog-binary-responses
kind: story
status: implemented
title: Binary responses through the catalog engine
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T14:23:53Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T14:23:54Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T16:33:08Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

The catalog engine answers an operation whose success response is binary (`application/octet-stream`, an image, a PDF, any non-JSON, non-text media type the selection declares) with the bytes, bounded and typed, instead of parsing them as JSON or refusing. Today a selection reads a response as JSON or as text only (`ResponseKind`, `adapters/catalog/tests/google_drive.rs:240-245`).

## Acceptance

- Spec first: the binary response kind and the shape of a binary result (media type, byte length, content digest, and the bytes or where they were written) are modelled in the catalog ESS specification and the selection schema, validated with the newest `ess`, and generated before implementation.
- A selection that declares a binary response answers through `connectors operations invoke` with that shape against a recorded fixture; a response over the declared size bound is refused by name, not truncated silently; a JSON or text selection is unchanged.
- A redirect to another host is followed only to a host the connection admits; any other host is refused by name.

## Consumers

story:parity-slack-file-reads and story:parity-jira-attachment-download.
