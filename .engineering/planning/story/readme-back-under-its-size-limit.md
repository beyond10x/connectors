---
format: aep.planning-md/3
id: story:readme-back-under-its-size-limit
kind: story
status: active
title: The README is back under its 27,000-byte limit, held by a test
tags:
- from-finding
relations:
- informed_by: story:docs-first-run-path
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: README.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:47Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:48Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

`story:docs-first-run-path` held the README under 27,000 bytes (25,705 at its merge `5c2fb81e9`). On
`origin/main` `8150245c2` `README.md` is 31,386 bytes (`wc -c`), and no gate step checks the size.
Found by the wave D417 store-close check, 2026-11-03.

## Acceptance

- `README.md` is at most 27,000 bytes again, or the limit is restated with its reason.
- A test in `crates/connectors-docs` fails when the README passes the limit.
