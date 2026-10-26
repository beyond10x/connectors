---
format: aep.planning-md/3
id: story:connector-projection
kind: story
status: implemented
title: An instance's descriptor and its authored profiles publish one tool-contract/1 document
summary: 'Moved from tools by ADR 0103 decision 5: profiles and omit beside the adapter, publish, Tavily contract drift-checked'
refs:
- provider: tools
  reference: story:connector-projection
relations:
- decomposes: epic:tool-contract-publication
- serves: vision:independent-contract-adapters
- depends_on: story:tool-contract-model
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: adapters/tavily/generated/tool-contract.yaml
- confidence: inferred
  path: adapters/tavily/spec/tool-profiles.json
- confidence: inferred
  path: crates/connectors-build/src/gate.rs
- confidence: inferred
  path: crates/connectors-tool-contracts
- confidence: inferred
  path: website/docs/reference
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T15:33:28Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-26T15:33:33Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-26T16:50:01Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Connectors turns an instance's descriptor and the tool profiles authored beside its adapter into one
`tool-contract/1` document, one operation per admitted operation, each with the full profile the
adapter's author declared, and `b10x-tool-contract`'s validator accepts it. The Tavily adapter is the
first with profiles, and its generated contract is committed and drift-checked by the gate.

Moved here from tools by Atlas ADR 0103 decision 5 (accepted 2026-10-21). The tools copy is archived
with a pointer to this story. Its host-side `tool-projection/1` document is not built: the profile and
`omit` are Connectors' operation definition.

## Design

- Crate `crates/connectors-tool-contracts` (library `connectors_tool_contracts`, binary
  `connectors-tool-contracts`). It depends on `connectors-core` and on `b10x-tool-contract` by git pin
  on tools tag `0.5.0` (ADR 0103 decision 4). That crate's `rust-version` is 1.98, so this crate
  declares 1.98 and the gate's `--msrv` Rust 1.91 workspace check excludes it.
- Declaration `adapters/<adapter>/spec/tool-profiles.json`, format `connectors.tool-profiles/1`:
  `summary`, and per descriptor operation id: `name` (the `<op>` segment), optional `summary`,
  `guidance`, `profile` (a `tool-contract/1` profile, read with that crate's own type) and `omit`
  (input fields the connection fills).
- `publish(descriptor, declarations) -> Result<Contract, Vec<Refusal>>`:
  - toolset and family `connector-<instance>`, version `v1alpha1`; an instance outside
    `[a-z][a-z0-9-]*` is refused (`name-invalid`).
  - every descriptor operation needs a declaration with a profile; one without is refused naming it
    (`profile-missing`); a declaration for an operation the descriptor lacks is refused too.
  - JSON Schema to field lists: `string`, `integer`, `boolean`, `array` with `items`, `object` with
    `properties`, `object` with only `additionalProperties` as `Map<String, V>`, a string `enum` (typed
    or not) as an enum type, `["T","null"]` or a property outside `required` as `Optional<T>`, `$ref`
    into `$defs`. Nested types are named `<op>.<Path>`. Anything else (`number`, `oneOf`, `anyOf`, an
    untyped node) is refused naming the operation and the schema path (`schema-unrepresentable`).
  - `omit` fields leave the published input; naming a field the input lacks is refused.
  - errors: `connector.Failed {code: connector.ErrorCode}` on every operation, and
    `connector.FieldNotAllowed {field}` on an operation with `omit`.
  - the result passes `tool_contract::validate`; a violation (rule `risk` is the risk floor) is a
    refusal (`risk-below-floor` for rule `risk`).
  - the document names no credential, configuration value or connection.
- `connectors-tool-contracts generate [--check]` writes or checks
  `adapters/tavily/generated/tool-contract.yaml` from `adapters/tavily/generated/descriptor.json` and
  `adapters/tavily/spec/tool-profiles.json`; the gate runs `--check`.

## Scope

- `crates/connectors-tool-contracts/` (new)
- `adapters/tavily/spec/tool-profiles.json` (new), `adapters/tavily/generated/tool-contract.yaml` (new)
- `Cargo.toml`, `Cargo.lock`: shared, merged serially
- `crates/connectors-build/src/gate.rs` (the drift step and the `--msrv` exclusion)
- `website/docs/reference/` (regenerated crate reference)

## Acceptance

Tests in `crates/connectors-tool-contracts/tests/publish.rs`:

- `tavily_contract_validates`: the Tavily descriptor and declarations publish a contract with three
  operations, `connector-tavily.search`, `.fetch`, `.crawl`, that `tool_contract::validate` accepts.
- `profile_is_the_declared_profile`: each operation's profile equals its declaration field by field.
- `operation_without_profile_is_refused`, `declaration_for_unknown_operation_is_refused`.
- `risk_under_the_floor_is_refused`: `search` declared `risk: low` with effect `secret` is refused
  under rule `risk`.
- `instance_name_outside_the_grammar_is_refused` (`Tavily.Prod`).
- `number_field_is_refused_with_its_path`: an input property `{"type":"number"}` is refused naming the
  operation and `/properties/<field>`; the same for `oneOf`.
- `omitted_field_leaves_the_input` and `omit_of_unknown_field_is_refused`.
- `contract_names_no_secret`: no `api_key`, `authorization`, `token` or configuration value in the
  document.
- `generated_contract_has_no_drift`: `generate --check` passes on the committed file and fails on a
  changed declaration.
