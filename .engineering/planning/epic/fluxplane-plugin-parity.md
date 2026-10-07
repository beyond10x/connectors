---
format: aep.planning-md/3
id: epic:fluxplane-plugin-parity
kind: epic
status: draft
title: Connectors replaces fluxplane-plugin in agent sessions
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:recent-agent-adapter-usage-20260909
revision: 1
---
## Outcome

The Connectors local CLI replaces `fluxplane-plugin` in the operator's Claude Code and Codex
sessions: every fluxplane operation those sessions use has a Connectors operation with the same
capability against the same provider.

## Evidence

`docs/fluxplane-plugin-parity.md` (2026-10-07): 301 declared fluxplane operations, 109 used,
6,356 calls since 2026-09-09. Covered 29 operations (2,659 calls), partial 23 (1,273), missing
249 (2,424). The used partial and missing operations form 25 units (3,788 calls with the mapped
undeclared names), planned as 7 waves after the feed-binding wave.

## Scope

One story per unit; a story's operations, calls, surface and planned wave are in its body and
in the parity page. Operations with 0 calls are not planned until a session uses them.

## Spec first

Each story models what it adds in this repository's ESS specification (the adapter's native
model, or the catalog provider's pinned OpenAPI source and selection), validates it with the
newest `ess`, regenerates, then implements. A provider with no model yet (Slack, Homer,
Alertmanager) gets its domain drafted and validated before the story is scheduled.
