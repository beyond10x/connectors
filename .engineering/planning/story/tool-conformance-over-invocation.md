---
format: aep.planning-md/3
id: story:tool-conformance-over-invocation
kind: story
status: active
title: The Tavily contract's implementation passes tool-conformance/1 through the adapter's invoke
summary: AdapterToolset over connectors_sdk::Adapter, a served fake provider, scenarios and mutants
relations:
- decomposes: epic:tool-contract-publication
- serves: vision:independent-contract-adapters
- depends_on: story:connector-projection
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: adapters/tavily/spec/tool-conformance
- confidence: inferred
  path: crates/connectors-tool-contracts/src/toolset.rs
- confidence: inferred
  path: crates/connectors-tool-contracts/tests/tavily_conformance.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T15:33:29Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-26T15:33:33Z", actor: "human:timo", revision: 4}
---
## Outcome

The published Tavily contract is evidence about an implementation: `AdapterToolset`, a
`tool_contract::Toolset` over an adapter's governed `invoke`, passes a `tool-conformance/1` suite run
by `tool_contract::conformance::run_with`, against a local fake of the provider's API served by the
runner. A mutant that drops a declared property fails at least one scenario.

## Design

- `connectors_tool_contracts::AdapterToolset`: holds the published contract, the descriptor, an
  `Arc<dyn connectors_sdk::Adapter>`, the omitted fields' values and a Tokio runtime handle.
  - `invoke("connector-<instance>.<name>", input)` refuses an input key named in `omit`
    (`connector.FieldNotAllowed`), adds the omitted values, calls the adapter's `invoke` with the
    descriptor operation id, and answers its output, or `connector.Failed {code}` from its error.
  - `schemas()` answers each operation's descriptor schemas with string enums typed `string`, so the
    runner's shape check compares the contract with what the adapter validates.
- Suite `adapters/tavily/spec/tool-conformance/*.yaml`: per operation one scenario holding `effects`,
  `idempotency`, `determinism`, `bounds` and `output` (and `subjects` where declared) over
  `fixture.serve` routes `/search`, `/extract`, `/crawl`, and refusal scenarios.
- The Tavily composition lives in this crate's tests (dev-dependency on `connectors-tavily` and
  `connectors-host`), so the library keeps no concrete adapter (adapter boundary gate).

## Scope

- `crates/connectors-tool-contracts/src/toolset.rs`, `crates/connectors-tool-contracts/tests/tavily_conformance.rs`
- `adapters/tavily/spec/tool-conformance/` (new)
- `Cargo.lock`: shared, merged serially

## Acceptance

`crates/connectors-tool-contracts/tests/tavily_conformance.rs`:

- `tavily_passes_tool_conformance`: the committed contract equals the published one, the report's
  `passed()` is true, all 7 scenarios ran, no schema mismatch, nothing missing; the test prints the
  report's count line.
- Scenarios in the suite: `search-answers-results`, `search-reports-a-short-answer`,
  `fetch-answers-pages`, `crawl-answers-pages`, `upstream-refusal-is-connector-failed` (a `401`
  answers `connector.Failed` with `code: unauthorized`), `plan-limit-is-capacity` (a `432` answers
  `code: capacity`), `input-outside-schema-is-invalid-input`.
- `omitted_field_is_refused_and_filled`: a contract published with `omit: [country]` refuses an input
  carrying `country` with `connector.FieldNotAllowed` and sends the connection's value upstream.
- `every_mutant_fails_a_scenario`: the mutants `undeclared-error`, `drops-results`,
  `swallows-refusal` and `drops-code` each fail at least one scenario
  (`tool_contract::mutants::each_mutant_fails`).

Changed from the first draft: no `search-sends-the-key-as-bearer` scenario. A `tool-conformance/1`
file expectation is `equals`, `sha256` or `$lines` (b10x-tool-contract 0.5.0,
`crates/tool-contract/src/conformance.rs:517`), so the suite cannot hold one header of a logged
request; the bearer header stays held by `adapters/tavily/tests/protocol.rs`. The `omit` mutant is
`omitted_field_is_refused_and_filled`, which fails when the refusal or the fill is removed.
