---
format: aep.planning-md/3
id: story:invoke-definition-file
kind: story
status: implemented
title: operations invoke --definition runs an operation of a hand-written definition with no configured instance
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- serves: vision:independent-contract-adapters
- depends_on: story:connection-binding-excludes-operations
scope:
- confidence: cited
  path: adapters/catalog/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/Cargo.toml
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: cited
  path: apps/connectors/src/help.rs
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/operations.rs
- confidence: cited
  path: apps/connectors/src/local/select.rs
- confidence: cited
  path: apps/connectors/src/local/session.rs
- confidence: cited
  path: apps/connectors/tests/connector_instance_selectors.rs
- confidence: cited
  path: apps/connectors/tests/invoke_definition.rs
- confidence: cited
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: cited
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: inferred
  path: crates/connectors-host/src/local/definitions.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: inferred
  path: website/docs/guides
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/contracts/cli.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T08:55:23Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-18T08:55:23Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-18T11:25:38Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":3,"review_outcome":7}}}
---
## Problem

A hand-written `connectors-definition/1` file can be lowered (`connectors-build definition`, the
only caller of `crates/connectors-catalog/src/definition.rs` `lower()`), but using it needs a
release of this repository or a hand-made provider configuration with absolute bundle paths and a
host instance entry (`story:definition-lowerer-trial-defects`, trial of 2026-10-18). The owner
re-reads instances from the configuration file per request (`crates/connectors-host/src/local/owner.rs`
`selected`), and the catalog provider reads a bundle only from disk (`adapters/catalog/src/local.rs`
`Local::load`), so an instance held only in CLI memory never reaches the owner.

## Outcome

`architecture-decision-record:provider-definitions-and-runtimes` D12 point 3, for the case where
the definition's instance has no connection yet or already has one bound to its authority and
profile.

- Command: `connectors operations invoke --definition <file.yaml> --operation <key>` with the
  existing input flags (`--input-json`, `--input-file`, `--input-stdin`) and output envelope.
  `--definition` is a third selector, exclusive with `--adapter` and `--connector`/`--instance`
  (`invalid_input` otherwise). `--definitions-root <dir>` names the tree holding the vendored source
  and shared auth files; it defaults to the parent of the directory holding the file. The bare
  `connectors invoke` stays the legacy explicit command (`apps/connectors/src/compat.rs`).
- Lowering: `lower()` writes into an owner-only cache under the state directory keyed by the digest
  of the definition and its source; a second run with unchanged inputs reuses it. The host entry for
  the instance (connector id from the definition, instance id `definition`) lives in an owner-only
  configuration file beside the cache that the owner reads with the operator's configuration; the
  operator's configuration file is not modified. A collision with a configured instance of the same
  connector and instance id refuses by name.
- Authentication: with no connection for that instance, the command asks once whether to connect;
  on yes it runs the profile's existing acquisition (token prompt on the terminal, or the existing
  OAuth consent, whose client file is passed with `--credential-file`), then invokes. A
  non-interactive run (no terminal) refuses with `next_action` naming the connect command instead of
  asking. With a connection whose binding matches, it invokes with no question; a bundle- or
  selection-only change to the file follows `story:connection-binding-excludes-operations`. Until
  `story:invoke-definition-reuse-credential` lands, an authority or profile change to the file
  answers today's `lifecycle_conflict` with today's `next_action`, and nothing is asked.
- Lowering alone never connects: the cache and the owner-only entry grant no connection, admission
  or OAuth exchange (D12 point 4); only the "yes" answer to the question starts an acquisition.
  This story reads a file given on the command line; a definition taken from an installed index
  (D5) is not an input here.
- Everything after lowering is the configured-instance path: admission against the lowered
  selection, approval for writes, audit.

## Acceptance

Against the disposable HTTPS provider of `adapters/catalog/tests/local_runtime/`, a test writes a
token-profile definition by hand, runs `operations invoke --definition` with no configured
instance and no connection, answers "yes" to the one connect question and enters the token at the
token prompt that follows, and receives the operation's result; a second run with the same file and
a third after only an added operation return results with neither the question nor the token
prompt, and the operator's configuration file is byte-identical before and after.

## Scope

Cited:

- `apps/connectors/spec/cli.yaml` (the `operations invoke` binding near lines 562-586) and the
  generated `apps/connectors-cli-contract/` (`connectors-build cli`); `ess/domains/cli.yaml`
  (`OperationInvokeInput`); `crates/connectors-conformance/tests/cli_surface.rs`;
  `contracts/cli/v1alpha1/semantics.md` (selector rules near 372-421, a new subsection for
  `--definition`), `contracts/cli/v1alpha1/scenarios.md`, `contracts/cli/v1alpha1/fixtures/values.json`.
