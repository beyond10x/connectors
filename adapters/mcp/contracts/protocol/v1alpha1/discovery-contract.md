# MCP CLI discovery and errors: authored selected intent

**Static documentation; MCP runtime unavailable.**
[discovery-contract.json](discovery-contract.json) is the authored machine-readable
inventory `mcp-cli-selected-intent/1`. It is not a CLI response, MCP discovery
result, service payload, generated parser contract or new runtime schema.
The [human guide](../../../../../docs/local-mcp-cli.md) uses the same paths and
flags. This child contract does not complete the working CLI journey.

## Dispositions and ownership

`current-generic` means syntax present in
[cli.yaml](../../../../../apps/connectors/spec/cli.yaml), whose generated parser is
called by [local.rs](../../../../../apps/connectors/src/local.rs).
`selected-intent` means only the newly selected spelling below.
`runtime-unavailable` means it must not be advertised as executable.
`unresolved-binding` means current generic syntax lacks MCP-specific configuration,
profile, identifiers and executable binding. These are documentation dispositions,
not product lifecycle states. [Compatibility](../../../../../apps/connectors/spec/compatibility.json)
continues to say `mcp: deferred`.

All JSON `sources` are repository-relative file paths. `commands[].id` is a
static documentation key, not an operation identifier. `commands[].path` is a CLI
path; protocol method names live separately under `protocol_discovery`.
`flags` names the complete command-specific flag inventory for each listed path;
`global_flags` names the applicable process globals. `constraints` describe source
selection without pretending every listed flag is required. Omitted CLI commands
remain outside this selected journey, not removed from the existing binary.

## Selected inbound launch

```text
connectors --config CONFIG --state-dir STATE_DIR server --transport stdio
```

The required explicit `--config` and `--state-dir` values are absolute local paths;
`--transport` is required, accepts `stdio`, and has no selected default. No
`--output` applies to this selected server command. Runtime is unavailable.
[Local binding](../../server/v1alpha1/semantics.md) owns single-owner admission,
framing, cancellation and supervision: the local MCP client launches Connectors
as its child, stdin receives frames, stdout sends only frames, and stderr carries
safe diagnostics. No credential source shares protocol stdin. This selects no
outbound child owner or multi-caller Connection assignment.

The existing compatibility `serve` path is not this command and is not an alias.
No parser, generated source, `ess-cli/1` declaration or compatibility flag is changed
by this inventory. The launch's typed native ESS values, shared/native composition,
pinned ESS 0.45.0 expressibility and streaming process binding remain prerequisites.
There is no selected server startup success/error payload or exit-code mapping;
it must be specified with that binding, not copied from generic `ProcessOutput`.

## Current generic paths, unresolved MCP binding

These are the same complete command-specific flag inventories in the JSON and
[human guide](../../../../../docs/local-mcp-cli.md). Each has generic globals
`config`, `state-dir`, `output`, grammar `current-generic`, and MCP
`unresolved-binding`. The listed source alternatives belong to the existing
static-entry binding; they do not establish a managed MCP OAuth codec.

| Command | Flags |
|---|---|
| `connections connect` | `adapter`, `profile`, `credential-file`, `credential-stdin`, `credential-prompt` |
| `connections status` | `adapter`, `connection`, `acquisition` |
| `connections revalidate` | `adapter`, `connection`, `expected-revision` |
| `connections repair` | `adapter`, `connection`, `expected-revision`, `credential-file`, `credential-stdin`, `credential-prompt` |
| `connections revoke` | `adapter`, `connection`, `expected-revision` |
| `operations list` | `adapter`, `limit`, `cursor` |
| `operations describe` | `adapter`, `operation` |
| `operations invoke` | `adapter`, `connection`, `operation`, `schema`, `revision`, `input-json`, `input-file`, `input-stdin`, `approval-file`, `idempotency-key` |

Connect/repair select exactly one protected static source. Status selects exactly
one connection or acquisition. Invoke selects one business input source, an exact
operation/schema/revision and an explicit connection where required. The shared
mutation profile governs approval-file and idempotency-key applicability; this
inventory does not make MCP writes callable. Outbound local operation/profile
identifiers remain null in the JSON, deliberately distinct from remote
`tools/call`, `resources/read`, `prompts/get`.

