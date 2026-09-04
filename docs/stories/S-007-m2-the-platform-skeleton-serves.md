---
id: S-007
title: "M2 — the platform skeleton serves in both postures"
pillar: Platform
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [domain, protocol, service, server]
note: "architecture §9 milestone M2. Exit: personal-local authentication plus hosted conformance against the pinned B10x Identity owner bundle, with routes and dependency fences green. Connectors never owns hosted login/session/service credentials. This is a milestone container — it will spawn children; keep the exit criteria here as the definition of done for the milestone, not for one commit"
---

# M2 — the platform skeleton serves in both postures

## Goal

Stand up the platform family's four crates with real boundaries. `connectors serve` owns secure
personal-local authentication; organization/hosted posture consumes a pinned B10x Identity
validated-envelope/verifier contract and performs its own connector audience-scope and Grant
admission. Everything after M2 hangs off these boundaries, so tenant-in-principal,
identity-verifier isolation, routes-as-data, and the closed connector-audit vocabulary are cheapest
to make unrepresentable now.

## Acceptance

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

## Progress

- 2026-08-14 — identity ownership curated before implementation. Personal-local authentication
  remains here; hosted identity is an external owner contract. Hosted stability cannot be claimed
  before the Identity owner bundle and shared negative fixtures exist.
- 2026-08-14 — the personal-local alpha slice now binds an owner-credentialed, mode-`0600` Unix
  socket under an owner-only state root, serves the immutable generic Connector operation bundle,
  and refuses worktree state in the product CLI. Zero-config mode advertises no operations. This
  does not complete M2: non-Unix fallback, hosted Identity conformance, routes-as-data, persistence,
  and the remaining platform skeleton criteria stay open.

## Notes

- Exit criterion from the 2026-08-14 architecture amendment: personal posture is healthy; hosted
  conformance passes the pinned Identity owner bundle; route and dependency fences prove no
  Identity implementation or persistence entered this repository.
- **[S-002](S-002-effects-are-read-never-derived.md)'s grant-admission half is anchored here**
  (decision of 2026-08-13, recorded in S-002's Progress): when `crates/domain` exists, grant
  admission reads the document's declared effects, the no-derivation fence lands, and S-002's
  failing-first admission test stops being vacuous. Whoever designs M2's children should spawn
  that work as one of them.
- The predecessor's two-crate split (host/server) was right; its failure mode was god modules — a
  10.7k-line route file is a **named anti-goal**. The four-crate split exists to move the pressure
  points out of the transport crate; if a module here starts to grow, that is the signal this
  milestone was built wrong, not that the cap is inconvenient.
- M1 (the catalog migration) is not a milestone story in this seed: its day-one changes are
  [S-001](S-001-the-document-carries-the-callers-contract.md),
  [S-002](S-002-effects-are-read-never-derived.md) and
  [S-003](S-003-the-lockfile-gets-a-verifier.md). The copy-and-extract half of M1 (catalog dirs,
  family crates, `catalog-build` minus the emitters, and the one-time pack differential) has
  shipped; remaining post-M1 work is tracked by the existing web, coverage, and source-tooling
  stories rather than a missing milestone story.
- Do not scaffold the platform family ahead of the build order (AGENTS.md § Boundaries). The catalog
  family already builds; M2 is the milestone that introduces the deployable platform family.

## Superseded by

`story:m2-the-platform-skeleton-serves` in the AEP planning store, at
`.engineering/planning/story/m2-the-platform-skeleton-serves.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
