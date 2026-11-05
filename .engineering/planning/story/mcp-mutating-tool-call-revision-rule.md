---
format: aep.planning-md/3
id: story:mcp-mutating-tool-call-revision-rule
kind: story
status: draft
title: A mutating MCP tool call has one stated rule for an absent revision
tags:
- from-finding
relations:
- derived_from: review-result:connectors-wave-2026-11-04-u1-adversary
revision: 1
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
