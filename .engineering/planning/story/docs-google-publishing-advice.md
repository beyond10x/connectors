---
format: aep.planning-md/3
id: story:docs-google-publishing-advice
kind: story
status: implemented
title: The Google OAuth guide says a personal-use app can be published without verification
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: docs/catalog-google-oauth.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T19:08:22Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T19:08:23Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:42Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

`docs/catalog-google-oauth.md:117-118` says publishing the OAuth app lifts the 7-day refresh-token
expiry "subject to Google's verification for sensitive scopes". An app used only by its developer
may be set to *In production* without verification (personal use, fewer than 100 users;
support.google.com/cloud/answer/13464323, quoted in `docs/design/connector-vocabulary.md`
§ Consequences). The same guide's § Repair names `connections repair --adapter` (lines 105-107).

Part of the docs set (the `docs` skill: `website/docs`, `README.md` and `AGENTS.md` as one set); the
terms are defined once in `README.md` by story:docs-vocabulary-readme-website and linked from here.

## Change

- § Testing-mode expiry: state that a personal-use app can be set to *In production* without
  verification, which lifts the 7-day expiry, with the source above; one consent per instance
  (`gmail/private` and `drive/private` are two).
- § Repair becomes § Reconnect: `connections reconnect --connector … --instance …`, and the `Stale`
  state a refused refresh token leaves the connection in (story:connection-stale-state).
- Obligation, not the acceptance: `connectors-docs vocabulary docs/catalog-google-oauth.md` exits 0.
- Text the change does not make stale is left alone.

## Domain relations

None asserted; definitions are linked from `README.md`.

## Scope

- `docs/catalog-google-oauth.md`

## Acceptance

`docs/catalog-google-oauth.md` § Testing-mode expiry no longer says publishing is subject to
verification and instead states, citing support.google.com/cloud/answer/13464323, that an app used
only by its developer can be set to *In production* without verification, which lifts the 7-day
refresh-token expiry.
