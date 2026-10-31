---
format: aep.planning-md/3
id: story:per-caller-operation-allow-list
kind: story
status: draft
title: An instance admits different operations to different callers
relations:
- informed_by: story:mcp-cloud-caller-isolation
- informed_by: story:standing-write-approval-for-unattended-agents
revision: 1
---
## Problem

Admission is per instance. An instance's `permissions.operations` (`Permissions` in
`crates/connectors-host/src/local/config.rs`, held by each entry's `permissions` table) names the
operations every caller of that instance may run, and `README.md` § Configure instances says so:
"Every operation of an enabled connector is available on its instances. Configuration does not
group operations or hide a subset". Two callers that share an instance, such as an operator and an
agent, or two agents, therefore get the same operations. The only way to give one of them less is a
second instance with the same credential.

## Request

A consumer requirement relayed by the orchestrator on 2026-10-29: reads stay open, and every write is
approved by a human the agent cannot impersonate. To hold that, an agent must be able to read where
a person can also write, on one instance.

## Outcome

An instance admits different operations to different callers. Each caller is named and admitted
separately. Its allow-list is checked at admission, before any provider call, and a refusal names
the caller and the operation. A caller not listed gets nothing, and the instance's own
`permissions.operations` stays the upper bound for every caller.

## Open questions

- What identifies a caller locally (the OS user, a confined process, an MCP session or a token), and
  which of these an agent cannot forge. This overlaps story:mcp-cloud-caller-isolation, which
  assigns connections to callers in a multi-caller cloud MCP placement.
- Whether a caller's write list may only narrow what story:standing-write-approval-for-unattended-agents
  grants.

## Acceptance (draft)

- Two callers of one instance: one is admitted reads only, the other reads and writes. Each is
  admitted exactly its own list, and the refused write makes no provider call.
- A caller with no entry is refused by a named code that names the caller.
