---
format: aep.planning-md/3
id: story:owner-bounded-read-rechecks-semantic-effects
kind: story
status: active
title: The owner's bounded read refuses an operation with semantic effects
tags:
- from-finding
relations:
- derived_from: review-result:wave-20261105-u2-adversary-findings
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/owner/governed/bounded.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/governed/tests/bounded_reads.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T17:29:38Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-05T17:29:38Z", actor: "human:timo", revision: 6}
---
<!-- Starting point for a `story` artifact, seeded by `aep artifact new story <name>`.
     No frontmatter here on purpose: the `---` block is written by the CLI from the id, kind, status
     and relations you gave it, and a second copy in this file would be the one that went stale.
     Delete the italic guidance as you fill each section. -->

# Story: <name>

## Outcome

*What is true for whom once this has shipped, in one sentence. If it names a component rather than a
person, it is a task — say what changes for someone.*

## Context

*Why this is worth doing now, and what it depends on. Link the epic or specification it comes from
rather than restating it; the `derived_from` relation already carries the edge.*

## Acceptance

*The conditions under which this is done, each one something a person or a check can observe. "Works
correctly" is not one of them.*

## Out of Scope

*What a reasonable reader would expect to be included and is not — the boundary that stops this
story quietly becoming an epic.*

## Ambiguities

*Each gap this story found and did not close, classified. `inferable` — the answer is already
written down, so give the `path:line` or the artifact id that settles it.
`requires-stakeholder-input` — nobody here can decide it, so name who does, and raise that entry as
a `decision-blocker` with a `blocks` edge to this story, or it is a sentence somebody improvises
later.*

## Open Questions

*Anything still undecided belongs in `## Ambiguities` above, classified and with its citation or its
decider. Keep this section for a question that is neither — an unowned question is a story that
stalls without anybody noticing.*

## Why

The local MCP server refuses, by name, an operation that has semantic effects or effects other than `network` when its approval is not required (`apps/connectors/src/mcp/projection.rs:214-221`). That check is the only guard for the case. The owner's bounded read checks approval and idempotency only (`crates/connectors-host/src/local/owner/governed/bounded.rs:28-36`). It does not recheck `semantic_effects` or effects other than `network`. A caller that reaches the owner socket directly, as the same uid, without the MCP server, would get such an operation run as a bounded read.

The wave 2026-11-05 adversary on story:mcp-write-operation-call-by-name-is-refused found this by reading the code (finding F4, pre-existing). It was not run, and the adversary found no caller other than the MCP server.

## Outcome

The owner refuses a bounded read whose operation carries semantic effects, or effects other than `network`. It refuses with the same `not_found`-class answer the projection gives, before any provider request. The MCP projection and the owner then hold the rule independently.

## Acceptance

- A runtime case drives the owner's bounded read directly with an operation that has non-empty `semantic_effects` and approval not required. The case is refused and records 0 provider requests. The same case on an admitted read makes 1 request.
- The case is shown red with the owner recheck disabled.
