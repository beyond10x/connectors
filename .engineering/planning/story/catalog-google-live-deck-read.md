---
format: aep.planning-md/3
id: story:catalog-google-live-deck-read
kind: story
status: draft
title: The operator's deck is read live through browser-consented Drive and Slides connections
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-google-slides-reads
- depends_on: story:cli-oauth-loopback-acquisition
scope:
- confidence: inferred
  path: docs/catalog-google-drive.md
- confidence: inferred
  path: docs/catalog-google-oauth.md
- confidence: inferred
  path: docs/catalog-google-slides.md
revision: 2
---
## Outcome

The operator's deck `1HurXAVXiOQszM7a_tpgYqRj5Uhy3gT_W6Iz-X4ZVYWY` is read live, as text and as
structure, through connections created by browser consent. This is the epic's live-run acceptance,
held in its own item so the provider chain does not wait on the Google Cloud console.

## Acceptance

Each observation recorded with `aep plan artifact evidence` against this story, naming the command:

- `connectors connections connect <google-drive instance> --credential-file <client.json>` and the
  same for a `google-slides` instance complete with browser consent, and
  `connectors connections status` reports both ready.
- `connectors operations invoke <google-drive instance> files.export` with
  `{"fileId": "1HurXAVXiOQszM7a_tpgYqRj5Uhy3gT_W6Iz-X4ZVYWY", "mimeType": "text/plain"}` exits 0 and
  returns non-empty text.
- `connectors operations invoke <google-slides instance> presentations.get` with
  `{"presentationId": "1HurXAVXiOQszM7a_tpgYqRj5Uhy3gT_W6Iz-X4ZVYWY"}` exits 0 and the response's
  `presentationId` equals the deck id.
- A second invoke more than 60 s before the access token expires makes no token request
  (the refresh profile's cache), observed in the connectors execution audit.
- `docs/catalog-google-drive.md` and `docs/catalog-google-slides.md` link
  `docs/catalog-google-oauth.md`, and the OAuth guide's steps match what the operator did.

## Out of scope

Calendar, Gmail and writes live runs.
