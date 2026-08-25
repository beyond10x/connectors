---
id: S-073
title: "The hosted posture connects a catalogued provider"
pillar: Platform
status: backlog
priority:
design: ../design/16-subscription-credential-custody.md
epic: subscription-custody
areas: [connectors-runtime, service, server, hosted-vault]
note: "CatalogBackend is composed only into PersonalRuntime (composition.rs:344). HostedRuntime wires curated backends only, so all 57 catalogued API-key providers are unconnectable in a deployment. The largest piece of the Connect Claude work, and it is generic, not Anthropic-specific."
---

# The hosted posture connects a catalogued provider

## Goal

Make a catalogued provider connectable in a deployment by a signed-in person, under their own
identity, so "connect Anthropic" in the platform UI reaches the same credential custody the CLI
already reaches on a laptop.

## Acceptance

- [ ] `CatalogBackend` is composed into `HostedRuntime`
      (`crates/connectors-runtime/src/composition.rs:438-742`), backed by `HostedVaultStore` /
      `PreparedVaultStore` and `PostgresState`. It is generic over the catalogue, not special-cased
      per provider.
- [ ] Per-user addressing: one fresh `instance_id` per Connection, keyed to the owner's subject,
      addressed by `CredentialRef::for_instance(tenant, authority, instance_id, service, leaf)` —
      the Jira shape at `crates/integration-jira/src/backend/auth.rs:452-465`.
- [ ] The address is produced by `integration_catalog::credential_address`
      (`crates/integration-catalog/src/lib.rs:798-818`) and by nothing else. A second copy of the
      addressing rule is what made `auth status` report every named instance as not-connected
      (`crates/connectors-console/src/enrol.rs:70-72`).
- [ ] `connect_session_access` returns `SelfService` for catalogued paste credentials: any admitted
      person connects their own credential without an operator.
- [ ] `setup_profiles` advertises the available flows so a UI can render them without hardcoding a
      provider list.
- [ ] Values commit through a prepared transaction — reserve → prepare → pending row → commit →
      reclaim — with crash recovery. **No point write.** A killed process mid-connect leaves no
      half-written credential.
- [ ] Non-credential response fields land in Connection metadata, never the credential store.
- [ ] **Two people, two connections.** A test asserts that a second signed-in person sees and
      resolves only their own credential. This is the per-user claim; it is a test, not a
      click-through.
- [ ] Provider credentials appear in no Operation, Connection or Event contract, and nothing
      credential-shaped reaches logs, audit records, argv, environment or crash artifacts — proven
      with the repository's sentinel convention.

## Progress
- (not started)

## Notes

- Measured 2026-08-25: `CatalogBackend::open` is called once, at
  `crates/connectors-runtime/src/composition.rs:344`, inside `PersonalRuntime` (lines 134-423).
  `HostedRuntime` (438-742) wires Slack, GitLab, Jira, Platform, Monitoring, SIP and Kubernetes and
  no catalogued provider. `PersonalConfig::read_hosted` at `:517` is SIP voice configuration only.
- This is the documented design intent, not a new idea:
  `crates/connectors-console/src/connect.rs:31` carries the guided-provider list with the note that
  the generic catalog-driven flow is what removes it.
- Scope guard: ship it generic. Fifty-seven catalogued API-key providers become deployable by the
  same change; making it Anthropic-shaped would be a third copy of a rule that already has two.
