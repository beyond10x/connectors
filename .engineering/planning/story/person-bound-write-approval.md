---
format: aep.planning-md/3
id: story:person-bound-write-approval
kind: story
status: draft
title: A write approval bound to a person, not to the OS user
relations:
- informed_by: story:standing-write-approval-for-unattended-agents
- informed_by: story:mcp-cloud-caller-isolation
revision: 1
---
## Problem

A write approval proof is issued and signed by whoever runs as the owner's OS user. The Secret
Service binding admits a peer by UID alone (`crates/connectors-custody/src/secret_service/mod.rs`,
the `SO_PEERCRED` check that compares `peer.uid` with the owner's UID). `docs/local-approvals.md`
says the issuing command "makes no claim about an independent human review". So an agent running as
the owner's OS user can prepare, issue and sign its own approval proof, and then run the write with
it.

## Request

A consumer requirement relayed by the orchestrator on 2026-10-29: reads stay open, and every write is
approved by a human the agent cannot impersonate.

## Outcome

A write approval is bound to a person, not to the OS user. Issuing or signing a proof needs a
presence the agent cannot produce: a step outside the agent's process tree and its terminal, such
as a confirmation on another device, a hardware key touch or a separate session. An agent running as
the owner's OS user cannot issue or sign a proof that a write admits.

The confined-agent case is in scope. The configuration file, the state directory and the keyring
sit outside the agent's confinement. The agent can ask for an invocation, but it cannot read or
write the files and secrets that issuance depends on.

## Open questions

- Which person-presence mechanisms each platform offers, and which one the default is.
- How this composes with story:standing-write-approval-for-unattended-agents, where a person
  approves once for a scoped, expiring series of writes, and with story:mcp-cloud-caller-isolation
  for callers that are not local processes.

## Acceptance (draft)

- An agent process running as the owner's OS user, given the CLI, cannot issue a proof that a write
  admits. Each attempt is refused by a named code and makes no provider call.
- A confined agent cannot read the configuration, the state directory or the keyring, and a write it
  requests runs only with a proof the person issued outside the confinement.
