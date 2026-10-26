---
format: aep.planning-md/3
id: approval-record:wave-20261026-tool-contracts
kind: approval-record
status: approved
title: 'Wave 20261026-tool-contracts: ADR 0103 tool contract publication'
summary: The orchestrator's row 12 approval of the tool contract wave
relations:
- decides: story:tool-contract-model
- decides: story:connector-projection
- decides: story:tool-conformance-over-invocation
- decides: epic:tool-contract-publication
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-26T15:33:36Z", actor: "human:timo", revision: 2}
---
## Approval

The directing session `b10x-chef-orchestrator`, 2026-10-26, dispatch-queue row 12, under the grant
the operator gave it for class-C items in this workspace (verbatim):

> Atlas ADR 0103 (b10x-tool-contract, accepted; ...) in connectors: find what the ADR asks of
> connectors, file it in the store as an epic with stories (Scope per story), implement what is one
> wave's worth (the contract surface the ADR names, with ESS scenarios first), gate, bot PR, merge by
> the route, release per AGENTS.md if the release process allows (otherwise land and say so).

## Wave 20261026-tool-contracts

| story | decided |
|---|---|
| `story:tool-contract-model` | in the wave |
| `story:connector-projection` | in the wave |
| `story:tool-conformance-over-invocation` | in the wave |
| `story:tool-contracts-for-catalog-connectors` | later wave |
| `story:tool-contract-serving` | later wave |

Calls taken inside the item, by the implementing session:

- The three stories land on one branch `wave/20261026-tool-contracts`, serially: they share
  `Cargo.lock` and `crates/connectors-tool-contracts/`.
- `b10x-tool-contract` is pinned to tools tag `0.5.0`, its newest release. Its `rust-version` is
  1.98, so `connectors-tool-contracts` is left out of the local `--msrv` Rust 1.91 workspace check;
  CI builds on Rust 1.98.1.
- The contract's toolset is `connector-<instance>`, the name the moved tools story used.
