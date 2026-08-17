# Hosted module request authority

**Status:** accepted implementation design · **Date:** 2026-08-16

## Boundary

Connectors is the only hosted HTTP caller of Work and Ontology. Cloud enforces that reachability
with NetworkPolicy, but labels authenticate no person. Connectors therefore issues a distinct
request authority after it has resolved Identity's exact audience envelope and admitted the
selected Connector operation.

The authority is compact Ed25519 JWS with protected type
`b10x.module-request.v1+jws`. Claims bind:

- Connector issuer, key ID, and exact Work or Ontology audience;
- verified `tenant_id`, initiating Identity `sub`, and immediate `act` actor;
- canonical module operation and the admitted grant list;
- exact HTTP method and path/query, SHA-256 of the body, and optional idempotency-key digest;
- authority-snapshot identity and digest;
- issue/not-before/expiry times with a maximum 30-second lifetime; and
- a random one-time `jti` redeemed durably by the receiver.

Any missing, stale, replayed, wrong-audience, wrong-operation, malformed, or byte-mismatched
authority fails closed. Identity/session tokens, provider credentials, and static module bearers
never reach the owner service. Signing failure refuses the invocation; no direct-owner fallback
exists.

## Tenant and user attribution

Hosted configuration lists the exact tenants this Connector deployment may project to modules.
The Babelforce developer profile lists one tenant because it is a single-organization deployment.
The runtime still selects the tenant from each verified `PrincipalContext` and refuses one outside
that configured set. Subject and actor are carried separately so a person's resource ownership and
an immediate delegated/service actor are not collapsed.

Work event cursors, deduplication IDs, and stored event envelopes are partitioned by tenant. An
unpartitioned legacy checkpoint is accepted only when exactly one admitted tenant provides an
unambiguous mapping. Synthetic tests exercise independent tenant checkpoints.

## Key custody

The private key file is deployment-owned, absolute, owner-readable, and used only by the
B10x Integration signer. Modules receive a public key under their own audience. Key ID is
part of the protected header. A rotation may configure a bounded public-key overlap at receivers;
private Identity, database, SIP, Slack, and provider credentials are never reused.

This design implements
[Architecture ADR 0041](../../../../architecture/adr/0041-hosted-domain-modules-require-connector-signed-request-authority.md).
