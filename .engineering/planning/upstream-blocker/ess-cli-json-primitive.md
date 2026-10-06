---
format: aep.planning-md/3
id: upstream-blocker:ess-cli-json-primitive
kind: upstream-blocker
status: cleared
title: ESS CLI generator refuses the Json primitive for result fields
refs:
- provider: github
  reference: beyond10x/ess#468
relations:
- blocks: story:cli-json-answers-as-json
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-06T23:25:52Z", actor: "human:timo", revision: 2}
---
Filed upstream: https://github.com/beyond10x/ess/issues/468 (2026-10-06, by the bot); the ess-ship
session was told the same day.

ESS 0.53.0 `ess generate cli` refuses a CLI result field typed `Json` ("unsupported CLI primitive
`Json`"): the CLI contract resolver maps only `String`, `Boolean` and `Integer`
(`crates/specify/ess-cli-contract/src/resolve.rs:37-42` at tag 0.53.0). `story:cli-json-answers-as-json`
declares `OperationDescription.input_schema`, `output_schema` and `OperationInvokeResult.result` as
`Json` in `ess/domains/cli.yaml`; regeneration refuses, and the generated result check refuses an
object there as `cli_result`.

State on branch `feat/cli-json-answers-as-json`: the `cli.yaml` types, the contract fixtures and
semantics, and four failing tests (`describe_and_invoke_answer_json_values_not_json_text` and the
changed `dynamic_business_sources_are_acquired_once_and_validate_both_directions` in
`crates/connectors-conformance/tests/cli_surface.rs`; `operations_describe_answers_schemas_as_json_objects`
and `adapters_describe_answers_operation_schemas_as_json_objects` in `apps/connectors/tests/json_answers.rs`).
The CLI handler change in `apps/connectors/src/local/operations.rs` waits for the regenerated contract.

Cleared when an ESS release maps `Json` for CLI result fields and connectors pins it.
