---
format: aep.planning-md/3
id: story:standing-write-approval-for-unattended-agents
kind: story
status: draft
title: A standing, scoped write approval for an agent running unattended
summary: One connection, one operation, input constraints, a rate cap and an expiry, set once by a person
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

An agent running unattended, with Connectors in its container, sends a Slack message within limits
a person set once: one connection, one operation, a channel allow-list, a rate cap and an expiry.
A call outside those limits is refused by name and makes no provider request.

Asked by the Loom Kubernetes agent design (question Q2): its `reply.send` effect in `auto` mode
needs this. Until it lands, that design runs in `approval` mode.

## Blocked

decision-blocker:standing-write-approval-mode blocks this story. The shape below assumes option A.
If Timo picks B or C, this story is rewritten or withdrawn.

## Before proposed

- An ADR for the approval contract change (`contracts/operations/v1alpha1/semantics.md` steps 5
  and 8 bind one exact input and spend once).
- The standing approval modelled in ESS beside `ess/domains/approval_issuers.yaml`, through
  `ess:specifying`: its scope, constraints, counter and expiry. It is a new noun.
- Scope records from `aep:story-scoper`.

## Shape under option A

- Issued on the operator's host by a new `connectors approvals issue-standing` command, signed with
  the existing approval-signing key.
- Written to a file the container mounts; read under `mounted_secret` custody
  (`contracts/auth/custody/v1alpha1/semantics.md` §5.2), which already refuses signing in the
  container.
- Verified on each call: connection, operation, each input constraint, `not_after`, and the count of
  calls in the current hour.
- Each call is recorded in the attempt ledger with the standing approval's id.

## Acceptance (named conformance scenarios, to be confirmed with the ESS model)

- `a_send_to_an_allow_listed_channel_is_admitted`
- `a_send_to_another_channel_is_refused_with_no_request`
- `a_call_over_max_per_hour_is_refused`
- `a_call_after_not_after_is_refused`
- `a_standing_approval_for_one_operation_admits_no_other`
- `each_admitted_call_is_recorded_with_the_standing_approval_id`
