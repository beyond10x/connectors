# 17 — Attempt-bounded subscription credential leases

Status: accepted 2026-09-01. Resolves design 16 open question 3 and backs S-076.

## Decision

Connectors remains the durable owner of a user's subscription credential. Agent Platform may ask
for a short-lived capability only after Identity has exchanged that user's session for the
`connectors.credentials.lease` scope. The capability is bound to one opaque Harness attempt id, a
maximum lifetime of one hour, and a finite use count. Harness redeems it only at the provider
bearer boundary; Agent Platform and Devcenter never receive the stored credential.

This is a narrow capability relay, not a credential export API. There is no route that reads by
tenant or subject, no enumeration, no operator bypass, and no response that combines Identity
authority with credential material. The redemption route accepts only the cryptorandom lease
bearer and its exact attempt id. The credential is returned once per admitted use in a redacted,
zeroizing client allocation and must be discarded immediately after constructing the provider
request.

## Authority and lifecycle

- Presence, replacement and disconnect require an Identity access token for
  `connectors.connections.self`; callers can affect only the verified tenant and subject.
- Lease creation requires `connectors.credentials.lease`. Identity sessions are never accepted by
  the redemption route.
- A lease token is stored only as SHA-256 in process memory and compared in constant time. Live
  leases are intentionally not durable: restart revokes all of them.
- Attempt mismatch, expiry, a zero or greater-than-1,024 use count, an unknown id or a wrong bearer
  all produce the same refusal. At most 10,000 live leases are admitted by one process.
- Credential replacement and disconnect serialize with redemption and revoke every capability
  over the previous value. A capability can never float from one credential generation to the
  next.
- The setup token has no refresh flow. Replacement is explicit rotation; redemption always uses
  the current connected generation or refuses.

## Transport and storage

The hosted feature is disabled unless `[claude_code] enabled = true` and a Vault-backed secret
store is configured. Credential bodies are bounded to 16 KiB. Lease and redemption bodies are
bounded separately. Every successful response on this surface carries `Cache-Control: no-store`
and `Pragma: no-cache`; the typed client refuses a credential-bearing response without both.

The storage address is derived from the verified tenant and a one-way subject digest under the
permanent `com.anthropic.claude-code` authority. The catalog provider remains `custody_only` and
publishes no service or operation, so Connectors cannot spend the value through discovery,
invocation, channels, events, or raw proxying. Only the separately authenticated lease redemption
can cross the custody boundary.

## Deliberate limits

The first implementation is one provider because the provider contract and Harness bearer shape
are known. The authorization and lease machinery may become generic after a second subscription
provider demonstrates the shared fields; the API must not pretend that all vendor credentials have
the same lifecycle today. Durable or transferable leases, background service-agent credentials,
refresh grants, and administrator export remain out of scope and refused.

## 2026-09-01 amendment — refresh before redemption

S-077 admits refresh grants inside Connector custody. This does not widen the lease: redemption
still returns only a current access credential for its exact attempt and use. When the stored
access credential enters its refresh skew, custody serializes refresh with redemption, validates
the provider response, handles refresh-token rotation, replaces the same secret-store record, and
returns the new access credential. Refresh tokens never cross the custody boundary.

The earlier `setup token has no refresh flow` statement remains true for legacy pasted values; it
no longer describes the preferred OAuth-backed record.
