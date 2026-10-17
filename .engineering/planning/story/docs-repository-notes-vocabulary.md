---
format: aep.planning-md/3
id: story:docs-repository-notes-vocabulary
kind: story
status: implemented
title: The living repository notes use the connector vocabulary; dated records carry a pointer
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: docs/design.md
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: docs/live-e2e.md
- confidence: cited
  path: docs/verification.md
- confidence: cited
  path: docs/website-verification.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T22:01:00Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-10-16T22:01:00Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:43Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
## Context

The vocabulary check from story:docs-vocabulary-readme-website, run on 2026-10-16 at wave head
0255333f1 over `docs/*.md`, reported 247 findings in 12 files. Five of those files are living
documents that describe how the repository works today. The other seven are dated or historical
records. The coordinator decided the dated records stay as written, because rewriting a record
of what was said on a date falsifies it:

| kept as written | reason |
|---|---|
| `docs/review-response-2026-09-08.md`, `docs/independent-review-response-2026-09-08.md` | dated review records |
| `docs/recent-adapter-usage-20260909.md` | dated usage record |
| `docs/cli-migration-v1-to-v2.md` | migration record for an earlier CLI version |
| `docs/fluxplane-plugin-parity.md`, `docs/stack-integration-proposal.md` | proposals, not descriptions of shipped behaviour |
| `docs/website-design.md` | the design record for the site |

## Change

- `docs/design.md`, `docs/development.md`, `docs/verification.md`, `docs/live-e2e.md` and
  `docs/website-verification.md` use provider, connector, auth profile, instance, connection and
  operation wherever they describe what a user configures, selects or connects. "adapter" stays
  only where it names the internal implementation, and that line says so ("internal").
- `docs/design.md` links `README.md#vocabulary` and `docs/design/connector-vocabulary.md` once
  near the top, instead of defining the terms again.
- Each dated record in the table above gains one first-line note that it predates the connector
  vocabulary and links `README.md#vocabulary`; its body is unchanged.

## Scope

- `docs/design.md`
- `docs/development.md`
- `docs/verification.md`
- `docs/live-e2e.md`
- `docs/website-verification.md`
- the seven dated records above (first line only)

## Acceptance

`connectors-docs vocabulary docs/design.md docs/development.md docs/verification.md
docs/live-e2e.md docs/website-verification.md` exits 0. It exits 1 on the same files at
0255333f1.
