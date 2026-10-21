---
format: aep.planning-md/3
id: story:connection-follows-configuration-upgrade
kind: story
status: active
title: A connection follows a configuration upgrade without credential re-entry
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/tests/local_runtime/revision_recovery.rs
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/binding_scope_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/upgrade_tests.rs
- confidence: inferred
  path: docs/local-catalog-provider.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T02:14:55Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "draft", at: "2026-10-19T18:41:43Z", actor: "human:timo", revision: 6}
- {from: "draft", to: "proposed", at: "2026-10-21T21:05:13Z", actor: "human:timo", revision: 11}
- {from: "proposed", to: "active", at: "2026-10-21T21:05:13Z", actor: "human:timo", revision: 12}
---
## Problem

A connection is bound to the exact configuration revision of its instance
(`registry_instances.configuration_revision`, `registry/lifecycle.rs` `register`). Any change that
moves the revision — a rebuilt bundle, a selection change, a new provider executable, a CA file —
leaves every connection of the instance stale: `connect` answers `Conflict`, `repair`
`IdentityMismatch`, and the documented way out is a new instance id and a fresh credential entry
(`docs/local-catalog-provider.md` "Connect it again under a new `instance` id"; "Configuration
upgrades are not implemented"). On 2026-10-06 the operator had to be asked for a Zendesk OAuth
client secret again after a bundle gained one cited parameter, although nothing about the
credential, the provider host or the identity had changed.

## Outcome

A connection follows a configuration upgrade of its instance without credential re-entry when the
authentication it was admitted under is unchanged. `connections revalidate` on a connection whose
binding revision differs from the configured one performs the upgrade: it reads the connection's
current custody version (never asking for material), runs the new provider's declared validation
against it, and publishes the connection under the new revision only when

- the instance id, adapter id and provider authority are unchanged;
- the profile id and profile revision are unchanged (the revision digests scheme, capability,
  purpose, subject, entry fields and minimum scopes, so a scope change also refuses);
- the validated external identity equals the connection's recorded identity.

Anything else refuses with the existing codes and `next_action` naming `repair` or a fresh
`connect`, and changes nothing. A revoked connection is never upgraded. Every other connection of
the instance keeps its own state until it is revalidated itself.

## Acceptance

Narrowed 2026-10-21 against the 0.54.0 behaviour (see Scope). Each line names its holder.

- `connections revalidate`, given no credential source, across a changed admitted host set or a changed request prefix (same instance, authority, profile and identity), answers `ready` and the binding names the configured revision. The same change whose validation answers another identity publishes nothing and leaves the connection on its old revision. Holder: new tests in `crates/connectors-host/src/local/registry/upgrade_tests.rs`.
- The CA-file journey (`adapters/catalog/tests/local_runtime/revision_recovery.rs`, `a_changed_ca_file_stales_the_connection_until_it_is_revalidated`) asserts the revision the revalidate answer names.
- Every `#[ignore]` test in `revision_recovery.rs` is classified in `crates/connectors-build/src/ignored.rs`, and the ignored-test manifest check passes.
- `docs/local-catalog-provider.md` states the upgrade path (bundle, selection and executable changes follow; CA, host and prefix changes wait for `connections revalidate`; a lifetime change needs revoke then connect) and keeps the new-instance wording for a changed provider host or `auth` field.

## Not in scope

Upgrading across a changed provider authority, profile or identity; moving material between
connections; reviving a revoked connection.

## Delivered in 0.54.0, and what remains

Most of this story shipped under ADR D12 (`CHANGELOG.md` 0.54.0, Changed, second bullet): a
connection is bound to the credential side of its configuration (`credential_revision`) and to
the provider executable, not to the operation list. A bundle, source or selection change leaves
the connection `ready`; a provider executable change alone validates the stored credential once
and follows when the recorded identity answers, refusing as `identity_mismatch` otherwise; a CA
file, token CA, admitted host or request prefix change gives `pending` with `stale` until
`connections revalidate`, whose next run records the new revision. No credential is asked for in
any of these.

Remaining, to be re-derived against that behaviour before work starts:

- A change of only `evidence_lifetime_ms` still needs `connections revoke` and a new connect
  (`docs/local-catalog-provider.md` around line 1083).
- `docs/local-catalog-provider.md` still tells the operator to give the instance a new id for a
  changed provider host or `auth` field; that refusal is deliberate and out of this story's scope.
- Whether the acceptance lines on a revalidate across a changed CA file or admitted host are
  already held by a test is not established here.

The story returns to draft so the narrowed remainder is decided before it is proposed again.

## Scope

Derived 2026-10-21 by `story-scoper` at e26481df09. Remainder only: tests, a manifest and one docs paragraph; no production line expected.

- **Primary surface:** `crates/connectors-host/src/local/registry/upgrade_tests.rs` — cited
- **Files:** `adapters/catalog/tests/local_runtime/revision_recovery.rs:530-542` (CA journey, `#[ignore]`) — cited
- **Manifest:** `crates/connectors-build/src/ignored.rs:211-216` classifies 3 of the 11 `#[ignore]` tests in `revision_recovery.rs` (626, 662, 684); 510, 532, 550, 572, 598, 727, 756 and 783 are unclassified — cited
- **Also likely:** `crates/connectors-host/src/local/registry/binding_scope_tests.rs` (beside `a_trust_change_keeps_the_explicit_revalidation`, :226) — inferred
- **Documents:** `docs/local-catalog-provider.md:1089-1100` — inferred
- **Safety fact:** host and prefix changes follow the CA path because `changed` (`revalidation.rs:98-114`) and `upgradable` (`:125-131`) compare authority and profile, not the credential side — cited, unproven at runtime

### Held already (non-ignored tests)

Acceptance lines 1-4 of this story: `upgrade_tests.rs:148, :200, :236, :377, :412`. The docs no longer carry "Configuration upgrades are not implemented".

### Not this story

A change of only `evidence_lifetime_ms` stays revoke then connect under the same instance id: contract scenario C05 and `contracts/cli/v1alpha1/semantics.md:1345-1348` say so, and `upgrade_tests.rs:995` holds it. `profile.revision` digests the lifetime, so the upgrade comparison refuses it.
