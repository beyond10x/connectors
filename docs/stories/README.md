# b10x/connectors — backlog

The opening backlog. One file per story lives in this directory (`S-NNN-<slug>.md`); frontmatter
carries `id`, `title`, `pillar`, `status`, `priority`, `epic`, optional `design`, query-only `areas`,
and an optional one-line `note`. Status is one of `backlog | ready | in-progress | blocked | done`;
`priority` ranks the `ready` stories (lower = higher) and is omitted otherwise.

**The index below is hand-written.** Board generation (`/track:board` and the rest of the `track:*`
commands, which render Now / Next / Blocked / Backlog from story frontmatter into a generated region)
arrives with the track scaffolding later; until then, edit this table when you add or move a story.

Context these stories assume, read current authority in order:
[../design/01-domain-model.md](../design/01-domain-model.md) →
[../design/02-architecture.md](../design/02-architecture.md) →
[../VISION.md](../VISION.md) (historical founding intent), grounded by
[../research/](../research/). Repository housing is governed by
[`b10x/architecture` ADR 0006 — B10x supersedes selfdirect repository housing](https://github.com/b10x/b10x/blob/main/architecture/adr/0006-b10x-supersedes-selfdirect-housing.md).

## Status

**M1 has landed.** The catalog builds here: `catalog build` compiles `providers/` plus the vendored
spec cache into `catalog/`, the pack, `connectors.lock` and the site projection, and the one-time
byte-differential against the predecessor's pack passed byte-exact. S-022 then closed the orphaned
input debt and made Anthropic's shipped API/Admin connector reproducible from explicitly
repository-authored specs. The coordinated catalog wave then retired the predecessor artifact
identity and `quirks` vocabulary, declared host effects, and added the five independent beyond-HTTP
axes. What M1 still left open is tracked by the remaining `post-m1` stories: web and coverage gates.

The first platform-family foundation and a supervised development SIP/RTVBP runtime are present for
S-024/S-027/S-032/S-033. The remaining lifecycle matrix and provider work still follow the build order in
[02-architecture.md §9](../design/02-architecture.md). The `ready` stories are the catalog's day-one
changes (architecture §2) plus the post-M1 repairs; the milestone stories M2–M5 are containers that
will spawn children as each milestone is designed.

## Index

| ID | Title | Status | Pillar | Areas |
|---|---|---|---|---|
| [S-001](S-001-the-document-carries-the-callers-contract.md) | The document carries the caller's contract, so nothing at runtime parses source | done | Catalog | catalog, catalog-build, connector-resolve |
| [S-002](S-002-effects-are-read-never-derived.md) | Per-operation effects are read from the document, never derived | blocked | Catalog | catalog, catalog-build, domain |
| [S-003](S-003-the-lockfile-gets-a-verifier.md) | `catalog check` verifies every addressable hash and refuses unverifiable claims | done | Catalog | catalog-build, connector-spec |
| [S-015](S-015-retire-the-quirks-umbrella.md) | Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts | done | Catalog | catalog, catalog-build, connector-spec |
| [S-016](S-016-sources-are-processed-by-code.md) | Sources are processed by code: the index is validated, checksummed and refreshed by the tool | ready (5) | Catalog | catalog-build, connector-spec |
| [S-020](S-020-a-ci-gate-exists.md) | A CI gate exists, and it runs what the repository claims it runs | in-progress (6) | Catalog | ci, catalog-build, web |
| [S-018](S-018-the-explorer-works-against-the-new-site-json.md) | The web explorer works against the site JSON M1 actually emits | ready (7) | Catalog | web, catalog-build |
| [S-017](S-017-mint-source-entries-from-the-mined-catalogs.md) | Mint source entries from the mined competitor catalogs | backlog | Catalog | catalog-build, docs |
| [S-019](S-019-retire-the-flux-connectors-identity.md) | Retire the flux-connectors identity from the artifacts | done | Catalog | catalog, catalog-build, connector-resolve, web |
| [S-021](S-021-coverage-regains-its-second-direction.md) | Coverage regains its second direction: every gap between declared and published has a reason | backlog | Catalog | catalog-build, providers |
| [S-022](S-022-orphaned-inputs-are-removed-or-re-owned.md) | Orphaned inputs are removed or re-owned | done | Catalog | specs, migration, web |
| [S-004](S-004-adopt-token-response-metadata.md) | An OAuth token response can carry declared metadata into the connection, not the credential store | backlog | Catalog | catalog, connector-spec, service |
| [S-005](S-005-header-name-rate-limit-retry.md) | A rate limit the vendor discloses at runtime can be declared by header name | backlog | Catalog | catalog, connector-spec |
| [S-006](S-006-per-service-verification-probes.md) | A service declares how a credential is verified, and what a failure means | backlog | Catalog | catalog, connector-spec, service |
| [S-007](S-007-m2-the-platform-skeleton-serves.md) | M2 — the platform skeleton serves in both postures | backlog | Platform | domain, protocol, service, server |
| [S-008](S-008-m3-connect-a-provider-and-invoke-it.md) | M3 — connect a real provider, grant it, invoke it | backlog | Platform | domain, protocol, service, server |
| [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md) | M4 — a provider event reaches a client by push and by pull, with provenance | backlog | Platform | domain, protocol, service, server |
| [S-010](S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md) | M5 — flux re-points at the platform and the gitlab plugin is deleted | blocked | Clients | protocol, server, docs |
| [S-011](S-011-deployment-declared-destination-aperture.md) | Deployment-declared destination aperture | backlog | Platform | service, server, domain |
| [S-012](S-012-declarative-webhook-routing-grammar.md) | Webhook verification and attribution are declared as data, not as a per-provider script | backlog | Catalog | catalog, connector-spec, service |
| [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md) | Decide the connect-session ↔ OAuth-callback custody chain in personal posture | backlog | Platform | domain, service, catalog |
| [S-014](S-014-auth-as-tool-result.md) | Not-connected is a next step: the response carries a connect URL | backlog | Platform | protocol, service, server |
| [S-023](S-023-beyond-http-facts-enter-the-document.md) | Beyond-HTTP facts enter the connector document orthogonally | done | Catalog | catalog, catalog-build, connector-spec, connector-resolve |
| [S-024](S-024-one-zero-io-plan-seam-selects-a-closed-driver.md) | One zero-IO plan seam selects a closed built-in driver | in-progress | Platform | domain, service, server, connector-resolve |
| [S-025](S-025-satellite-federation-has-a-threat-modeled-contract.md) | Satellite federation has a threat-modeled contract | backlog | Platform | domain, protocol, service, server |
| [S-026](S-026-one-real-non-http-driver-proves-the-model.md) | One real non-HTTP driver proves the five-axis model | in-progress | Platform | catalog, domain, service, server |
| [S-027](S-027-direct-byte-session-establishment-is-operation-scoped.md) | Direct-byte session establishment is operation-scoped | in-progress | Platform | domain, protocol, service, server |
| [S-028](S-028-external-driver-artifacts-stay-deferred.md) | External driver artifacts stay deferred behind attestation | blocked | Platform | domain, service, server, docs |
| [S-029](S-029-substrate-events-enter-durable-delivery-with-gap-recovery.md) | Substrate events enter durable delivery with gap recovery | blocked | Platform | domain, protocol, service, server |
| [S-030](S-030-raw-proxy-is-break-glass-not-a-model-capability.md) | Raw proxy is break-glass authority, not a model capability | backlog | Platform | domain, protocol, service, server |
| [S-031](S-031-contract-bundles-are-versioned-and-pinned.md) | Connector contract bundles are versioned, signed, and pinned | backlog | Platform | catalog, protocol, ci, docs |
| [S-032](S-032-sip-driver-terminates-one-governed-call.md) | The SIP driver terminates one governed call | in-progress | Platform | catalog, domain, service, server, driver-sip |
| [S-033](S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md) | Neutral RTVBP bridges the call to an application channel | in-progress | Platform | domain, protocol, service, server, rtvbp-voice-endpoint, voice-runtime |
| [S-034](S-034-production-credential-custody-closes.md) | Production credential custody closes with owner evidence | backlog | Platform | connector-secrets, service, protocol, ci, docs |

## Epics in this seed

| Epic | Stories | What it is |
|---|---|---|
| `catalog-day-one` | S-001, S-002, S-003, S-015, S-023 | Architecture §2's day-one catalog changes plus the accepted beyond-HTTP fact shape. S-001, S-003, S-015 and S-023 are done; S-002's declaration half landed in the coordinated wave and only its M2 grant-admission half remains blocked. |
| `catalog-adoptions` | S-004, S-005, S-006 | The three adoptions the precedents analysis ordered by cost/benefit: `token_response_metadata`, header-name rate-limit retry, per-service verification probes. |
| `post-m1` | S-018, S-019, S-020, S-021, S-022 | What the M1 import report left open. S-019 and S-022 are done; Rust/governance/catalog CI exists while its web and remaining failing-first arms remain open (S-020), the explorer still needs its new site JSON (S-018), and reverse coverage remains S-021. |
| `sources` | S-016, S-017 | The SOURCES.toml machinery: code that validates, checksums, refreshes and probes every external source — and mints new entries by mining the vendored competitor catalogs (Nango providers.yaml, Airbyte, Apideck, a spec directory) with per-field citations. |
| `build-order` | S-007, S-008, S-009, S-010 | One story per milestone of architecture §9, with that milestone's exit criteria as Acceptance. Containers: each will spawn children. |
| `carried-constraints` | S-011, S-012, S-013, S-014, S-030 | Design constraints retained from predecessor evidence and restated here: egress aperture, webhook grammar, personal OAuth custody, auth-as-tool-result, and raw-proxy containment. |
| `beyond-http` | S-023, S-024, S-025, S-026, S-027, S-028 | ADR 0010's delivery order: orthogonal document facts, one plan seam, satellite trust, one real driver, direct-byte establishment, and external artifacts last. SIP/RTVBP is the selected proof and is detailed by the native-voice epic. |
| `substrate-integration` | S-029 | The source-scoped, snapshot-first bounded-cursor to durable-delivery bridge and its gap-recovery contract; implementation waits for released substrate vectors and platform crates. |
| `contract-release` | S-031 | Reproducible, signed, pinned schemas and cross-repository conformance bundles. |
| `native-voice` | S-032, S-033 | The two independently provable owner slices: a closed sipx-backed SIP endpoint, then a neutral RTVBP direct-byte bridge and model-free composed call. |
| `credential-production` | S-034 | Connector-owned production lifecycle, custody backends, satellite completion, leakage conformance, and release evidence under architecture ADR 0032's phase-8 gate. |

## Known gaps in this backlog

- **M1 never had a milestone story**, and now does not need one: the copy-and-extract half shipped
  (catalog dirs, family crates, `catalog-build` minus the emitters, the byte-differential), and what
  it left open is filed as the `post-m1` epic. Its day-one changes are now done except S-002's
  intentionally deferred M2 grant-admission half.
- **The predecessor's later migration waves 2–6** (slack/jira/confluence/opsgenie; the observability set behind
  declared destinations; aws/huggingface's new credential schemes; kubernetes/docker/sql/websearch;
  the final plugin-host deletion) are unfiled here; S-010 covers wave 1 only.
- **Application migration waves remain historical planning input.** Their unavailable predecessor
  record is not normative; each future wave must restate its own parity and deletion rules in a
  B10x story before work starts.
- **Later-shaped work** named in the vision — SDKs, the platform CLI, an MCP endpoint, the SaaS org
  lifecycle, catalog overlays, the coverage-matrix projection, the meta-tools discovery surface — is
  deliberately unfiled until its milestone is in reach.
