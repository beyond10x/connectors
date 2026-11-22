---
format: aep.planning-md/3
id: story:drive-file-content-read
kind: story
status: implemented
title: Drive returns an uploaded file's content, such as a PDF
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:31Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:32Z", actor: "human:operator", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-13T07:29:38Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Need

Teams read PDFs and other uploaded files from Drive. `files.get` returns metadata only, "its content
is not downloaded here" (`adapters/catalog/providers/google-drive/operations.json:16`); `files.export`
covers Google Docs, Sheets and Slides only.

## Acceptance

- A Drive read returns an uploaded file's content (`files.get` with `alt=media`) as bytes in the
  same shape and bound as a Gmail attachment read, refused above the binary read limit with the file
  size named.
- It works under the read-only Drive scope.
- A protocol test reads a PDF fixture and refuses an oversized one and a Google Docs file (pointing
  to `files.export`).
