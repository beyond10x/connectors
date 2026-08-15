# Vision: b10x/connectors

**Status:** historical founding document; superseded where noted below · **Date:** 2026-08-13

> [!IMPORTANT]
> This document preserves the repository's founding intent, not its current ownership model.
> [Design 01](design/01-domain-model.md) and
> [`b10x/architecture` ADR 0003](https://github.com/b10x/architecture/blob/main/adr/0003-identity-and-connectors-are-separate-domains.md)
> move general identity, organizations, login sessions, and foundation access credentials to
> Identity. [Design 07](design/07-credential-custody-topologies.md) and
> [`b10x/architecture` ADR 0032](https://github.com/b10x/architecture/blob/main/adr/0032-central-managed-credential-custody-is-bounded.md)
> replace the owner-file/identical-posture credential language with the bounded current custody
> decision. Read the current domain and architecture before using the historical text below.

## What this is

One repository holding both halves of a **unified integration platform for agent automation**:

1. **The catalog** — third-party providers (GitLab, Slack, Zendesk, …) declared as reviewed text,
   compiled into one canonical, deterministic data artifact per provider: operations with explicit
   request templates, the complete credential surface, configuration fields, events, channel
   bindings, and quirks. The bytes a human reviewed are the bytes the runtime executes.
2. **The platform** — a deployable service that owns identity, organizations, connections,
   credentials, grants, invocation, and event delivery. Users sign in and configure connections
   once; a client then authenticates **once**, holds **one token**, and can do everything its
   grants admit: discover operations, invoke them, receive events.

Vendor credentials live in the platform and never cross to a client. A client holds authority,
never secrets: *the credential never crosses the boundary; the authority does.*

## Why

Agents are becoming the primary consumers of vendor APIs, and they need what human developers
needed — managed auth, one invocation surface, event delivery — plus what humans never needed:
a machine-readable **risk vocabulary** (risk, idempotency, effects, direction per operation) that
an authorization system and an approval UI can actually consume, and auth flows an agent can
drive mid-conversation.

The category exists — Merge, Nango, Composio, Apideck, Paragon (see
[research](research/unified-api-platforms.md)) — and its open flanks are structural:
per-connection pricing that punishes success, self-host tiers crippled below the SaaS product,
raw vendor access sold as the premium escape hatch, webhook delivery without customer-facing
replay, and one shared token store as the industry's blast radius. None of them declares
*operations* as reviewable data, and none has a risk vocabulary deeper than a UI hint
(see [catalog-precedents](research/catalog-precedents.md)).

This repository also consolidates two predecessor codebases (flux-connectors, flux-exchange)
whose hard-won invariants are carried forward as design — and whose structural mistakes
(engine-crate lockstep, runtime parse-back of compiled artifacts, cross-repo release trains for
data changes) are the reason it is **one** repository.

## The three postures

One codebase, one domain model, three deployment postures — **with an identical feature set**.
A posture changes where identity comes from and who operates the deployment, never what the
product can do:

| | **personal** | **org** | **saas** |
|---|---|---|---|
| Runs | on your machine, loopback | on your infrastructure | hosted, multi-org |
| Identity | local owner | your IdP (OIDC) | hosted accounts / SSO |
| Organizations | exactly one, implicit | exactly one, explicit | many |
| OAuth apps | yours (BYO) | yours (BYO) | BYO, shared apps optional |
| Vendor tokens live | in your owner-bound files | in your deployment | per-org envelope-encrypted |

The posture ladder is itself the answer to the centralized-token-store breach class: secrets
concentrate only as far as the user chooses. Connections are portable between postures because
the catalog and the auth templates are the same text everywhere.

## The client contract

- **Authenticate once.** A user signs in and mints a Service Account token; the client presents
  that one token for everything. It grants access to *operations*, never to credentials.
- **Discover, invoke, subscribe.** An effective catalogue (what this principal's grants admit),
  invocation of declared operations, and an event stream — push (signed webhooks with replay) or
  pull (authenticated subscription).
- **Not-connected is a next step, not an error.** When an operation needs a connection that does
  not exist or has degraded, the response carries a connect URL a human can complete —
  auth-as-tool-result, designed for agents that hit the wall mid-task.
- **Context-sized discovery.** The catalog distinguishes *callable* from *worth projecting into a
  model*; search/inspect/execute meta-tools (and later an MCP endpoint) replace dumping hundreds
  of tool schemas into a context window.
- **No generic raw proxy in v1.** Credential-bearing model calls use declared operations. A later
  operator-only break-glass proxy, if accepted, is destructive/max-effect, destination- and
  method/path-bounded, separately granted, and never model-exposed.

## Principles

1. **Review equals execution.** Canonical documents are committed, hashed in a lockfile, and
   every derived form is byte-identical to those inputs. A catalog change is a readable diff.
2. **Declared facts drive authorization.** Grants admit from risk/effects/idempotency the catalog
   declares — never from op-id lists a human maintains. Deny beats allow beats predicate.
3. **Fail closed, refuse by name.** A pack that does not verify refuses startup. An unknown
   schema is refused by name. A refusal names the address, never the value. Nothing repairs.
4. **Secrets are owner-bound and unrepresentable elsewhere.** Owner-only storage with no memory
   fallback; audit records that cannot carry a credential by type; registration identity
   (client_id/secret) is deployment configuration, never catalog content.
5. **Structure over review.** Authorization gates chain through unconstructible proof types —
   skipping one is a compile error, not a finding. The tenant lives inside the principal.
6. **One source of truth.** Status, docs, and projections are derived from canonical artifacts,
   and the derivation is tested. Prose that can drift from reality is a defect class.
7. **No foreign engine in the dispatch path.** The platform consumes and produces *data*
   (request plans, documents, events). No third-party runtime trait ever appears in a dispatch
   signature; behaviour engines integrate as clients, not as dependencies.
8. **The open tiers are the product.** Identical features in every posture; identities and
   connections are never metered in open tiers; raw access is never the premium tier.
9. **Honest delivery.** Events carry provenance (native webhook vs polled); delivery is durable
   per client with replay-by-id. No reliability theater.
10. **Nouns are forever.** The vocabulary is picked once ([domain model](design/01-domain-model.md));
    the catalog schema is versioned, the vocabulary is not.

## Non-goals (v1)

- **No behaviour hosting.** No workflows, no managed apps, no server-side automation of any
  kind. Behaviour lives in clients (an agent runtime, a script, a cron job) against the client
  contract. This keeps the platform's trusted computing base small and its dispatch path
  data-only; a hosted-behaviour tier can arrive later as a *supervised client runtime*, never as
  an embedded engine.
- **No unified data models.** We do not normalize Zendesk tickets and Jira issues into one
  schema. Declared operations are the generic invocation layer; lossiness is the category's tax and
  we decline to charge it.
- **No arbitrary or in-process plugin runtime.** Every integration is a declared connector executed
  by the platform. V1 protocol drivers are built into the closed platform registry; any future
  vendor-specific executable requires a separate attestation decision and runs out-of-process
  through substrate, never as side-loaded code or a caller-selected plugin.

## Relationship to flux

[flux](https://github.com/codewandler/flux) is the first native client: its embedded platform
client (one token, effective catalogue → tool projection, invoke, subscribe) is the reference
consumer of the client contract, and its CLI will manage a personal-posture instance (verified
download, supervised local process). flux depends on this platform's *protocols*, and this
platform depends on nothing of flux — the dependency arrow between the products exists only at
runtime, in one direction, over versioned contracts.

## Later

Go and TypeScript SDKs (`sdks/go`, `sdks/ts`), a platform CLI, an MCP endpoint backed by connect
sessions, the SaaS posture's org lifecycle, and catalog overlays (an organization extending the
official catalog with its own providers — same schema, own signing trust).
