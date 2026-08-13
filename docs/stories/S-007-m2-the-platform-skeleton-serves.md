---
id: S-007
title: "M2 — the platform skeleton serves in both postures"
pillar: Platform
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [domain, protocol, service, server]
note: "architecture §9 milestone M2. Exit: `connectors serve` healthy in personal and org posture, routes fence green. This is a milestone container — it will spawn children; keep the exit criteria here as the definition of done for the milestone, not for one commit"
---

# M2 — the platform skeleton serves in both postures

## Goal

Stand up the platform family's four crates with real boundaries, and make `connectors serve` a
process that a human can sign into — personal posture with zero configuration, org posture against a
real IdP — carrying organizations, service accounts and audit. Everything after M2 hangs off these
nouns, so the invariants that are structural (organization-in-principal, routes-as-data, closed audit
vocabulary) are cheapest to make unrepresentable now.

## Acceptance

- [ ] `crates/domain`, `crates/protocol`, `crates/service`, `crates/server` exist with architecture
      §2's boundaries enforced, not merely intended: no IO and no HTTP in `domain`; `protocol` holds
      versioned wire contracts with `deny_unknown_fields` and bounded diagnostics; `service` holds
      use-cases over ports and is testable without a socket; `server` is composition only.
- [ ] `connectors serve` with no config is the **personal posture**: loopback bind, local-owner
      identity, one implicit organization, state under an owner-only state root that **refuses a
      working-tree path**. Zero configuration is the tier's contract — nothing may be required of the
      user to reach a healthy process.
- [ ] `connectors serve --config platform.toml` is the **org posture**: OIDC sign-in with PKCE and
      signature-verified claims, one explicit organization, and an operator subject allowlist keyed
      by immutable IdP subject. A missing or malformed operator policy admits **nobody**; an unknown
      config field is refused **by name**; identity resolution distinguishes "nothing presented" from
      "presented and bad".
- [ ] Organizations, Service Accounts and Audit are live: tokens minted by a signed-in human with a
      bounded lifetime and revocation, the store keeping a **verifier and never the token**,
      resolution taking one explicit clock reading; audit with a closed action vocabulary, a closed
      target vocabulary, and **no generic metadata field** — token, body and credential values are
      unrepresentable in the record type.
- [ ] Organization-in-principal is structural: no constructor, port or route handler accepts a tenant
      beside an identity, and admission compares and refuses on mismatch rather than rewriting. Prove
      it with a compile-level check, not a scanner.
- [ ] **Routes fence green**: the HTTP surface is an enumerable value compared against a hand-declared
      list with an argument per entry, and `Access` lives on the route rather than inside the handler.
- [ ] The dependency fence classifies every workspace member (catalog / platform / network) and an
      unclassified member fails the test run; the module size discipline is live (soft cap ~1,500
      lines, a breach requiring a named waiver in the fence test rather than silence).
- [ ] Bind policy is fail-closed: loopback-only while no real identity is armed.

## Progress
- (not started)

## Notes

- Exit criterion, verbatim from architecture §9: *"`connectors serve` healthy in both postures;
  routes fence green."*
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
  [S-003](S-003-the-lockfile-gets-a-verifier.md); the copy-and-extract half of M1 (catalog dirs,
  family crates, `catalog-build` minus the emitters, the one-time pack differential) still needs a
  story of its own.
- Do not scaffold ahead of the build order (AGENTS.md § Boundaries): nothing in this repository
  builds yet, and M2 is the milestone that changes that.
