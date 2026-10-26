# Tool contracts for admitted operations, `tools/v1alpha1`

**Status:** proposed; publication and conformance implemented for the Tavily adapter
(`crates/connectors-tool-contracts`). The model is
[`connectors.tool_contracts`](../../../ess/domains/tool_contracts.yaml).

## What is published

For an instance's descriptor revision Connectors publishes one `tool-contract/1` document: toolset
and family `connector-<instance>`, version `v1alpha1`, one operation `connector-<instance>.<name>`
per admitted operation. `tool-contract/1` and `tool-conformance/1` are owned by the tools repository
and implemented by its crate `b10x-tool-contract`; Connectors pins a tools release of that crate and
reads its profile type, validator and conformance runner from it.

Each operation's profile (effects, consequences, risk, idempotency, determinism, bounds, subjects,
placement, capability, output) is authored beside the adapter that implements the operation, in
`adapters/<adapter>/spec/tool-profiles.json`. A descriptor's provider profile name (such as
`tavily/2026-10`) is not a profile: an admitted operation without an authored profile is refused,
never defaulted. Input fields the connection fills are listed under the declaration's `omit`; they
leave the published input and a call that carries one is refused.

## What a published contract never carries

A credential, a connection reference, a grant, an approval, or a value of the instance's
configuration. Which connection a call runs under, its grant check and its approval evidence stay in
the governed invocation that answers the call.

## Refusals

The publisher refuses the whole document, naming the operation, when:

| refusal | when |
|---|---|
| `profile-missing` | an admitted operation has no declaration with a profile, or a declaration names an operation the descriptor does not admit |
| `schema-unrepresentable` | an input or output schema holds a shape the `tool-contract/1` type grammar cannot type (a `number`, a union, an untyped node), named by its schema path; or an `omit` names a field the input lacks |
| `risk-below-floor` | the declared risk is under the floor its effects and consequences set (`tool-contract/1` rule `risk`) |
| `profile-invalid` | the validator refuses the document under another rule (bounds, capability, subjects, preconditions, paging), or the profile names a secret under `requires` |
| `name-invalid` | the instance name is outside `[a-z][a-z0-9-]*`, or a declared `<name>` is outside `[a-z][a-z0-9_-]*` |

## Errors a published operation answers

`connector.Failed` with field `code`, one of the service error codes (`invalid_input`,
`unauthorized`, `rate_limited`, `unavailable`, …), for every refusal or failure of the governed
invocation; and, on an operation with `omit`, `connector.FieldNotAllowed` with field `field`.

## Conformance

The authored scenarios under [`scenarios/`](scenarios/) hold the model's publication outcomes and
run in the repository gate. The implementation is held to `tool-conformance/1` by
`crates/connectors-tool-contracts/tests/tavily_conformance.rs`, which runs the suite under
`adapters/tavily/spec/tool-conformance/` against the adapter's own `invoke` over a local fake of the
provider's API.
