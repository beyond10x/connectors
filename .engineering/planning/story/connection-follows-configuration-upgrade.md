---
format: aep.planning-md/3
id: story:connection-follows-configuration-upgrade
kind: story
status: draft
title: A connection follows a configuration upgrade without credential re-entry
relations:
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T02:14:55Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "draft", at: "2026-10-19T18:41:43Z", actor: "human:timo", revision: 6}
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

- A connection admitted under revision A revalidates under revision B (same profile, same
  authority, same identity) to `ready`, with no credential source given, and its binding names B.
- The same with a changed provider authority, a changed profile scheme or fields, or an identity
  the new validation answers differently refuses, publishes nothing and leaves the connection on A.
- A revoked connection is not upgraded.
- Invocation under B works after the upgrade; invocation under A is no longer admitted.
- The upgrade is recorded in metadata and the connection registry docs, and
  `docs/local-catalog-provider.md` no longer tells the operator to reconnect under a new instance.

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
