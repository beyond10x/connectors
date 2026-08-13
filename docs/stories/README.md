# b10x/connectors — backlog

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
[../research/](../research/). Repository housing is governed by
[`b10x/architecture` ADR 0006](https://github.com/b10x/architecture/blob/main/adr/0006-b10x-supersedes-selfdirect-housing.md).

## Status

**M1 has landed.** The catalog builds here: `catalog build` compiles `providers/` plus the vendored
spec cache into `catalog/`, the pack, `connectors.lock` and the site projection, and the one-time
byte-differential against the predecessor's pack passed byte-exact. What M1 left open is the `post-m1`
epic below — there is no CI at all, the web explorer is written against a site JSON the projection no
longer emits, coverage lost a direction in the test consolidation, three inputs are orphaned, and the
predecessor's generator identity is still stamped into every artifact.

The platform families are unstarted; nothing should be scaffolded ahead of the build order in
[02-architecture.md §9](../design/02-architecture.md). The `ready` stories are the catalog's day-one
changes (architecture §2) plus the post-M1 repairs; the milestone stories M2–M5 are containers that
will spawn children as each milestone is designed.

## Index

| ID | Title | Status | Pillar | Areas |
|---|---|---|---|---|
| [S-001](S-001-the-document-carries-the-callers-contract.md) | The document carries the caller's contract, so nothing at runtime parses source | ready (1) | Catalog | catalog, catalog-build, connector-resolve |
| [S-002](S-002-effects-are-read-never-derived.md) | Per-operation effects are read from the document, never derived | ready (2) | Catalog | catalog, catalog-build, domain |
| [S-003](S-003-the-lockfile-gets-a-verifier.md) | `catalog check` recomputes every hash and exits non-zero on drift | ready (3) | Catalog | catalog-build, connector-spec |
| [S-015](S-015-retire-the-quirks-umbrella.md) | Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts | ready (4) | Catalog | catalog, catalog-build, connector-spec |
| [S-016](S-016-sources-are-processed-by-code.md) | Sources are processed by code: the index is validated, checksummed and refreshed by the tool | ready (5) | Catalog | catalog-build, connector-spec |
| [S-020](S-020-a-ci-gate-exists.md) | A CI gate exists, and it runs what the repository claims it runs | ready (6) | Catalog | ci, catalog-build, web |
| [S-018](S-018-the-explorer-works-against-the-new-site-json.md) | The web explorer works against the site JSON M1 actually emits | ready (7) | Catalog | web, catalog-build |
| [S-017](S-017-mint-source-entries-from-the-mined-catalogs.md) | Mint source entries from the mined competitor catalogs | backlog | Catalog | catalog-build, docs |
| [S-019](S-019-retire-the-flux-connectors-identity.md) | Retire the flux-connectors identity from the artifacts | backlog | Catalog | catalog, catalog-build, connector-resolve, web |
| [S-021](S-021-coverage-regains-its-second-direction.md) | Coverage regains its second direction: every gap between declared and published has a reason | backlog | Catalog | catalog-build, providers |
| [S-022](S-022-orphaned-inputs-are-removed-or-re-owned.md) | Orphaned inputs are removed or re-owned | backlog | Catalog | specs, migration, web |
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
| `post-m1` | S-018, S-019, S-020, S-021, S-022 | What the M1 import report left open: no CI at all (S-020), a web explorer written against a site JSON that no longer exists (S-018), the predecessor's generator identity still stamped into every artifact (S-019), the coverage direction the test consolidation dropped (S-021), and three orphaned inputs (S-022). Two of them are ordering constraints on other work: S-019 rides the schema wave, S-022 lands before S-016's check. |
| `sources` | S-016, S-017 | The SOURCES.toml machinery: code that validates, checksums, refreshes and probes every external source — and mints new entries by mining the vendored competitor catalogs (Nango providers.yaml, Airbyte, Apideck, a spec directory) with per-field citations. |
| `build-order` | S-007, S-008, S-009, S-010 | One story per milestone of architecture §9, with that milestone's exit criteria as Acceptance. Containers: each will spawn children. |
| `carried-constraints` | S-011, S-012, S-013, S-014 | Design constraints ported from the predecessors and the research rather than re-derived — the egress aperture (X-143), the webhook routing grammar, the personal-posture OAuth custody question, and auth-as-tool-result. |

## Known gaps in this backlog

- **M1 never had a milestone story**, and now does not need one: the copy-and-extract half shipped
  (catalog dirs, family crates, `catalog-build` minus the emitters, the byte-differential), and what
  it left open is filed as the `post-m1` epic. Its day-one *changes* — S-001, S-002, S-003, S-015 —
  are still open work.
- **Decision 0024's waves 2–6** (slack/jira/confluence/opsgenie; the observability set behind
  declared destinations; aws/huggingface's new credential schemes; kubernetes/docker/sql/websearch;
  the final plugin-host deletion) are unfiled here; S-010 covers wave 1 only.
- **Later-shaped work** named in the vision — SDKs, the platform CLI, an MCP endpoint, the SaaS org
  lifecycle, catalog overlays, the coverage-matrix projection, the meta-tools discovery surface — is
  deliberately unfiled until its milestone is in reach.
