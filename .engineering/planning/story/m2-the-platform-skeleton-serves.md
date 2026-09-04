---
format: aep.planning-md/1
id: story:m2-the-platform-skeleton-serves
kind: story
status: draft
title: M2 — the platform skeleton serves in both postures
refs:
- provider: legacy
  reference: S-007
relations:
- derived_from: epic:build-order
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-007-m2-the-platform-skeleton-serves.md:24`. **read**

- [ ] `crates/domain`, `crates/protocol`, `crates/service`, `crates/server` exist with architecture
      §2's boundaries enforced, not merely intended: no IO and no HTTP in `domain`; `protocol` holds
      versioned wire contracts with `deny_unknown_fields` and bounded diagnostics; `service` holds
      use-cases over ports and is testable without a socket; `server` is composition only.
- [ ] `connectors serve` with no config is the **personal posture**: an owner-permissioned Unix
      socket where supported, otherwise loopback plus an automatically generated high-entropy token
      stored in an owner-only state root that **refuses a working-tree path**. Zero manual
      configuration remains the contract; unauthenticated localhost is not an identity mode.
- [ ] `connectors serve --config platform.toml` in **organization/hosted posture** consumes the
      released B10x Identity verifier/owner bundle. Receiver configuration pins issuer/trust
      roots, exact Connectors audience, expected tenant/trust domain/deployment, and revocation
      posture. A missing, stale, incompatible, or malformed verifier configuration refuses startup;
      identity resolution distinguishes "nothing presented" from "presented and bad".
- [ ] Hosted requests accept only the verifier's closed principal result. Identity browser/CLI
      sessions, upstream-provider tokens, service client assertions, enrollment bearers, and
      caller-written principal/tenant fields are refused as resource authority. Connectors neither
      terminates OIDC nor mints, rotates, or persists hosted Identity credentials.
- [ ] The closed audience scopes from Design 01 are conformance-tested as exact strings. Identity
      service principals remain Identity-owned; connector route capability and receiver-owned
      Grants are separate, narrowing gates. A token-carried Connection/Grant reference is never
      admission proof.
- [ ] Connectors-owned relational state contains stable tenant/principal projections only where
      receiver records need them, plus integrations, connections, Grants, channels, deliveries, and
      connector audit. No Organization, membership, login-session, upstream-token, service-principal
      credential, or reusable service-bearer verifier table exists.
- [ ] Tenant-in-admitted-principal is structural: no constructor, port or route handler accepts a
      tenant beside an admitted identity, and admission compares and refuses on mismatch rather than
      rewriting. Prove it with a compile-level check, not a scanner.
- [ ] Connector management policy is fail-closed. Being authenticated, human, an organization
      member, or an Identity service principal never becomes ambient operator capability.
- [ ] **Routes fence green**: the HTTP surface is an enumerable value compared against a hand-declared
      list with an argument per entry, and `Access` lives on the route rather than inside the handler.
- [ ] The dependency fence classifies every workspace member (catalog / platform / network) and an
      unclassified member fails the test run; the module size discipline is live (soft cap ~1,500
      lines, a breach requiring a named waiver in the fence test rather than silence).
- [ ] Bind policy is fail-closed: personal mode remains owner-permissioned/local; a reachable hosted
      listener cannot start without the compatible Identity verifier and tenant binding.

## Context

Stand up the platform family's four crates with real boundaries. `connectors serve` owns secure
personal-local authentication; organization/hosted posture consumes a pinned B10x Identity
validated-envelope/verifier contract and performs its own connector audience-scope and Grant
admission. Everything after M2 hangs off these boundaries, so tenant-in-principal,
identity-verifier isolation, routes-as-data, and the closed connector-audit vocabulary are cheapest
to make unrepresentable now.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/02-architecture.md`. **read**

Source `note:` field, quoted: “architecture §9 milestone M2. Exit: personal-local authentication plus hosted conformance against the pinned B10x Identity owner bundle, with routes and dependency fences green. Connectors never owns hosted login/session/service credentials. This is a milestone container — it will spawn children; keep the exit criteria here as the definition of done for the milestone, not for one commit”

## Status

`backlog` in the source. Quoted from `docs/stories/S-007-m2-the-platform-skeleton-serves.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-007-m2-the-platform-skeleton-serves.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-15 · 5 revision(s)
- Legacy id `S-007`, recorded as the reference `legacy:S-007`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
