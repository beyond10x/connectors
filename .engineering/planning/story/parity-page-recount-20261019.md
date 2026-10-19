---
format: aep.planning-md/3
id: story:parity-page-recount-20261019
kind: story
status: implemented
title: The parity page carries the recount of 2026-10-18
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: docs/fluxplane-plugin-parity.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:38Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:39Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-19T05:54:48Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`docs/fluxplane-plugin-parity.md` states what `main` serves on 2026-10-19: a dated recount
section replaces the amendment list as the current state, the per-plugin table and the gap-unit
table carry the recount's verdicts, every `operations.json` and `adapter.json` citation points at
its current line, and the page records that the old `flux` GitLab plugin declares the same 64
operations as the fluxplane one. The epic's plan of 2026-10-19 cites the recount; this story makes
the page the source it cites.

## Input

The recount of 2026-10-18 against `c15004d81` (read-only; its file is outside the repository and
is cited by date): covered 112 operations / 5,957 calls, partial 6 / 5, missing 183 / 394; 17 of 25
units closed, 8 open with 407 calls; three verdicts moved on 0-call operations
(`kubernetes.node.list` to covered by `nodes.list`, `opsgenie.test` to covered by the page's own
`*.test` rule and the identity read, `loki.recent_logs` to covered because the page already marks
the same capability covered through Grafana). The flux map: 40 covered, 5 partial, 19 missing of
64, all 19 with 0 calls.

## Acceptance

- The Summary, per-plugin and gap-unit tables agree with each other and with the recount's
  arithmetic (a reviewer can sum the rows); the sentence under Summary gives the current split.
- Every citation of the form `operations.json:<line>` or `adapter.json:<line>` on the page resolves
  to the named operation at that line on `main` (`grep -n` of each key).
- story:parity-confluence-cql-search-v1-held is archived with this story's commit: U20 is covered
  by `confluence-v1` (`adapters/catalog/providers/confluence-v1/operations.json`), and the held
  story names no remaining work.
- Documentation-only: no runtime change, no live call; `connectors-docs vocabulary` passes (the
  page keeps its dated-record banner).

## Scope

Primary: `docs/fluxplane-plugin-parity.md`. No collision with any other story.
