---
format: aep.planning-md/3
id: architecture-decision-record:standing-scoped-approval
kind: architecture-decision-record
status: accepted
title: A standing, scoped write approval as an added approval mode
summary: Exact scope, expiry, revocation, rate cap and grant id replace steps 5 and 8 for an unattended writer
relations:
- decides: story:standing-write-approval-for-unattended-agents
- decides: story:served-keyed-writes-under-a-standing-approval
- informed_by: decision-blocker:standing-write-approval-mode
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-30T03:19:13Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
---
## Context

An agent running unattended (a pod with no operator at a terminal) needs to write through
Connectors: open and merge a pull request, send a message. Every such write is approval-gated,
and an approval today binds one exact input, lives at most `u16` seconds and is spent once
(`contracts/operations/v1alpha1/semantics.md` steps 5 and 8). Nobody is there to issue one.

`decision-blocker:standing-write-approval-mode` was cleared on 2026-10-22 with option A (the
operator's answer "a", relayed by the orchestrator): a standing approval, granted explicitly by
the operator, with an exact scope (connector instance, operation, target), an expiry
(`not_after`), revocation before it, a rate cap (`max_per_hour`), and every write under it
recorded with the grant id. The per-input approvals stay as they are; the standing approval is an
added mode.

Two placements need it: the local CLI in a container (`story:standing-write-approval-for-unattended-agents`)
and a served instance on `POST /v1alpha2/invoke` (`story:served-keyed-writes-under-a-standing-approval`,
wave D243-C1), where the caller is admitted by a service-account token and the provider
credential never enters the caller's pod.

## Decision

1. A standing grant is an added approval mode. Steps 5 and 8 (verify one exact subject, spend
   once) do not apply to a write admitted under a grant; in their place the host checks, before
   any attempt is recorded and before any provider request:
   - the grant names this instance and this operation;
   - every target member the grant names (a JSON pointer into the operation's input and its exact
     value, such as `/owner` and `/repo`) is present in the input with that value;
   - now is before `not_after`;
   - the grant is not revoked;
   - fewer than `max_per_hour` writes were admitted under the grant in the current hour.
   A write that fails any check is refused by name with no provider request and no attempt.
2. Every write admitted under a grant records the grant id with its attempt.
3. A write under a grant is keyed: the caller supplies `idempotency_key`, the host keeps the keyed
   reservation of operations §5.1, a repeat of the same key and input replays the original
   outcome with no second provider request and no second count against the rate cap, and the
   same key with other input is refused as a conflict. An unattended caller retries; the key is
   what makes a retry safe.
4. Served placement: the grants are part of the instance's mounted configuration, a file the
   operator writes and the host reads on every call. A removed grant, or one listed as revoked,
   is a revocation from the next call on, with no restart and no revalidation. The served host
   signs nothing and is issued nothing: the grant's authority is the operator's deployment, the
   same authority that configures the instance and its credential.
5. Local placement: a grant is issued on the operator's host, signed with the existing
   approval-signing key, and mounted into the container under `mounted_secret` custody; the
   container verifies the signature and never signs (that story's Shape section). A changed or
   removed grant file is read on the next call as a revocation, not as a credential rotation
   waiting for `revalidate_connection`.
6. The hourly count lives in the host's state directory, beside the attempt ledger.
7. Nothing grants a standing approval by default, and `approval` (a per-input approval) on the
   served binding stays refused until its own binding exists.

## Consequences

- The approval contract gains a second mode; `contracts/operations/v1alpha1/semantics.md` and
  `contracts/service/compatibility.md` §2.1 say which checks replace steps 5 and 8 under it.
- `ess/domains/standing_approvals.yaml` models the grant: id, instance, operation, target,
  `not_after`, revocation, `max_per_hour`, and the per-hour count.
- A served instance can write only what its operator granted; a write the grant does not name is
  `approval_required` with no provider request.
- The served placement trusts its mounted configuration as it already trusts its mounted
  credential: whoever can change the instance's Secret can widen its grants. That is the same
  boundary as today's credential and is stated, not hidden.
