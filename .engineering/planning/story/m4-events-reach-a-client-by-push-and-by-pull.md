---
format: aep.planning-md/1
id: story:m4-events-reach-a-client-by-push-and-by-pull
kind: story
status: draft
title: M4 — a provider event reaches a client by push and by pull, with provenance
refs:
- provider: legacy
  reference: S-009
relations:
- derived_from: epic:build-order
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-009-m4-events-reach-a-client-by-push-and-by-pull.md:22`. **read**

- [ ] **Channel supervisor**: owns the provider-side transports declared by the catalog's channel
      bindings, per connection instance, with opaque host-minted ids and a restart-safe lifecycle the
      platform owns. A channel bound to a connection survives that connection's reauthorization
      (S-008's stable-id invariant).
- [ ] **Webhook terminator**: one inbound endpoint; per-provider verification and attribution come
      from declarative catalog rules ([S-012](../../../docs/stories/S-012-declarative-webhook-routing-grammar.md));
      verification runs over the **raw body before parsing**, comparison is constant-time with a
      timestamp tolerance, and a vendor challenge/handshake is answered without waking a client.
- [ ] **Event store**: append-only, provenance (`native` vs `polled`) on every row and honestly
      recorded, dedup by delivery id, and the domain model's family split — operational events
      (credential degraded, delivery failing, channel down) are never mixed into the data stream and
      have their own subscriptions.
- [ ] **Deliveries**: durable per-endpoint queues with the Svix envelope — `id` + `timestamp` +
      HMAC-SHA256 over `{id}.{timestamp}.{body}` with a **dedicated signing key** — and retries with
      backoff. Never sign with an API key; never canonicalize the payload before signing (both are
      named anti-patterns from the researched field).
- [ ] **Replay by id**: a client replays a delivery it names, through the client API, under its own
      grants. The replay is distinguishable from the original (same event id, new delivery attempt),
      is audited, and cannot be used to reach an event the principal's inbound grants do not admit.
- [ ] **Subscriptions**: one authenticated, multiplexed WebSocket per client, gated by inbound
      grants — every requested event must belong to the binding's **closed** declared set; an
      ungranted subset and a cross-tenant channel id share one refusal that discloses neither.
- [ ] **Exit**: one test in which a single provider event reaches a client both by push (signed
      delivery) and by pull (subscription), carrying the same event id and the same provenance.

## Context

Make the reverse direction real: the platform owns provider-side transports so events flow without
any client running, terminates and verifies inbound webhooks from declarative catalog rules, stores
events honestly with provenance, and delivers them to clients durably — with **replay-by-id** as a
first-class client API rather than a support ticket.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/02-architecture.md`. **read**

Source `note:` field, quoted: “architecture §9 milestone M4. Exit: a provider event reaches a client by push and by pull, with provenance. Replay-by-id is the category's most conspicuous gap (research §6 pitfall 4) and is acceptance here, not a later nicety”

## Status

`backlog` in the source. Quoted from `docs/stories/S-009-m4-events-reach-a-client-by-push-and-by-pull.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-009-m4-events-reach-a-client-by-push-and-by-pull.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-16 · 4 revision(s)
- Legacy id `S-009`, recorded as the reference `legacy:S-009`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
