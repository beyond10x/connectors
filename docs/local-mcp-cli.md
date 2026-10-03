# MCP through the local CLI: selected intent

**MCP runtime unavailable.** This guide selects a future local command spelling;
it is not an executable MCP walkthrough. The current CLI has no `server` command,
MCP adapter configuration, MCP connection profile or MCP operation binding.
[Compatibility metadata](../apps/connectors/spec/compatibility.json) remains
`mcp: deferred`. The [authored inventory](../adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json)
is static documentation, not CLI output or a runtime discovery endpoint.

For working provider journeys today, use the [catalog provider](local-catalog-provider.md),
[Kubernetes](local-kubernetes-cli.md) or [guarded GitLab merge](local-gitlab-merge.md)
guide. Those paths do not provide MCP support.

## Inbound stdio: selected intent, runtime unavailable

The exact intended launch is:

```text
connectors --config CONFIG --state-dir STATE_DIR server --transport stdio
```

`CONFIG` and `STATE_DIR` stand for explicit absolute local paths, not literal
arguments or credentials. `--transport` is required and this selected command
accepts only `stdio`. There is no selected default transport. This launch has no
`--output`, cloud, caller-assignment, outbound-child or credential flags. Do not
substitute the existing `serve` command: it starts the compatibility federation
service and is not an alias for an MCP server.

A local MCP client launches Connectors and owns that child. Connectors reads MCP
frames from stdin and reserves stdout exclusively for MCP frames; safe diagnostics
go to stderr. No JSON CLI success envelope, progress, prompt or startup banner
belongs on protocol stdout. No credential is acquired over this stdin/stdout
binding. The process admits the configured local owner before exposing any
capability. This is the [selected inbound binding](../adapters/mcp/contracts/server/v1alpha1/semantics.md),
not a decision about Connectors spawning an outbound MCP server.

The selected revisions are primary `2026-07-28` and interoperability `2025-11-25`.
Their per-request metadata versus initialization obligations remain those of the
[selection matrix](../adapters/mcp/contracts/protocol/v1alpha1/selection.md) and
local binding. Process startup alone is neither protocol initialization nor a
grant to execute an operation.

**Unavailable step:** configure the MCP projection and launch that command from
your client. The native configuration shape below is selected and validated by a
library and the protected owner's internal projection policy. The stdio launch
handler is not yet connected to that policy and a serving loop.
Do not paste an invented adapter stanza into a working installation.
Multi-caller/tenant-to-Connection assignment and cloud placement remain unresolved.
The local single-owner intent does not close either decision.

**Unavailable step:** discover and invoke through the running server. Once an
executable binding exists, [projection](../adapters/mcp/contracts/server/v1alpha1/projection.md)
requires implemented, fully bound, enabled, selected and metadata-admitted
operations. `tools/list`, `resources/list` and `prompts/list` expose only those
operations; empty eligibility gives an empty list, while unavailable admission
policy gives a refusal. Discovery neither probes credentials nor grants execution.
Resources and prompts remain read-only. Mutation advertisement remains withheld
until the complete [mutation/replay binding](../adapters/mcp/contracts/server/v1alpha1/mutations.md)
exists, including its protected approval source and explicit business key.

## Selected local projection configuration

The protected owner can validate this configuration for internal projection;
**the production MCP stdio entry remains unavailable**. The selected
location appends `.mcp-stdio.json` to `CONFIG`: `/private/config.toml` would use
`/private/config.toml.mcp-stdio.json`. The application must admit an owner-only
regular file without following symlinks, bounded to 1 MiB. Missing or invalid input
is a refusal, never an implicit empty catalog. Generic owner configuration stays
unchanged; there is no extra server flag or default exposure.

