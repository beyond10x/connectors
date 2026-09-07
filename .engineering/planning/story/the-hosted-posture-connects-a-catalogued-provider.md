---
format: aep.planning-md/1
id: story:the-hosted-posture-connects-a-catalogued-provider
kind: story
status: draft
title: The hosted posture connects a catalogued provider
refs:
- provider: legacy
  reference: S-073
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/connectors-config/src/hosted.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: cited
  path: crates/hosted-vault/src/prepared.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
revision: 3
---
## Acceptance

Verbatim from `docs/stories/S-073-the-hosted-posture-connects-a-catalogued-provider.md:21`. **read**

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

## Context

Make a catalogued provider connectable in a deployment by a signed-in person, under their own
identity, so "connect Anthropic" in the platform UI reaches the same credential custody the CLI
already reaches on a laptop.

Source frontmatter: pillar Platform · areas [connectors-runtime, service, server, hosted-vault] · design `../design/16-subscription-credential-custody.md`. **read**

Source `note:` field, quoted: “CatalogBackend is composed only into PersonalRuntime (composition.rs:344). HostedRuntime wires curated backends only, so all 57 catalogued API-key providers are unconnectable in a deployment. The largest piece of the Connect Claude work, and it is generic, not Anthropic-specific.”

## Status

`backlog` in the source. Quoted from `docs/stories/S-073-the-hosted-posture-connects-a-catalogued-provider.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-073-the-hosted-posture-connects-a-catalogued-provider.md`, which is not deleted and now names this artifact.

- First written 2026-08-25 · last touched 2026-08-25 · 2 revision(s)
- Legacy id `S-073`, recorded as the reference `legacy:S-073`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Scope

- **Primary surface:** `crates/integration-catalog/src/hosted.rs:54` — cited; `HostedCatalogBackend` owns hosted acquisition, subject isolation, setup profiles and prepared publication/recovery.
- **Composition:** `crates/connectors-runtime/src/composition.rs:971` — cited; HostedRuntime registers the generic adapter; lines 689–705 supply Vault/prepared custody and line 1269 selects PostgreSQL.
- **Policy:** `crates/connectors-config/src/hosted.rs:78` — cited; `HostedCatalogConfig` controls enablement, admitted providers and session lifetime.
- **Addressing:** `crates/integration-catalog/src/lib.rs:1020` — cited; shared `credential_address` constructs instance-qualified references.
- **Custody dependency:** `crates/hosted-vault/src/prepared.rs:254` — cited; existing prepared-store implementation, with shared-journal restart coverage at line 765.
- **Already implemented:** core behavior exists at released base `4d0cd308` — cited; acquisition uses fresh instances and shared addressing (hosted.rs:435–455), prepared publication/recovery (471–562), SelfService/profile discovery (695–712), and a two-person isolation/address/sentinel test (1259–1324). The story’s “personal only” context is stale.
- **Confidence:** high for ownership and shipped core behavior — cited; production composition and concrete test assertions are present.
- **Would collide with:** hosted catalog acquisition/recovery, runtime composition, hosted policy, shared credential addressing and prepared custody changes — inferred from these owners.
- **Small-wave suitability:** suitable for acceptance/evidence reconciliation first; no missing implementation slice established by this inspection — inferred.
