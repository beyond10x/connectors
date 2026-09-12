---
format: aep.planning-md/1
id: story:mcp-inbound-local-binding
kind: story
status: active
title: Specify the local-client inbound MCP binding exposed by the CLI
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-profile-selection-matrix
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/scenarios
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/semantics.md
revision: 6
---
## Acceptance

`adapters/mcp/contracts/server/v1alpha1/semantics.md` and its scenarios specify the
complete local-client inbound binding exposed by the CLI — which transport a local
MCP client uses to reach it, which principal it admits before any capability is
named, and how message framing, streaming, progress, cancellation, connection and
session loss, and version and capability mismatch behave on that binding — such
that **every one of those six behaviours has a named observable outcome, the state
transition it turns on in the `connectors.sessions.Session` vocabulary, and a
scenario file under `adapters/mcp/contracts/server/v1alpha1/scenarios/` named after
it**; such that the whole binding is described without reference to any cloud
control plane, identity service or network service this repository does not already
have; and such that a behaviour the selected transport does not offer appears in
that same enumeration as refused with its reason rather than absent.

Framing covers a malformed or truncated inbound message; streaming covers a
partially delivered outbound result. Both are named that way in the document so the
six behaviours and their six scenario files correspond one to one.

Lost replies and mutation uncertainty are `story:mcp-inbound-mutation-replay`.
Error mapping onto `connectors.service_wire.ErrorCode` is
`story:mcp-inbound-capability-projection`. Those two plus the six scenario files
above are the inbound half of the epic's acceptance criterion 5.

## Scope

- `adapters/mcp/contracts/server/v1alpha1/semantics.md` — new; the inbound base
  document. No other story writes this file.
- `adapters/mcp/contracts/server/v1alpha1/scenarios/` — new; session lifecycle
  scenarios. `story:mcp-inbound-capability-projection` and
  `story:mcp-inbound-mutation-replay` also add files to this directory under their
  own names.

## Domain relations

**`McpInboundSession → McpCaller` is `UNMAPPED:`** in `story:mcp-domain-model`.
In the local placement this document can specify the binding without resolving it,
because the local placement admits exactly one principal and there is therefore
nothing to assign. That single-principal fact is cited, not assumed:
`contracts/cli/v1alpha1/semantics.md` section 1 states "The first local binding
admits the current effective Linux UID as the configured owner, verified again at
a protected Unix-domain socket using kernel peer credentials. Missing/mismatched
owner policy refuses before protected entry or provider work." The enforcement is
in the running code —
`crates/connectors-host/src/local/owner/transport.rs:413` refuses a socket path
that is not a socket, whose `uid()` is not the process's own, or whose mode has
any group or other bits set (inferred from that line; no `ess/1` document declares
this as a relation).

The moment there is more than one principal, the assignment becomes
`decision-blocker:mcp-caller-connection-assignment` and this document stops. That
boundary is stated in its status line rather than papered over, and the cloud
placement is `story:mcp-inbound-cloud-profile`.

## What this story must establish

`epic:mcp-contracts`'s inbound deliverable begins: "an explicitly selected
local-client binding". Acceptance criterion 2 is the observable test: "A local MCP
client connects to the selected `$BIN server` local binding, discovers only
supported/admitted capabilities and receives correctly mapped results and errors.
No cloud control plane is necessary."

**Inbound stdio is not behind the process-execution question.** In this direction
the MCP client spawns the CLI and Connectors is the child, so no provider binary is
executed by this repository. `decision-blocker:mcp-outbound-stdio-process-ownership`
holds only the reverse case, and says so.

**The entry point is an intent, not a promise.** The epic is explicit: "`$BIN
server` is the requested entry-point intent; finalize exact flags and transports
during contract authoring rather than treating this tracking record as a
implemented CLI promise." Today the binary is `connectors`
(`apps/connectors/Cargo.toml:2`), `apps/connectors/src/main.rs:21` routes
`describe`, `invoke` and `serve` into the legacy compatibility path, and no
`server` command exists in the generated binding
(`apps/connectors/spec/cli.yaml`, whose `commands:` list at line 143 onward
contains no such path). `apps/connectors/spec/compatibility.json:12` records
`"mcp": "deferred"`. This document states the binding's semantics; the command
surface is `story:mcp-cli-journey-discovery-contract`.

**Session loss has an owner already.** `ess/domains/sessions.yaml:95-164` declares
`connectors.sessions.Session` with `Offered`, `Establishing`, `Ready`, `Closing`,
`Closed` and `Lost` and a `continuity_lost` transition into `Lost` from every
non-terminal state. The same file at lines 89-91 records why no edge is drawn to a
connection: "a future selected session binding must join its one Connection and
establishment authority, whose persistence model remains unselected ... no stub
edge is implied." This document reuses that vocabulary and repeats that refusal.

## What it does not cover

Which Connectors capabilities are advertised and how results map —
`story:mcp-inbound-capability-projection`. Mutating invocations —
`story:mcp-inbound-mutation-replay`. The cloud placement —
`story:mcp-inbound-cloud-profile`. Anything outbound.
