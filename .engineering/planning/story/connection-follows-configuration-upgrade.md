---
format: aep.planning-md/3
id: story:connection-follows-configuration-upgrade
kind: story
status: proposed
title: A connection follows a configuration upgrade without credential re-entry
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T02:14:55Z", actor: "human:timo", revision: 3}
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