The [generated native types](../adapters/mcp/generated/configuration-types/types.rs)
define the closed JSON shape. `format` is `connectors-mcp-local/1`; `exposures` is
an explicit array, possibly empty. Every exposure requires `adapter_alias`,
`operation_ref`, `connection_ref`, a nonempty unique `families` array drawn from
`tools`, `resources`, `prompts`, and boolean `enabled`. At most 1,000 exposures are
accepted. An adapter/operation pair occurs only once, so two connection selections
cannot compete for one advertised identity. These selectors refer to existing
host configuration and metadata; parsing creates no connection or authority.

`limits` requires unsigned integer JSON tokens in these ranges:

| Member | Range |
|---|---|
| `frame_octets` | 256–1,048,576, including newline |
| `response_octets` | 1,024–33,554,432, including all representations and framing |
| `concurrent_requests` | 1–16 |
| `request_milliseconds` | 1–120,000 |

The native response ceiling allows space for an 8 MiB owner service envelope in
both structured content and escaped JSON text. Each operation must still fit its
complete selected binding before advertisement and dispatch. These limits cannot
extend the shared two-second lease or five-second teardown bounds.

An exposure may select `approval_file`, an absolute bounded private path. It is
never advertised or accepted from MCP input. Discovery does not open it or infer
that it contains valid proof. Only an admitted new mutation may read it through
the existing protected-source path; an admitted replay does not read or spend it.
Duplicate or unknown JSON members, explicit null instead of a path, invalid
selectors and conflicting families refuse configuration.

Configuration must be read anew for every admitted list page and invocation.
Selection, enablement or target changes invalidate affected projection revisions;
the serialized owner must repeat or fence selection at dispatch. A startup-only
snapshot does not satisfy this contract. The protected owner implements loading,
revision checks and audit admission; the serving loop and runtime families remain
unfinished.

The owner also reads `CONFIG.operation-curation.json`, the separate
[protected operation curation](../contracts/service/local-operation-curation.md).
Its complete metadata is pinned to the executable selection and the exact original
cached bootstrap. Curation changes invalidate the projection revision, and current
metadata permissions filter both operations and their metadata. Missing policy
refuses; missing operation curation supplies no defaults. This internal metadata
path does not establish native profile support, enforce every declared execution
limit or enable MCP serving.

The catalog executable now implements the explicit
[private bounded-read binding](../contracts/cli/v1alpha1/private-bounded-read.md).
It carries the original monotonic execution deadline and shares one provider
cutoff across OAuth exchange and business HTTP. Existing private selections do
not upgrade automatically. The owner still needs to select and enforce this
binding from admitted operation metadata, including complete public-envelope byte
limits, before these reads can be advertised through MCP.

## Outbound HTTP: current generic grammar, MCP binding unresolved

The grouped CLI grammar below already exists in the
[presentation source](../apps/connectors/spec/cli.yaml). All MCP uses of these
patterns are **unavailable**. `ADAPTER`, `PROFILE`, `CONNECTION`, `OPERATION`,
`SCHEMA` and `REVISION` must eventually come from an implemented, admitted binding;
this guide supplies none. An alias is configured selection, not an adapter name.
Do not assume `mcp`, `mcp.oauth` or a remote method such as `tools/call` is a local
adapter, profile or operation identifier.

Each pattern may use the existing generic globals `--config CONFIG`,
`--state-dir STATE_DIR` and `--output json` before its command path. JSON output
here belongs to ordinary grouped commands, never the selected stdio server.

