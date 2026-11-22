---
format: aep.planning-md/3
id: story:slack-sdk-upstream-import
kind: story
status: draft
title: Slack request shapes come from node-slack-sdk at a pinned commit
relations:
- decomposes: epic:slack-complete-web-api
revision: 1
---
## Outcome

The Slack request shapes come from `slackapi/node-slack-sdk` at a pinned commit
(`packages/web-api/src/methods.ts` and `packages/web-api/src/types/request/`), replacing the
2021 OpenAPI v2 source. The imported document lists every method `methods.ts` binds (303 on
2026-11-23) with its arguments, required or optional, and their types.

## Change

- `connectors-build definition import` gains a node-slack-sdk source: it reads the two paths at a
  commit named with its SHA-256, and writes a projection the definition reads (the place
  `slack-web.openapi.json` holds today). The importer is Rust; no Python.
- The committed upstream file carries the commit, the retrieval date and the SHA-256, like
  `adapters/slack/upstream/README.md` does today. Tests read the committed file; no network.
- Amendments that still apply to the new source are carried; the rest are dropped with a reason.

## Acceptance

- The projection lists 303 methods for the pinned commit (or the count `methods.ts` has at the
  commit chosen), each with its arguments; a test compares the count and five named methods'
  arguments (`conversations.replies`, `chat.postMessage`, `files.getUploadURLExternal`,
  `apps.manifest.create`, `apps.connections.open`) against the TypeScript source.
- A changed source byte fails the digest check.

## Scope

connectors: `crates/connectors-build` (definition import), `adapters/slack/upstream/`.
connectors-definitions: `upstream/slack/`, `upstream/slack-user/`. Inferred.
