---
format: aep.planning-md/1
id: story:mcp-cli-journey-discovery-contract
kind: story
status: draft
title: Write the human MCP CLI journey and the machine-readable discovery and error contract
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-outbound-connection-lifecycle
- depends_on: story:mcp-inbound-local-binding
- depends_on: story:mcp-profile-selection-matrix
- depends_on: story:mcp-inbound-capability-projection
- depends_on: story:mcp-outbound-invocation-results
- depends_on: story:mcp-inbound-mutation-replay
- depends_on: story:mcp-outbound-auth-lifecycle
scope:
- confidence: inferred
  path: adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md
- confidence: inferred
  path: apps/connectors/spec/compatibility.json
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: docs/local-mcp-cli.md
revision: 2
---
## Acceptance

`docs/local-mcp-cli.md` and
`adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md` together describe
the complete selected MCP surface twice — once as a journey a person can follow
from nothing to a working outbound connection and a running inbound binding, and
once as the machine-readable discovery and error contract an agent reads — with
the exact command paths, flags and transports named, and with every behaviour in
the human text resolving to a statement in a contract document rather than to a
plan.

## Scope

- `docs/local-mcp-cli.md` — new; the human journey, in the shape of the existing
  `docs/local-kubernetes-cli.md` and `docs/local-gitlab-cli.md`
- `adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md` — new; the
  machine-readable discovery and error contract
- `apps/connectors/spec/compatibility.json` — the `"mcp": "deferred"` value at
  line 12 becomes the selected disposition. Shared file: it is also the CLI
  compatibility record maintained under the local-CLI stories, and nothing else in
  this decomposition touches it.
- `contracts/cli/v1alpha1/semantics.md` — the command inventory gains the selected
  entry point. Shared file, owned by `story:local-cli-binding-semantics`
  (implemented); this story appends a section rather than editing existing ones,
  and no other story in this decomposition touches it.

## Domain relations

None of its own. Every behaviour this document describes is specified elsewhere
and cited; where a behaviour rests on an `UNMAPPED:` marker or a blocker, the
human text says the behaviour is not available rather than describing it in the
future tense.

## What this story must establish

The epic's last deliverable: "Provide a human CLI journey and machine-readable
discovery/error contract. `$BIN server` is the requested entry-point intent;
finalize exact flags and transports during contract authoring rather than treating
this tracking record as a implemented CLI promise." Acceptance criterion 6 closes
with the same requirement: "CLI/docs describe the final selected behavior for
humans and agents."

**What exists today, so the gap is stated rather than assumed.** The binary is
`connectors` (`apps/connectors/Cargo.toml:2`). `apps/connectors/src/main.rs:19-23`
routes `describe`, `invoke` and `serve` into the legacy compatibility path, and
`apps/connectors/spec/compatibility.json:7-9` records those three legacy routes.
No `server` path exists in the generated binding: `apps/connectors/spec/cli.yaml`
opens its `commands:` list at line 143 and contains none.
`apps/connectors/spec/compatibility.json:12` currently reads `"mcp": "deferred"`.
`docs/cli-migration-v1-to-v2.md:104` records the same state for the v1 `serve mcp`
verb: "not available | inbound `/mcp` as a host transport binding onto governed
operations (ADR 0028); how it is started is undecided". Naming the entry point is
therefore a decision this story makes and records, and the epic authorizes exactly
that.

**A presentation surface here has a generated owner.**
`contracts/cli/v1alpha1/semantics.md` section 2 states "The presentation binding
is authored separately as `ess-cli/1`", and `docs/development.md:52-56` gives the
generation and `--check` path. Any command this story names must be expressible in
that binding or the story says why it is not; a documented flag with no generated
counterpart is the failure this constraint exists to catch.

**The error contract is the agent-facing half.** The discovery contract states,
per selected capability, the identifier an agent reads and the exhaustive error
set it may receive, mapped onto `connectors.service_wire.ErrorCode`
(`ess/domains/service_wire.yaml:14-16`). An error an agent can receive and cannot
find in that document is a defect in the document.

## What it does not cover

Any behaviour behind `decision-blocker:mcp-outbound-stdio-process-ownership` or
`decision-blocker:mcp-caller-connection-assignment`: the journey stops where those
stop, and says which question it stopped at. No runtime, parser or handler is
implemented, and no command is made to work; this story writes the documents that
say what the commands are.
