---
format: aep.planning-md/3
id: decision-blocker:standing-write-approval-mode
kind: decision-blocker
status: cleared
title: How is a write admitted for an agent running unattended?
summary: Approvals bind one exact input, live at most u16 seconds and are spent once; A standing approval, B auto-issuer, C per-send human
relations:
- blocks: story:standing-write-approval-for-unattended-agents
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-22T09:22:46Z", actor: "human:timo", revision: 3, executor: "agent:connectors-session", correlation: "relayed-by-b10x-chef-orchestrator-20261022"}
---
## Question

An agent that runs unattended (a Kubernetes pod with no operator at a terminal) needs to send a
Slack message through Connectors. Every Slack write is a required-approval mutation
(`README.md:295`). How is such a write admitted?

## What Connectors does today (read 2026-10-22)

- An approval binds one exact call: the target is
  `{"connection","operation","revision","input"}`
  (`crates/connectors-host/src/local/owner/approval_issuance.rs:226`).
- Its lifetime is a `u16` count of seconds (`approval_issuance.rs:189`); the shipped policy sets 300
  (`:323`).
- The host verifies it against the complete canonical subject and spends it exactly once
  (`contracts/operations/v1alpha1/semantics.md` steps 5 and 8).
- The only approval mode is `"required"` (`approval_issuance.rs:303,320`).
- Under the container custody `mounted_secret`, signing an approval is refused as
  `custody_read_only` (`docs/local-mounted-secret-custody.md:126`).

So no standing or scoped approval exists, and a container cannot issue one for itself.

## Options

| | What it does | Cost |
|---|---|---|
| **A (recommended)** | A new standing approval: issued once on the operator's host, scoped to one connection and one operation, with input constraints (for example a channel allow-list), a rate cap (`max_per_hour`) and `not_after`. Delivered to the container as a mounted file; the host verifies the constraints and counts each call. | Changes the approval contract: steps 5 and 8 no longer hold for this mode. Needs an ESS model of the standing approval, a per-call counter in the state directory, and a proof that outlives `u16` seconds. ADR-level. |
| B | A host-side auto-issuer: the container's host signs a per-call approval under a local policy file. | Needs writable custody for the signing seed, which `mounted_secret` refuses. The approver is the agent's own host, so an approval no longer names a person. |
| C | No change: a person issues each send's approval (the agent's `approval` mode). | The agent is not autonomous. How a per-call proof reaches the container: I don't know. |

Recommendation A: it keeps a person as the issuer and bounds what the agent can write.

## Clears when

Timo picks an option. A needs an ADR before story:standing-write-approval-for-unattended-agents
moves past draft.

## Decided (2026-10-22)

Option A. Timo's answer "a", relayed by the orchestrator session `b10x-chef-orchestrator` on
2026-10-22, with these bounds:
- The operator grants a standing approval explicitly; nothing grants one by default.
- Its scope is exact: connector, operation and target (for example a channel id).
- It has an expiry and can be revoked.
- Every send under it is recorded with the grant id.
- It never covers an operation outside its scope.

The per-input approvals stay as they are; the standing approval is an added mode, and the
CHANGELOG lists it as a feature. The scope check gets an adversary pass before it merges.
story:standing-write-approval-for-unattended-agents runs in the wave after the MCP waves.
