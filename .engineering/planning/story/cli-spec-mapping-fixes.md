---
format: aep.planning-md/3
id: story:cli-spec-mapping-fixes
kind: story
status: active
title: CLI spec matches the owner greeting and cites the contract correctly
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: ess/domains/cli.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:06:17Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T11:06:18Z", actor: "human:timo", revision: 5}
---
## Source

ESS design review of the CLI domain (2026-09-30).

## Acceptance

- `LocalOwnerGreeting` (`ess/domains/cli.yaml:146-152`) declares the optional `build` field the host sends
  (`crates/connectors-host/src/local/owner.rs:358-360`), and the fallback `build` request is modelled.
- Stale citations fixed: `cli.yaml:692` → semantics.md:300, `:814` → :304; comments at `:80` and `:285` name ESS 0.45.
- `ess specify validate` passes; definitions and CLI contract regenerate.
