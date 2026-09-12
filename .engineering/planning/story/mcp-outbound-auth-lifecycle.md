---
format: aep.planning-md/1
id: story:mcp-outbound-auth-lifecycle
kind: story
status: draft
title: Specify outbound MCP credential entry, persistence, refresh, repair and revocation
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-domain-model
- depends_on: story:mcp-outbound-connection-lifecycle
scope:
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/auth.md
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/scenarios
revision: 2
---
## Acceptance

`adapters/mcp/contracts/client/v1alpha1/auth.md` specifies the full credential
lifecycle for an outbound MCP server — protected entry or acquisition, persistence
across CLI exit and restart, expiry, refresh, repair and revocation — such that
the four failure states the epic separates (missing, expired, revoked,
unavailable) each have a distinct named outcome, and none of them can be satisfied
by proceeding with no credential.

## Scope

- `adapters/mcp/contracts/client/v1alpha1/auth.md` — new. No other story writes
  this file.
- `adapters/mcp/contracts/client/v1alpha1/scenarios/` — shared directory, also
  written by `story:mcp-outbound-connection-lifecycle` and
  `story:mcp-outbound-invocation-results`; this story adds only credential-state
  scenario files and edits no sibling's file.

## Domain relations

**`McpServerBinding → credential custody` is `UNMAPPED:`** in
`story:mcp-domain-model`, and this document must not resolve it by prose. The
shape it would take if the answer is "reuse the established owner" is citable, and
this document cites it as the candidate rather than the decision:
`ess/domains/credentials.yaml:41-45` declares
`connectors.credentials.CredentialGeneration.connection`, `kind: references`,
`cardinality: one`, `via: connection_ref`, onto
`connectors.auth_bindings.Connection`; and
`ess/domains/auth_bindings.yaml:105-106` declares a `Connection`'s
`active_generation` and `active_custody_version` as `references`, `cardinality:
one`. Whether an MCP server binding *is* such a `Connection` is precisely what the
marker holds open.

What is **not** open, and what this document states as a requirement rather than a
choice, is the separation the epic mandates: "Caller credentials for inbound
access, credentials for an outbound MCP server and underlying provider credentials
are distinct." An outbound MCP server credential is never a provider credential
and never reaches a provider.

## What this story must establish

The epic's outbound deliverable: "authenticated connection and persistent local
credential use/repair", and its auth deliverable: "protected credential
entry/acquisition, persistence, expiry/refresh/repair, revocation and failure.
Credential custody is replaceable and local by default."

Acceptance criterion 1 supplies the observable test this document must make
checkable: the CLI "exits/restarts and reuses its persisted credential binding.
Missing, expired, revoked or unavailable credentials produce distinct safe
outcomes."

The established owners this maps onto, all already documented:
`contracts/auth/acquisition/v1alpha1/semantics.md` owns "managed
begin/complete/refresh/revoke/repair flows" including "OAuth2 code, client
credentials, static entry and static_config" (`contracts/README.md`, index table);
`contracts/auth/custody/v1alpha1/semantics.md` owns "immutable secret versions,
credential sets, CAS active reference"; `contracts/auth/management.md` owns
"host management ownership and protected completion"
(`contracts/README.md`, closing paragraph). `initiative:complete-local-connectors:19`
records that this direction is in scope and that its local baseline holds: "The
admitted OS Secret Service collection owns credentials. ... Local native credential
entry is protected; native SaaS OAuth onboarding is excluded. MCP browser OAuth and
refresh are included."

The distinct-outcomes requirement has a shape already in the store to map onto:
`ess/domains/connection_admission.yaml:8-10` declares
`connectors.connection_admission.ConnectionState` with `reauthorization_required`,
`custody_unavailable` and `revoked` as separate variants. A document that folds
three of the epic's four states into one is refuted by that enum.

## What it does not cover

Session establishment and transport — `story:mcp-outbound-connection-lifecycle`.
Any inbound caller credential — that is a different trust boundary and is not
specified here. No credential is acquired, stored or exercised; this is an
authored document and its scenarios.
