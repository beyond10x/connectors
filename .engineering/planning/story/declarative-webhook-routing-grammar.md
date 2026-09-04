---
format: aep.planning-md/1
id: story:declarative-webhook-routing-grammar
kind: story
status: draft
title: Webhook verification and attribution are declared as data, not as a per-provider script
refs:
- provider: legacy
  reference: S-012
relations:
- derived_from: epic:carried-constraints
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-012-declarative-webhook-routing-grammar.md:23`. **read**

- [ ] The grammar expresses, as data: the **verification matrix** (digest, encoding, signed template,
      tolerance), the **discriminator** that labels an event, the **delivery-id selector** for dedup,
      the **attribution rule** that resolves a payload to a connection, and the **payload map** — over
      a **closed** event set, no wildcards.
- [ ] Coverage is **measured, not asserted**, before generalizing: all five existing channel bindings
      re-express with no loss (byte-comparable behaviour against their current fixtures), and the
      Nango providers that declare a `webhook_routing_script` (54 of 957) are each classified as
      expressible or named as a defeat, with the count recorded in the design.
- [ ] Two known defeats of the predecessor's shape are expressible or explicitly refused with a named
      reason:
      **(a)** Stripe's composite `Stripe-Signature: t=…,v1=…,v0=…` — the digest is neither the whole
      header value nor a literal prefix of it, the timestamp is a *component of the same header*, and
      several `v1=` arrive during a secret rotation, so a verifier admits if **any** candidate
      matches;
      **(b)** a **body-sourced timestamp**, which is unimplementable by construction for a
      verify-before-parse terminator (it is an input to the decision about whether the body may be
      parsed at all) and must therefore be refused at **build** time, in the repository that owns the
      declaration, not at load time in the consumer.
- [ ] A verification scheme we know exists and cannot yet model is a **declarable third state**, not a
      guess and not a lie: the predecessor had no way to say "the vendor signs, and we cannot describe
      how", so the only declarations that loaded were wrong ones.
- [ ] **No script escape hatch in v1.** A declaration attempting one is refused at load by name, and
      the justification (Nango's 94%) is restated where the schema documents itself, so the next
      person to want one has to defeat the grammar with a real provider first.
- [ ] Adversarial fixtures per rule kind: wrong signature, replayed timestamp outside tolerance,
      unknown discriminator, unattributable payload, duplicate delivery id — each refused by name and
      counted, never absorbed. Failing-first test named per rule kind.

## Context

Give the webhook terminator a declarative rule grammar in the catalog — how a provider's delivery is
verified, which connection it attributes to, which event it is, and how it deduplicates — so
terminating inbound events scales across providers without per-provider code. Nango leaves exactly
two things to scripts (webhook routing, 5.6%; post-connection, 4.3%); this is the boundary we intend
to close declaratively, and the design has to be tested against real providers before it generalizes.

Source frontmatter: pillar Catalog · areas [catalog, connector-spec, service]. **read**

Source `note:` field, quoted: “architecture §6 and research consequence 3: design the declarative routing grammar against the five existing channel bindings and the Nango corpus BEFORE scaling inbound events. No script escape hatch in v1 — Nango's own data says 94% of 957 providers never needed one”

## Status

`backlog` in the source. Quoted from `docs/stories/S-012-declarative-webhook-routing-grammar.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-012-declarative-webhook-routing-grammar.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 1 revision(s)
- Legacy id `S-012`, recorded as the reference `legacy:S-012`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
