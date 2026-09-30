---
format: aep.planning-md/3
id: story:catalog-parameters-declare-their-type
kind: story
status: implemented
title: Catalog parameters declare the provider's type instead of string|integer|boolean
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/generated/bundles/confluence.bundle.json
- confidence: cited
  path: adapters/catalog/generated/bundles/gitlab.bundle.json
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/jira.bundle.json
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/engine.rs
- confidence: cited
  path: adapters/catalog/tests/parameter_types.rs
- confidence: cited
  path: adapters/catalog/tests/parameter_types_adversary.rs
- confidence: cited
  path: crates/connectors-catalog/src/authored.rs
- confidence: cited
  path: crates/connectors-catalog/src/inventory.rs
- confidence: cited
  path: crates/connectors-catalog/tests/authored.rs
- confidence: cited
  path: crates/connectors-catalog/tests/template.rs
- confidence: cited
  path: crates/connectors-catalog/tests/template_adversarial.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-09-30T16:53:25Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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
