---
format: aep.planning-md/3
id: architecture-decision-record:credential-less-public-reads
kind: architecture-decision-record
status: proposed
title: A credential-less profile admitted only for operations a catalog declares public and read-only
revision: 1
---
## Context

A consumer asks to read public GitHub repository metadata without a token. The host's local runtime admits a provider auth profile only when it names a credential scheme and at least one credential field (`crates/connectors-host/src/local/runtime.rs:595-612`); every invocation runs through a connection whose evidence was taken with a secret, and the approval and audit records name that connection. There is no path for a call that carries no credential.

## Decision (proposed)

1. A new profile scheme, `none`, with no credential fields, no acquisition and no identity probe.
2. Each catalog operation may carry `public: true`. A provider's `none` profile loads only if every operation it serves is `effect: read` and `public: true`; otherwise the load is refused naming the operation. The rule is checked at bundle load and again at invoke.
3. A `none` profile still runs through a connection: the connection holds no secret, its evidence is a reachability probe of the provider's base URL (no subject), and its revision changes only with configuration. The audit record names the connection and records `credential: none`.
4. No approval is needed for a public read, as for any read today.
5. Conditional requests are a per-connection, bounded answer store (`ETag`, body, content type) under the connection's state directory, owner-only, holding only public answers; a 304 returns the stored body. It is used only on `none` profiles in this decision.
6. Rate limits: a header-keyed `rate_limited` refusal with the reset time (shared with the token profile).

## What it changes

- Contract: the host's credential rule gains one admitted exception, bounded by (2). Every existing profile, operation and refusal is unchanged.
- Threat model: a public read can leak nothing a credential protects, since none is held. Risks: an operation wrongly marked public (mitigated by the read-only rule, review of every `public` mark, and a test per provider that lists exactly its public ids); the answer store on a shared-user host (every session runs as one user, so the store is readable by any of them; it holds only answers anyone on the internet can read); a misconfigured host sending a credential header on a `none` profile (the host attaches no secret because none exists).

## Alternatives

- A token with no scopes: works today, but the operator wants no token for public repositories.
- Direct HTTP in the consumer: bypasses the record of what was read.

## Status

Proposed. The operator decides it before any implementing wave.
