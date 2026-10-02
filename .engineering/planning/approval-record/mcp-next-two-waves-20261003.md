---
format: aep.planning-md/3
id: approval-record:mcp-next-two-waves-20261003
kind: approval-record
status: draft
title: Operator authorizes two MCP waves and managed cleanup
relations:
- decides: story:mcp-outbound-auth-lifecycle
- decides: story:mcp-inbound-mutation-replay
- decides: story:mcp-composition-provenance
- decides: story:mcp-cli-journey-discovery-contract
revision: 1
---
## Operator instruction

The operator explicitly requested on 2026-10-03: "dispatch now the next 2 waves
$aep:wave including mcp inbound+outbound use multiple agents for that, cleanup
$worktree:cleanup $worktree:managing-worktrees".

This authorizes selection and dispatch of two dependency-ordered waves, multiple
isolated agents, the unit/integration/closing commits and verified integration,
and cleanup of completed task-owned managed worktrees after evidence retention.
It removes the stage-1 approval stop for these two waves; it does not remove the
wave pages, source gates, independent review or a refreshed second-wave selection.
Existing approval-record:milestones-delivery-20261002 continues to authorize bot
publication and verified source release. This record adds no deployment, consumer
pin, caller/Connection assignment, stdio supervision or external execution decision.

Initial candidates: outbound authentication and inbound mutation replay, followed
by single-owner composition and the human CLI/machine-discovery contract. Read-only
scopers confirm exact scope first. Any remaining dependency or model gap is recorded
and resolved before that candidate's implementation; the instruction is not evidence
that blocked work is ready. Cloud multi-caller isolation remains outside these waves.

The current invocation/projection integration gate must finish before its dependent
next-wave implementation begins. Cleanup includes its completed worker trees;
active integration/model trees remain retained with their live ownership.