[Auth](../../client/v1alpha1/auth.md) owns protected acquisition, persistent
association, missing/expired/revoked/custody-unavailable outcomes, identity-preserving
repair, publication-only refresh recovery and local/provider revocation separation.
Its unresolved native binding relations stay open. Tokens, authorization codes,
state, PKCE verifiers, completion URLs, custody references and approval proofs
never become ordinary discovery/result fields or business input. Safe references
are correlation, not authority. Actual protected OAuth continuation/completion
channels remain unresolved; static file/prompt/stdin grammar cannot substitute.

## Protocol discovery and invocation: runtime unavailable

[Selection](selection.md) retains primary `2026-07-28` and interoperability
`2025-11-25`. Primary protocol discovery is `server/discover`; legacy initialization
is `initialize`. Both use `tools/list`, `resources/list`, `prompts/list` for
operation discovery. Invocation methods are `tools/call`, `resources/read` and
`prompts/get`. These are MCP methods, not new CLI subcommands or local operation IDs.

[Projection](../../server/v1alpha1/projection.md) owns qualified names, inverse
selection, complete family codecs and admission precedence. Eligibility requires
implemented, bound, enabled, selected and metadata-admitted operations. Metadata
and execution admission are separate: listing grants no dispatch, does not probe
credentials and cannot spend approval. Every page rechecks admission/revision;
unknown names grant no alternate Connection. Resources/prompts are read-only.
[Mutation replay](../../server/v1alpha1/mutations.md) keeps mutation advertisement
withheld until its complete executable binding exists. MCP IDs never become
implicit business keys; retained-result disclosure requires current admission.

## Error layers

The JSON records complete current CLI and shared-service vocabularies from their
owning declarations. They are vocabularies, not a claim that every command can emit
every code. No exhaustive runnable MCP command/error set exists yet.

| Layer | Source and carrier | Boundary |
|---|---|---|
| CLI presentation | [generated runtime](../../../../../apps/connectors-cli-contract/src/runtime.rs), outer `error.code` with empty data | Parser/source/internal/dynamic-validation failures are not application `Failure.code`. |
| CLI application | [connectors.cli.Failure](../../../../../ess/domains/cli.yaml), outer `error.code: failure`, typed `error.data` | `data.code`, `kind`, `stage`, `next_action` and permitted safe correlations retain the shared CLI meanings. |
| Shared service | [connectors.service_wire.ErrorCode](../../../../../ess/domains/service_wire.yaml), optional `Failure.service_code` | Preserve the service code separately; `service_failure` names its CLI route. It is not a new CLI enum variant for every provider error. |
| MCP protocol | [native protocol source](../../../spec/ess/domains/protocol.yaml) and [invocation](../../client/v1alpha1/invocation.md) | Peer JSON-RPC codes are integers in an open set; the three native named protocol codes are not exhaustive. Do not coerce them into either closed CLI or service enum. |

For existing generic commands in `--output json`, success is one newline-terminated
`{"ok":true,"result":...}` on stdout; failure leaves stdout empty and sends
`{"ok":false,"error":{"code":...,"data":...}}` on stderr. Exit 0 means successful
command observation, 2 usage/input refusal, 1 operational refusal/failure and 130
interruption. The [shared CLI contract](../../../../../contracts/cli/v1alpha1/semantics.md)
owns exceptions such as help and completions. This finite envelope/exit contract
does not apply to protocol frames on the proposed stdio server.

Inbound mapping follows the complete [projection error table](../../server/v1alpha1/projection-cases.json).
A resolved tool's application/execution error uses `isError:true` with the complete
safe service Response in structured content and matching JSON text. Pre-resolution
admission/lookup/revision/framing errors use the applicable RPC path; resources and
prompts carry service errors as JSON-RPC `-32000` with the safe service Response in
`data`. Protocol decoding errors retain their protocol meaning. The binding-local
`-32000` carrier is not the universe of possible peer errors.

Outbound [result handling](../../client/v1alpha1/invocation.md) preserves bounded
peer error data and distinguishes protocol errors, tool business errors, malformed
answers and transport uncertainty. None proves non-execution after possible dispatch.
No automatic business retry, credential forwarding or rollback follows from any
error, cancellation, timeout, partial reply or successful local repair.

## Verification boundary

This inventory can be parsed and compared with its sources, links and human guide.
It introduces no typed runtime entity, validates no launch or wire behavior and
executes no conformance scenario. The working journey remains open until native
ESS ownership, CLI composition and expressibility, exact configuration and
protected ingress, implementation and real runtime acceptance are all established.
