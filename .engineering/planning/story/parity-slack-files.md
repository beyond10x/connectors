---
format: aep.planning-md/3
id: story:parity-slack-files
kind: story
status: draft
title: Slack file upload
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
- depends_on: story:parity-slack-file-reads
- depends_on: story:catalog-composed-write
revision: 7
---
## Outcome

Slack file upload: `slack.file.upload`, the last open operation of parity unit U13 of
`docs/fluxplane-plugin-parity.md` (the reads are implemented in `story:parity-slack-file-reads`,
the delete in `story:parity-slack-file-delete`).

## Operations

`slack.file.upload`

24 calls since 2026-09-09, provider `slack`. `slack.file.list`, `slack.file.info`,
`slack.file.download` and `slack.file.delete` are covered (`docs/fluxplane-plugin-parity.md` rows
at lines 593-597).

## Surface

catalog engine change: binary download and a multi-step external upload.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Domain draft

Drafted 2026-10-07 with `adapters/slack/spec/ess`; the file record (name, type, size) is UNMAPPED there. Binary download and the multi-step upload decide whether the catalog engine can carry this unit.

## Split

The reads (`slack.file.list`, `slack.file.info`, `slack.file.download`, 21 calls) moved to story:parity-slack-file-reads on 2026-10-10. Upload and delete (31 calls) stay here, held with the Slack message writes.

## Plan

Scheduled in phase P3 of the epic's plan of 2026-10-19. The delete half of this unit
(`files.delete`, 7 calls) moved to story:parity-slack-file-delete on 2026-10-19 (decision D2 of the
plan: a plain selection, shipped with the Slack single-user reads). What stays here is the upload
(`slack.file.upload`, 24 calls): `files.getUploadURLExternal`, the bytes to the answered URL, then
`files.completeUploadExternal` with channel and thread as caller inputs, after
story:catalog-composed-write (depends_on). The reads of the unit are implemented
(story:parity-slack-file-reads).
