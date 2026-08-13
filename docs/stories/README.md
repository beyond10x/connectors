# selfdirect/connectors — backlog

The opening backlog. One file per story lives in this directory (`S-NNN-<slug>.md`); frontmatter
carries `id`, `title`, `pillar`, `status`, `priority`, `epic`, optional `design`, query-only `areas`,
and an optional one-line `note`. Status is one of `backlog | ready | in-progress | blocked | done`;
`priority` ranks the `ready` stories (lower = higher) and is omitted otherwise.

**The index below is hand-written.** Board generation (`/track:board` and the rest of the `track:*`
commands, which render Now / Next / Blocked / Backlog from story frontmatter into a generated region)
arrives with the track scaffolding later; until then, edit this table when you add or move a story.

Context these stories assume, read in order: [../VISION.md](../VISION.md) →
[../design/01-domain-model.md](../design/01-domain-model.md) →
[../design/02-architecture.md](../design/02-architecture.md), grounded by
[../research/](../research/). The consolidation record is
`~/projects/flux-roadmap/decisions/0026-the-family-consolidates-into-selfdirect-connectors.md`.

## Status

Pre-v1, design phase — nothing builds yet, and nothing should be scaffolded ahead of the build order
in [02-architecture.md §9](../design/02-architecture.md). The `ready` stories are the catalog's
day-one changes (architecture §2); the milestone stories M2–M5 are containers that will spawn
children as each milestone is designed.

## Index

| ID | Title | Status | Pillar | Areas |
|---|---|---|---|---|
| [S-001](S-001-the-document-carries-the-callers-contract.md) | The document carries the caller's contract, so nothing at runtime parses source | ready (1) | Catalog | catalog, catalog-build, connector-resolve |
| [S-002](S-002-effects-are-read-never-derived.md) | Per-operation effects are read from the document, never derived | ready (2) | Catalog | catalog, catalog-build, domain |
| [S-003](S-003-the-lockfile-gets-a-verifier.md) | `connectors catalog check` recomputes every hash and exits non-zero on drift | ready (3) | Catalog | catalog-build, connector-spec |
| [S-015](S-015-retire-the-quirks-umbrella.md) | Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts | ready (4) | Catalog | catalog, catalog-build, connector-spec |
| [S-004](S-004-adopt-token-response-metadata.md) | An OAuth token response can carry declared metadata into the connection, not the credential store | backlog | Catalog | catalog, connector-spec, service |
| [S-005](S-005-header-name-rate-limit-retry.md) | A rate limit the vendor discloses at runtime can be declared by header name | backlog | Catalog | catalog, connector-spec |
| [S-006](S-006-per-service-verification-probes.md) | A service declares how a credential is verified, and what a failure means | backlog | Catalog | catalog, connector-spec, service |
| [S-007](S-007-m2-the-platform-skeleton-serves.md) | M2 — the platform skeleton serves in both postures | backlog | Platform | domain, protocol, service, server |
| [S-008](S-008-m3-connect-a-provider-and-invoke-it.md) | M3 — connect a real provider, grant it, invoke it | backlog | Platform | domain, protocol, service, server |
| [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md) | M4 — a provider event reaches a client by push and by pull, with provenance | backlog | Platform | domain, protocol, service, server |
| [S-010](S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md) | M5 — flux re-points at the platform and the gitlab plugin is deleted | backlog | Clients | protocol, server, docs |
| [S-011](S-011-deployment-declared-destination-aperture.md) | Deployment-declared destination aperture | backlog | Platform | service, server, domain |
| [S-012](S-012-declarative-webhook-routing-grammar.md) | Webhook verification and attribution are declared as data, not as a per-provider script | backlog | Catalog | catalog, connector-spec, service |
| [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md) | Decide the connect-session ↔ OAuth-callback custody chain in personal posture | backlog | Platform | domain, service, catalog |
| [S-014](S-014-auth-as-tool-result.md) | Not-connected is a next step: the response carries a connect URL | backlog | Platform | protocol, service, server |

## Epics in this seed

| Epic | Stories | What it is |
|---|---|---|
| `catalog-day-one` | S-001, S-002, S-003, S-015 | Architecture §2's day-one changes to the migrating catalog. S-001, S-002 and S-015 all change the document schema, the lowering, and every committed document — one implementor or a strict sequence, never parallel authors. S-015 additionally waits on the M1 byte-identity differential. |
| `catalog-adoptions` | S-004, S-005, S-006 | The three adoptions the precedents analysis ordered by cost/benefit: `token_response_metadata`, header-name rate-limit retry, per-service verification probes. |
| `build-order` | S-007, S-008, S-009, S-010 | One story per milestone of architecture §9, with that milestone's exit criteria as Acceptance. Containers: each will spawn children. |
| `carried-constraints` | S-011, S-012, S-013, S-014 | Design constraints ported from the predecessors and the research rather than re-derived — the egress aperture (X-143), the webhook routing grammar, the personal-posture OAuth custody question, and auth-as-tool-result. |

## Known gaps in this seed

- **M1 has no milestone story.** Its day-one changes are S-001/S-002/S-003 (and S-015 after the
  differential), but the copy-and-extract half — migrating `providers/`, `specs/`, `catalog/`,
  `scripts/`, `connectors.lock` and the catalog-family crates, extracting `catalog-build` minus the
  emitters and legacy writers, and running the one-time pack differential against the predecessor —
  still needs a story of its own.
- **Decision 0024's waves 2–6** (slack/jira/confluence/opsgenie; the observability set behind
  declared destinations; aws/huggingface's new credential schemes; kubernetes/docker/sql/websearch;
  the final plugin-host deletion) are unfiled here; S-010 covers wave 1 only.
- **Later-shaped work** named in the vision — SDKs, the platform CLI, an MCP endpoint, the SaaS org
  lifecycle, catalog overlays, the coverage-matrix projection, the meta-tools discovery surface — is
  deliberately unfiled until its milestone is in reach.
