---
format: aep.planning-md/3
id: story:cli-spec-mapping-fixes
kind: story
status: draft
title: CLI spec matches the owner greeting and cites the contract correctly
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Source

ESS design review of the CLI domain (2026-09-30).

## Acceptance

- `LocalOwnerGreeting` (`ess/domains/cli.yaml:146-152`) declares the optional `build` field the host sends
  (`crates/connectors-host/src/local/owner.rs:358-360`), and the fallback `build` request is modelled.
- Stale citations fixed: `cli.yaml:692` → semantics.md:300, `:814` → :304; comments at `:80` and `:285` name ESS 0.45.
- `ess specify validate` passes; definitions and CLI contract regenerate.
