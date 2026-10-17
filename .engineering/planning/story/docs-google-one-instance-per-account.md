---
format: aep.planning-md/3
id: story:docs-google-one-instance-per-account
kind: story
status: implemented
title: The four Google API guides configure one instance per account
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: docs/catalog-google-calendar.md
- confidence: inferred
  path: docs/catalog-google-drive.md
- confidence: inferred
  path: docs/catalog-google-gmail.md
- confidence: inferred
  path: docs/catalog-google-slides.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T19:08:21Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T19:08:22Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:42Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

The Google guides configure a read instance and a separate write instance per API, because a
connection's scopes cannot be widened in place (`docs/catalog-google-gmail.md` § Authentication;
`docs/waves/wave-20260930c-google-oauth-discovery/README.md:38`): four APIs with writes are eight
instances and eight consents. After story:catalog-connector-declarations each Google connector's
auth profile requests every scope its operations need, so one instance per account suffices.
`docs/catalog-google-oauth.md` (publishing advice, reconnect) is in
story:docs-google-publishing-advice.

Part of the docs set (the `docs` skill: `website/docs`, `README.md` and `AGENTS.md` as one set); the
terms are defined once in `README.md` by story:docs-vocabulary-readme-website and linked from here.

## Change

- `catalog-google-{gmail,drive,calendar,slides}.md`: one instance per account (`gmail/private`),
  configured by name; the separate write-instance sections go. Each guide says which scopes the
  single consent requests.
- Existing read/write pairs: the procedure of `migration-plan:connector-vocabulary` step 8 (add the
  merged instance, connect it, then revoke the old pair), because `setup migrate` does not merge
  pairs.
- Text the change does not make stale is left alone. Commands shown are re-run and their output
  pasted from the run; a live Google run waits on `credential-blocker:google-oauth-client`, so
  output is from the HTTPS fixture and marked as such.

## Domain relations

None asserted; definitions are linked from `README.md`.

## Scope

- `docs/catalog-google-{gmail,drive,calendar,slides}.md`

## Acceptance

`connectors-docs vocabulary --no-pins --one-instance` (story:docs-vocabulary-readme-website) exits 0
over `docs/catalog-google-{gmail,drive,calendar,slides}.md`.
