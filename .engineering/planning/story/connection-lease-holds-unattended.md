---
format: aep.planning-md/3
id: story:connection-lease-holds-unattended
kind: story
status: active
title: A revalidated connection keeps serving reads, and concurrent revalidations finish within the admission bound
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T23:35:15Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T23:35:15Z", actor: "human:timo", revision: 4}
---
## Outcome

A connection a caller revalidated once keeps serving reads for as long as its evidence and
credentials allow, without a manual revalidate before each call, and two concurrent revalidations
of one connection both finish within the admission bound.

## Today

Reported by a downstream consumer running connectors 0.50.0 (3f3d8c78052c) live, read calls only,
2026-10-14:

- connections are `pending` at rest;
- a revalidated connection serves reads for under about 66 s;
- two concurrent `connections revalidate` calls time out at 62–68 s.

Seen on a separate host the same day: the first revalidate after a configuration edit and owner
restart timed out at admission (`retry_explicitly`), and the retry answered `ready` after 24.5 s.
The 0.34.0 release plan recorded the timeout as not reproducing; on 0.50.0 it reproduces. This
gates unattended use of every operation that has a fluxplane equivalent (48 of 71).

## Change

Diagnose first: a reproduction against a live-shaped fixture (an evidence lifetime of 60 s, the
default; two revalidations started together), timing each stage (admission, the lifecycle lock,
the adapter probe, publication). Then spec first: if the fix changes when evidence is refreshed or
how concurrent revalidations are joined, the auth and CLI contracts and `ess/domains/cli.yaml`
model it before the code. Expected shape, to confirm against the trace: a read that finds its
evidence expired refreshes it once under the connection's lock, instead of refusing; a second
revalidation joins the first instead of queueing behind it to the timeout.

## Acceptance

- A test: a connection revalidated once serves reads across three evidence lifetimes with no
  caller revalidate.
- A test: two concurrent revalidations of one connection both answer `ready`, each within the
  admission bound.
- The cause of the 60 s-class admission timeout is named with file:line in the PR, verified by a
  trace of one instance.

## Scope

Primary: connectors-host local/registry (revalidation.rs, use_and_retirement.rs, observation.rs), owner/supervisor.rs, owner.rs, metadata.rs WAIT_BOUND, transport.rs; apps local/connections.rs, local.rs; contracts cli semantics.md:801 ("never revalidates on its own" must change), owner.md, auth read-refresh-once.md, evidence semantics; ess cli.yaml, credential_evidence.yaml. Needs a diagnosis (several 30 s bounds; 60 s EVIDENCE_LIFETIME_MS in adapters auth.rs). Collides with revision-recovery (wave c B, registry) and refusals-name-the-field (local.rs, cli.yaml).
