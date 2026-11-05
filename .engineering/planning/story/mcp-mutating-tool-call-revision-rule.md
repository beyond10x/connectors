---
format: aep.planning-md/3
id: story:mcp-mutating-tool-call-revision-rule
kind: story
status: active
title: A mutating MCP tool call has one stated rule for an absent revision
tags:
- from-finding
relations:
- derived_from: review-result:connectors-wave-2026-11-04-u1-adversary
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/mutation-cases.json
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/mutations.md
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/projection.md
- confidence: inferred
  path: crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T09:37:30Z", actor: "human:operator", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-05T17:29:39Z", actor: "human:operator", revision: 8}
---
## Why

The adversary pass on the MCP revision rule (review-result:connectors-wave-2026-11-04-u1-adversary,
finding 1) found two contracts that disagree. `adapters/mcp/contracts/server/v1alpha1/projection.md`
now binds a `tools/call` without a revision to the advertised revision. 
`adapters/mcp/contracts/server/v1alpha1/mutations.md:23-25` still says a mutating tool request carries
its exact current descriptor revision. Mutating tools are not advertised today, so no call reaches
either rule, but the first mutating tool will need one.

## Acceptance

- One contract text states which rule a mutating `tools/call` follows when it carries no revision, and
  the other refers to it.
- A case in `adapters/mcp/contracts/server/v1alpha1/mutation-cases.json` (or the projection case file)
  holds the chosen outcome for a mutating tool call without a revision, and the checker enforces it.

## Open question

Which rule: mutating tools require the revision (a client must have seen the description it acts on),
or they bind like reads. This is an operator decision.

## Decision (2026-11-05): option A

Decided by the directing session `b10x-chef-orchestrator` under its grant:

- A mutating `tools/call` requires its descriptor revision; `adapters/mcp/contracts/server/v1alpha1/mutations.md:23-25`
  stands.
- `adapters/mcp/contracts/server/v1alpha1/projection.md` limits the absent-revision binding to read tools
  and refers to `mutations.md`.
- A case row for an absent revision on a mutating call: `-32602`, no provider request; its checker holds it.
- Reason: it keeps the documented write contract and narrows a rule that no write reaches today, since no
  mutating tool is projected.
- Not taken: option B (mutating calls bind like reads) breaks a documented contract, which is the
  operator's alone; option C (leave both texts) leaves them disagreeing.

Accepted by the directing session for the next connectors wave (its acceptance); it does not join the
2026-11-05 wave.