- `apps/connectors/src/local/operations.rs` (`dispatch`, `invoke`), `apps/connectors/src/local/select.rs`,
  `apps/connectors/src/local/session.rs` (`acquire`), `apps/connectors/src/local.rs`
  (`LocalHandler`), `apps/connectors/Cargo.toml` (path dependency on `connectors-catalog`).
- `crates/connectors-host/src/local/config.rs` (loading the owner-only definition entries beside
  the operator's configuration), `crates/connectors-host/src/local/owner.rs` (`selected`).
- `crates/connectors-catalog/src/definition.rs` (`Request`, `lower`), only if the cache needs a
  scratch-directory change.
- Tests: `apps/connectors/tests/` (new `invoke_definition.rs`, `connector_instance_selectors.rs`),
  `adapters/catalog/tests/local_runtime/cli_journey.rs` (journey with the disposable provider),
  `adapters/catalog/tests/definition_auth.rs` as the template for a lowered profile.
- Documentation: a guide section under `website/docs/guides/`, regenerated `website/docs/reference/cli.md`
  and `website/docs/reference/contracts/cli.md` (`connectors-docs generate`), `apps/connectors/src/help.rs`.

## Not in scope

Reusing a credential stored under another instance, and the question on an authority or profile
change (`story:invoke-definition-reuse-credential`); OAuth consent that reads a definition's
authorize and token parameters (D6, no story); installing definitions from a tag (D5).

## Why a separate story from the reuse prompt

Reuse copies a stored credential to another instance's binding, a new custody operation with its
own security review; this story adds a selector, a cache and the existing acquisition paths, and is
already one implementor's unit.

## Shared surfaces

| Story | Shared | Order and reason |
|---|---|---|
| `story:connection-binding-excludes-operations` | `contracts/cli/v1alpha1/semantics.md`, `crates/connectors-host/src/local/config.rs`, `owner.rs`, `apps/connectors/src/local.rs` | after it (`depends_on`): the third run of the acceptance needs its rule |
| `story:spec-models-operation-admission` (draft) | `apps/connectors-cli-contract` (generated) | either order; the generated directory is never merged by hand, the second to land regenerates it with `connectors-build cli` |
| `story:mcp-cli-journey-discovery-contract` (draft) | `contracts/cli/v1alpha1/semantics.md` (inferred there) | either order; that story appends an MCP section, this one adds the `--definition` selector subsection |
| `story:toolchain-pins-newest-release-20261001` (draft) | `apps/connectors-cli-contract`, `ess` (inferred there) | either order; the second to land regenerates `apps/connectors-cli-contract` and reruns `ess specify validate` |
| `story:kubernetes-spec-service` (draft) | `crates/connectors-conformance` (whole directory there) versus `tests/cli_surface.rs` here | either order; that story's body asks to be re-scoped before it is scheduled beside other source edits |
| `story:configuration-change-error` (draft) | `contracts/cli/v1alpha1/semantics.md`, `apps/connectors/src/local.rs` (inferred on that story) | after `story:connection-binding-excludes-operations` like this one; different sections (selectors here, connect refusals there) |

The cache and the owner-only definition entries are new code; the scope names their likely home, `crates/connectors-host/src/local/definitions.rs`, as inferred.

## Delivered

Implemented on branch unit/invoke-definition-file (de9406bb81, 98e0c677fa, f9e181f3fd, 9869ac07ab).
Decisions taken inside the story, recorded in `contracts/cli/v1alpha1/semantics.md` §3 "Invoking a
definition file":

- The generated contract keeps `--schema` and `--revision` required (ESS refuses optional dynamic
  selector fields), so the host prepares the command line before the generated runner reads it and
  supplies them from the description.
- The instance's alias and instance id are `<connector>.definition`; its entry pins the installed
  catalog provider as a derived instance's does. `instances list` and `setup check` stay the
  operator's configuration alone.
- The collision refusal is `invalid_configuration` with the new `configuration_rule`
  `definition_instance_configured`.
- No terminal, or any answer but yes, is `not_found` at `admission` with `create_connection`.
- An authority or profile change answers the read admission refusal of the recorded connection
  (`lifecycle_conflict`, `check_configuration` for an authority change), sending nothing.
- At most 64 connector ids are held; no command forgets one (revoke, then remove `definitions/`).
- A definition names no trust root: the instance trusts the platform's roots, and every base URL
  variable needs a default.

The lowerer's own `valid_id` still admits `..` as a `catalog`; this command refuses non-plain names
before lowering, and `connectors-build definition --out` remains the operator's own directory.