| Current generic command path | Flags used by the intended MCP journey | MCP availability |
|---|---|---|
| `connections connect` | `--adapter ADAPTER --profile PROFILE`; exactly one existing static source: `--credential-prompt`, `--credential-file FILE` or `--credential-stdin` | unresolved-binding |
| `connections status` | `--adapter ADAPTER`; exactly one of `--connection CONNECTION` or `--acquisition ACQUISITION` | unresolved-binding |
| `connections revalidate` | `--adapter ADAPTER --connection CONNECTION --expected-revision REVISION` | unresolved-binding |
| `connections repair` | `--adapter ADAPTER --connection CONNECTION --expected-revision REVISION`; exactly one existing static source: `--credential-prompt`, `--credential-file FILE` or `--credential-stdin` | unresolved-binding |
| `connections revoke` | `--adapter ADAPTER --connection CONNECTION --expected-revision REVISION` | unresolved-binding |
| `operations list` | `--adapter ADAPTER`; optional `--limit LIMIT`, `--cursor CURSOR` | unresolved-binding |
| `operations describe` | `--adapter ADAPTER --operation OPERATION` | unresolved-binding |
| `operations invoke` | `--adapter ADAPTER --operation OPERATION --schema SCHEMA --revision REVISION`; `--connection CONNECTION` when required; exactly one of `--input-json JSON`, `--input-file FILE`, `--input-stdin`; `--approval-file FILE` and `--idempotency-key KEY` only under their selected mutation profile | unresolved-binding |

**Unavailable step: acquire and persist an MCP server credential.** The static
source flags above document existing generic grammar. They do not implement MCP
browser OAuth, refresh, protected browser continuation or completion ingress.
Those codecs, registration/profile identifiers, configuration and native
Connection/custody relations remain undeclared. Follow the
[outbound auth requirements](../adapters/mcp/contracts/client/v1alpha1/auth.md):
protected completion, exact authority/resource binding, definite custody and
publication acknowledgements, then safe connection observation. A CLI exit and
restart must eventually reuse that acknowledged association under current
admission; no such MCP restart evidence exists here.

**Unavailable step: discover, select and invoke an outbound operation.** Use exact
local operation/schema/revision selection once a binding publishes it. The
[outbound invocation contract](../adapters/mcp/contracts/client/v1alpha1/invocation.md)
covers remote `tools/call`, `resources/read` and `prompts/get`; those protocol names
are not a selected local CLI namespace. Preserve bounded content, partialness,
peer errors and uncertain effects. Neither discovery nor a repaired credential
authorizes an automatic business retry.

**Unavailable step: repair or revoke an MCP association.** Missing, expired,
revoked and custody-unavailable remain distinct auth observations. Repair must
preserve identity; local revocation needs definite durable cutoff and is separate
from any provider revocation. An uncertain rotating refresh cannot be exchanged
again; committed response recovery may only publish that candidate. These are
requirements, not claims that the generic commands currently implement MCP auth.

## Protected channels and errors

Existing static credential capture uses a hidden controlling terminal, admitted
owner-only regular file, or deliberately supplied protected stdin. The
[shared CLI contract](../contracts/cli/v1alpha1/semantics.md) owns descriptor,
permission, bounds and conflicting-source checks. Business JSON uses a different
channel; it never carries credentials or approval proof bytes. Credentials,
refresh tokens, codes, state, PKCE verifiers, actionable completion URLs and
custody locators must not enter argv, ordinary results, discovery, logs or audit.
An acquisition reference alone grants no completion authority. Inbound stdin is
reserved for MCP frames and cannot double as a credential input.

The [discovery/error contract](../adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md)
keeps four error layers separate: generated CLI presentation codes, typed CLI
`Failure.code`, optional shared `Failure.service_code`, and MCP JSON-RPC errors.
A tool's `isError` result is another result channel, not a process exit code.
An unknown integer peer error is not evidence of non-execution; timeout,
cancellation and lost replies grant no automatic retry or rollback claim.

## What must land before this becomes a working journey

Native launch input/result/error values need an ESS owner before a parser or
runtime story introduces them. Shared/native CLI composition needs an explicit
design: importing native values into the shared model is not implied. The pinned
[ESS toolchain](../crates/connectors-spec/toolchain.json), currently 0.45.0, must
prove the selected presentation is expressible. Generated `ProcessOutput` is a
finite command result and does not establish long-lived MCP stdout framing.
Actual configuration, protected OAuth binding, launch lifecycle, inbound/outbound
runtime and named conformance scenarios must then be implemented and verified.

Completing this selected-intent document leaves the working CLI journey open.
