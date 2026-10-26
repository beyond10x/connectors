---
format: aep.planning-md/3
id: story:tool-contract-serving
kind: story
status: draft
title: A host reads an instance's tool contract from the CLI and the client library
summary: connectors operations contract and connectors_client::tool_contract answer the admitted operations' document
relations:
- decomposes: epic:tool-contract-publication
- serves: vision:independent-contract-adapters
- depends_on: story:connector-projection
revision: 1
---
## Outcome

A host asks Connectors for the tool contract of one configured instance and gets the
`tool-contract/1` document of exactly the operations that instance admits, from the CLI and from the
Rust client library. The host's `catalogue` implementation (tools `story:toolset-tool-meta`) reads it
there; tools never reads a descriptor (ADR 0103 decisions 3 and 6).

## Design

- CLI `connectors operations contract --instance <i>` (ESS-generated CLI contract, `connectors.cli`),
  printing the YAML document; `--format json` prints it as JSON.
- `connectors_client` gains `tool_contract(instance)`, answering the same document.
- An instance without tool profiles answers a refusal naming the missing profiles, never a contract
  with defaulted profiles.

## Scope

- `ess/domains/cli.yaml`, `apps/connectors-cli-contract/` (regenerated), `apps/connectors/`
- `crates/connectors-client/`
- `website/docs/guides/` (one guide section), regenerated CLI reference

## Acceptance

- `operations_contract_prints_the_instance_contract`: for a configured Tavily instance the document
  validates and names `connector-<instance>.search`, `.fetch`, `.crawl`.
- `operations_contract_refuses_an_instance_without_profiles`.
- The CLI reference regenerates without drift.

## Not in this story

- MCP-backed operations: `decision-blocker:mcp-outbound-stdio-process-ownership`.
- Which connection a `tool.invoke` of a connector contract runs under for an inbound MCP caller:
  `decision-blocker:mcp-caller-connection-assignment`.
