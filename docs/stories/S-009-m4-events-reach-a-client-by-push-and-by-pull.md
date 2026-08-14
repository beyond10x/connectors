---
id: S-009
title: "M4 — a provider event reaches a client by push and by pull, with provenance"
pillar: Platform
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [domain, protocol, service, server]
note: "architecture §9 milestone M4. Exit: a provider event reaches a client by push and by pull, with provenance. Replay-by-id is the category's most conspicuous gap (research §6 pitfall 4) and is acceptance here, not a later nicety"
---

# M4 — a provider event reaches a client by push and by pull, with provenance

## Goal

Make the reverse direction real: the platform owns provider-side transports so events flow without
any client running, terminates and verifies inbound webhooks from declarative catalog rules, stores
events honestly with provenance, and delivers them to clients durably — with **replay-by-id** as a
first-class client API rather than a support ticket.

## Acceptance

- [ ] **Channel supervisor**: owns the provider-side transports declared by the catalog's channel
      bindings, per connection instance, with opaque host-minted ids and a restart-safe lifecycle the
      platform owns. A channel bound to a connection survives that connection's reauthorization
      (S-008's stable-id invariant).
- [ ] **Webhook terminator**: one inbound endpoint; per-provider verification and attribution come
      from declarative catalog rules ([S-012](S-012-declarative-webhook-routing-grammar.md));
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

## Progress

- 2026-08-14 — a personal-local Slack Socket Mode alpha now supervises one owned transport per
  durable Connection, persists normalized native events before acknowledgement, deduplicates by
  provider delivery id, and exposes bounded generic pull/replay. It is evidence for the channel,
  event-store and pull halves, not M4 completion: generic subscriptions, signed push delivery,
  retries, retention, operational events, audit, hosted authority, and the same-event push+pull exit
  remain open.

## Notes

- Exit criterion, verbatim from architecture §9: *"a provider event reaches a client by push and by
  pull, with provenance."*
- Depends on [S-008](S-008-m3-connect-a-provider-and-invoke-it.md); blocked in substance by
  [S-012](S-012-declarative-webhook-routing-grammar.md), which decides what the terminator reads.
- Predecessor material worth reading rather than re-deriving: flux-exchange X-101/X-102 (persisting
  and supervising generated connector channels), X-103 (grants carry a closed declared inbound event
  set and old grants default to none), X-104/X-105 (multiplexed subscriptions and the live subscribe
  surface), X-122 (channels bound to connection instances).
- Honest delivery is a vision principle (9), not a quality bar: provenance exists so a `polled`
  event is never presented as a vendor push. The researched category's failure here — synthesized
  "webhooks" from a polling engine, plus a documented 24h polling fallback — is the thing we are
  explicitly not doing.
- S-029 specializes the generic per-connection Channel rule for substrate: one supervised Channel
  per `(Connection, source_scope)`, exact native deduplication on
  `(deployment, source_scope, generation, seq)`, and snapshot-first bootstrap. This is accepted
  architecture only; it does not imply that the M4 platform crates or the S-029 adapter exist.
