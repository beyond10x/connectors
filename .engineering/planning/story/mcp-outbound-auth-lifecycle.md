---
format: aep.planning-md/3
id: story:mcp-outbound-auth-lifecycle
kind: story
status: implemented
title: Specify outbound MCP credential entry, persistence, refresh, repair and revocation
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-domain-model
- depends_on: story:mcp-outbound-connection-lifecycle
scope:
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/auth-cases.json
- confidence: cited
  path: adapters/mcp/contracts/client/v1alpha1/auth.md
- confidence: inferred
  path: crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T22:44:32Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-02T22:44:32Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-03T00:10:02Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
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

## Scope — confirmed 2026-10-03

Read-only aep:story-scoper inspected integration a2955675 and its reviewed candidate.
Cited: adapters/mcp/contracts/client/v1alpha1/auth.md, named in acceptance.
Inferred: adapters/mcp/contracts/client/v1alpha1/auth-cases.json and
crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs, new named document
cases and Rust guard. These replace the broad scenarios directory allocation.
Confidence medium: exact document is cited; sufficient guard design is inferred.
Collisions are limited to these three paths. Existing shared/native models,
contracts, scenario YAML, manifests and gates remain read-only. The gate excludes
outbound lifecycle YAML from ESS synthesis (gate.rs:258-268); JSON document cases
must not be called runtime conformance. Safety is source-inspected, not executed.

Document-only requirements can specify persistence, refresh, repair and revocation
without choosing the unresolved McpServerBinding/Connection/custody relations.
Those relations need native ESS design before executable persistence implementation.
The four failures remain distinct named outcomes without inventing new shared error
variants. Derive revision-specific discovery/registration from pinned archives.
Cover acknowledged restart binding, protected completion, issuer/audience mismatch,
uncertain rotating-token refresh with no second exchange, publication-only recovery,
identity-preserving repair, local versus provider revocation, and maintenance never
replaying a business call. Use mutated-copy controls for the same failure classes.
No actual credentials, restart acceptance, stdio ownership or provider forwarding.

## Implementation and first review correction — 2026-10-03

Authored contract and48 literal document cases cover both pinned revisions. The five initial checks first failed on the absent contract; author mutation testing subsequently caught stale restart reuse and required the current publication fence. Initial targeted25checks, formatting and scopedClippy passed. No native custody/Connection relation, OAuth flow or persistent implementation was introduced.

Independent review-result:mcp-auth-20261003 added a failing test: all48 contradictory duplicate Markdown rows were accepted by the document guard. The correction parses full structural Markdown table rows with unique identifiers and rejects misplaced rows, fenced/quoted substitutions and duplicated tables; it retains the reviewer regression and supports valid row reordering. The corrected affected suite executed27checks, zero failures/ignored; formatting and scopedClippy exited0. Contract/JSON unchanged; guard SHA25613319d3bf3dd607eaee0ad00e6b68e20920248be2562f69ffce1344fffafba2b. Correction report SHA2565c3e190c6218749bf006b45cd4b35587a4cabcec7984e202e2c7dc5536cfc7f2. Final bounded independent pass and integration gate remain pending; this is document conformance only.

## Final bounded review and correction — 2026-10-03

The second attack found one adjacent unsupported-HTML bypass. AEP findings comparison: carried0, new1 (whole HTML blocks bypass correspondence), resolved1 (contradictory Markdown duplicates). The final correction unconditionally rejects InlineHtml/Html events and documents that authored subset. Coordinator compared the guard against the frozen second-pass baseline: only the rejection condition/message and the preserved appended reviewer test differ; no assertion was removed or relaxed. Both reviewer regressions remain unchanged. No third attack ran.

Final28affected checks passed, zero failed/ignored, with formatting and scopedClippy exit0. Final source hashes: auth.md5beefe75580f987957e96c4553257e19a9fb6df6a0b3e159e9b74808c2b13209; auth-cases.json65d655e428eeaa03afc57af1182246131e94359b874e97ec4086a10d94830b48; guard8cbdb0638b95a4e1ad8a0e96a4c2c4b5b627cd8202a1c3e1372187ea91b8a8ea. Final correction report SHA25643b72c76f836ecf85c83a10285e0a31a5fb8c78827175bb1991f7be3ab5fed7e. Exact three source files integrated; full repository gate pending.
