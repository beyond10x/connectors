---
format: aep.planning-md/3
id: task:mcp-cli-selected-intent-contract
kind: task
status: implemented
title: Select MCP CLI intent and static discovery contract without claiming runtime support
relations:
- decomposes: story:mcp-cli-journey-discovery-contract
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-outbound-auth-lifecycle
- depends_on: story:mcp-inbound-mutation-replay
- depends_on: story:mcp-inbound-capability-projection
- depends_on: story:mcp-outbound-invocation-results
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T00:10:03Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-03T00:10:03Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-03T02:33:07Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Acceptance

Select one exact proposed MCP CLI syntax and document its authored discovery/error
contract, explicitly marked selected intent with runtime unavailable. Human prose
and a static JSON inventory agree on command paths, flags, safe protected-input
channels and transport/stdout ownership, each traced to its reviewed owning contract.
This static documentation inventory is not runtime discovery or a service payload.
The parent story stays open until the actual outbound/inbound CLI journey works.

## Scope

Cited writable: docs/local-mcp-cli.md and
adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md, named by the parent.
Inferred writable: adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json.
Read-only: compatibility.json (keep mcp=deferred), shared CLI semantics, ESS domains,
cli.yaml, generated parser/package/docs and production handlers. Confidence high
for document placement and absent runtime. Exact three document paths are the only
collision surfaces. No new runtime type/entity/command is introduced by metadata.

## Measured source boundary

Read-only scoper inspected apps/connectors/src/local.rs:25: production arguments
are delegated to the generated parser. apps/connectors/spec/cli.yaml:143 contains
no server command. Adding a declaration changes production parsing; it is outside
this task. No live server or MCP connection exists in the inspected CLI. The parent
simultaneously requested a working journey and excluded implementation; this child
closes only the selected-contract portion and cannot discharge the parent acceptance.

Derive separate CLI FailureCode, optional service_code and MCP protocol error
vocabularies from their owners; do not collapse them into shared ErrorCode. Refer
to actual docs/local-kubernetes-cli.md, local-catalog-provider.md and
local-gitlab-merge.md patterns; the parent's local-gitlab-cli.md reference is absent.
No example may imply connections connect --adapter mcp or server currently works.

Before parser/runtime stories, the selected launch inputs/results/errors need typed
native ESS ownership and pinned ESS0.45 CLI expressibility. The shared ESS CLI root
cannot import native types without a recorded composition design. Long-lived stdio
requires a binding reserving stdout for MCP frames; generated ProcessOutput alone
does not establish it. Record these as remaining obligations, not guessed models.

## Verification

Check static JSON parses, each document link/citation resolves, human syntax and
inventory match, and all unavailable steps are labelled. Reuse existing documentation
source checks. No new mirrored prose test or invented executable MCP fixture. No
runtime support, parser compatibility, wire behavior or conformance claim follows.

## Read-only preparation against final predecessor contracts — 2026-10-03

Proposed selected intent, runtime unavailable: `connectors --config CONFIG --state-dir STATE_DIR server --transport stdio`, with absolute local paths. The local MCP client owns the launched Connectors child. The proposed launch has no cloud/caller-assignment/outbound-child/credential flags and no generic `--output` surface; stdout exclusively carries MCP frames and stderr safe diagnostics. This task selects spelling; existing parser support is not claimed.

Outbound HTTP retains generic connections connect/status/revalidate and operations list/describe/invoke grammar from apps/connectors/spec/cli.yaml. Its MCP native profile, local operation identifiers, configuration and protected OAuth completion codec remain undeclared. Do not invent mcp.oauth or a local tools.call operation ID from remote tools/call. Existing static credential-prompt/file/stdin channels do not establish managed MCP OAuth. Protected continuations, state, codes, tokens and custody references never become ordinary outputs. Separate business input from credentials.

The static inventory distinguishes current-generic, selected-intent/runtime-unavailable and unresolved-binding. Keep parser errors, connectors.cli.FailureCode, optional shared service_code and open-ended peer JSON-RPC integer errors separate; the native three-code enum is not exhaustive. Native launch types, shared/native CLI composition, pinned ESS0.45 expressibility and actual stdout framing remain implementation prerequisites. Mutation advertisement stays withheld and the parent working CLI journey remains open.
