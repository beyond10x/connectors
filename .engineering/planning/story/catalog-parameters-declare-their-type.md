---
format: aep.planning-md/3
id: story:catalog-parameters-declare-their-type
kind: story
status: active
title: Catalog parameters declare the provider's type instead of string|integer|boolean
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: inferred
  path: adapters/catalog/tests/engine.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 5}
---
## Source

Split from story:cli-surface-minor-findings-0-18-0 (D12), found by the 0.18.0 CLI surface test. `declare` gives every
catalog parameter `["string","integer","boolean"]` (`adapters/catalog/src/lib.rs:677,718`), so `per_page=true` and
`limit=true` pass the host's schema check; the bound check refuses non-integers only for bounded parameters.

## Acceptance

- A catalog parameter's declared input type follows the pinned document's schema type (integer, string, boolean),
  keeping string forms of integers where the engine already accepts them.
- `per_page=true` and `limit=true` are refused as `invalid_input` before any request; the GitLab, Jira and Confluence
  bundles and the descriptor revision change only as stated in the CHANGELOG.
