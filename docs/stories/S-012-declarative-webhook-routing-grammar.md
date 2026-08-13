---
id: S-012
title: "Webhook verification and attribution are declared as data, not as a per-provider script"
pillar: Catalog
status: backlog
priority:
design:
epic: carried-constraints
areas: [catalog, connector-spec, service]
note: "architecture §6 and research consequence 3: design the declarative routing grammar against the five existing channel bindings and the Nango corpus BEFORE scaling inbound events. No script escape hatch in v1 — Nango's own data says 94% of 957 providers never needed one"
---

# Webhook verification and attribution are declared as data, not as a per-provider script

## Goal

Give the webhook terminator a declarative rule grammar in the catalog — how a provider's delivery is
verified, which connection it attributes to, which event it is, and how it deduplicates — so
terminating inbound events scales across providers without per-provider code. Nango leaves exactly
two things to scripts (webhook routing, 5.6%; post-connection, 4.3%); this is the boundary we intend
to close declaratively, and the design has to be tested against real providers before it generalizes.

## Acceptance

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

## Progress
- (not started)

## Notes

- Architecture §6: *"per-provider verification + attribution from declarative catalog rules (the new
  grammar — designed against the five existing bindings and the Nango corpus before generalizing; no
  script escape hatch in v1)."* Research: [catalog-precedents.md](../research/catalog-precedents.md)
  § consequences item 3, and [unified-api-platforms.md](../research/unified-api-platforms.md) § 5
  pattern 7.
- Predecessor reading, in order: flux-connectors C-60 (four vendors' "unique" schemes collapse to one
  parameterized algorithm over {digest, encoding, signed-template, tolerance}, proven against vendor
  documentation vectors rather than self-generated fixtures), C-64 (the verified-webhook seam design —
  and its two findings: the body-sourced-timestamp impossibility and the Stripe composite header),
  C-450 (`VerificationScheme` has no state for a signature we know exists and cannot model),
  C-141/C-151 (the HMAC declaration's measured gaps).
- Blocks [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md)'s terminator. Verification of
  *inbound events* is a different concern from credential verification
  ([S-006](S-006-per-service-verification-probes.md)) — the predecessor repeatedly conflated the two
  words; keep them apart in naming.
