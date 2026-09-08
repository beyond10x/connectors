# Adapter design: catalog, pre-compiled third-party specifications, and the generic HTTP engine

- **Status:** design, not implemented. One adapter crate `adapters/catalog/` (`docs/design.md:212`) building one executable with two realizations: `catalog` (index service) and `provider` (generic engine bound to one bundle). One build-time pipeline in `crates/connectors-spec` and `crates/connectors-build`.
- **Old baseline:** `../connectors` at `81459ac4`: `providers/*.toml` (65), `specs/` (83 files; 32 pinned spec entries over 17 providers in `connectors.lock`), `catalog/*.catalog.json` (65), `connectors.lock`, `SOURCES.toml` (28 sources), crates `connector-spec` (16,479 lines), `catalog-build` (9,594), `catalog`, `catalog-reader`, `catalog-cli`, `connector-resolve` (4,462), `integration-catalog` (3,935).
- **Contract:** [contracts/catalog/v1alpha1/semantics.md](../../contracts/catalog/v1alpha1/semantics.md). Index: [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

"Rebuild the catalog" means three things, and the old repository had all three as separate crates:

| Part | Old | New |
|---|---|---|
| Ingest: third-party or hand-authored declaration → reviewed, pre-compiled artifact | `connector-spec` (TOML and OpenAPI front-ends) + `catalog-build` (`catalog build`) | `connectors-spec --generate` per adapter document + `connectors-build catalog build` over all of them |
| Distribution: which definitions exist, provenance, where the artifact is | `catalog-reader` pack embedded in every consumer; `catalog` typed view | `adapters/catalog` realization `catalog`, or a plain index directory with no process |
| Generic execution: one engine runs every declared HTTP operation | `integration-catalog` ("One adapter for every declared provider", `crates/integration-catalog/src/lib.rs:1`) + `connector-resolve` | `adapters/catalog` realization `provider`, one process per loaded bundle |

The analogy asked for holds: as `integration-sip` was the SIP adapter, `integration-catalog` was the adapter for every catalogued provider; in this repository that is `adapters/catalog` next to `adapters/sip`. The difference from the old system is that the catalog is optional (`docs/design.md:39,147`), the pack is gone (`docs/design.md:79`), and execution authority does not change when a service is found through the catalog (`docs/design.md:1006`).

Scope by numbers (counts over `../connectors/catalog/*.catalog.json` on 2026-09-08): 65 providers, 1,017 operations; 1,001 on `http_v1`, which the generic engine covers; 8 `sql_v1`, 5 `cdp_v1`, 2 `audio_v1`, 1 `sip_v1`, which belong to the SQL and media adapters and are out of scope here. 738 operations are in the 17 spec-backed providers; 279 are in the 48 hand-authored ones. Methods: GET 547, POST 250, PUT 94, DELETE 92, PATCH 18, none 16. Direction: read 568, write 449.

Not all 1,017 become implementation obligations (`docs/design.md:1100`). Ingest produces a bundle per provider with an explicit coverage report; an operation is callable only when its realization is `generic` or `implemented`.

Placement: the pipeline runs offline in development and CI (`docs/design.md:292`). The `catalog` realization runs wherever a composition places it, or not at all. A `provider` realization runs once per provider bundle wherever that provider's origin is reachable; the old `placement_requirement` and `required_capabilities` (`public_network`, `private_network`, `connectors_deployment`; `../connectors/providers/alertmanager.toml:33-37`) are preserved as bundle facts a composition checks.

## 2. Old surface and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `providers/<id>.toml` with `[spec]` pin and `[[patch.operations]]` curation | `../connectors/providers/alertmanager.toml:13-18,26-37` | preserve as `adapters/<id>/spec/adapter.json` (`connectors.adapter/v2` plus the extensions in §8) |
| Hand-authored providers, body fields with `wire` paths and JSON-schema constraints, response schema, error envelope | `../connectors/providers/airtable.toml:352-388` | preserve as adapter documents without `upstream`; `wire` becomes the body template |
| `[spec]` sources: OpenAPI 3 (39 files), Swagger 2.0 (2 files: argocd, slack), AsyncAPI (2 provenance entries, b10x) | `../connectors/specs/`, `specs/*.provenance.toml` | OpenAPI 3 ingested; Swagger 2.0 by exact projection; AsyncAPI recorded only |
| `catalog build` / `check` / `diff`; `catalog sources check | refresh | diff` planned | `../connectors/crates/catalog-build/src/lib.rs:11-25`; `SOURCES.toml:7` | preserve as `connectors-build catalog build | check | diff | sources refresh` |
| Five build invariants: hermetic, deterministic, all-or-nothing, explicit, engine-free | `../connectors/crates/catalog-build/src/lib.rs:34-40` | preserve; "engine-free" now means no provider crate in the catalog graph |
| `catalog/<id>.catalog.json` canonical document, `catalog.pack`, `connectors.lock` | `../connectors/connectors.lock:1-27` | replace with `adapters/<id>/generated/` bundles and `catalog/index.json`; the facts survive, the bytes do not (`docs/design.md:911`) |
| `catalog-reader` embedded pack with vendored SHA-256 | `../connectors/crates/catalog-reader/src/lib.rs:12-40` | drop the embed; keep refusal by name |
| `catalog` typed `&'static` view built on first use | `../connectors/crates/catalog/src/lib.rs:34` | replace with `catalog.*` operations and the client |
| `integration-catalog` join of catalog, resolver, credential assembly, admission plan, egress aperture; excludes acquisition, datasources, events | `../connectors/crates/integration-catalog/src/lib.rs:14-36` | preserve as the `provider` realization; exclusions kept |
| `connector-resolve` request plan from template, input, endpoint slots, credential | `../connectors/crates/connector-resolve/src/lib.rs:1-25` | preserve as the generic engine; endpoint slots (`{origin}`, `subdomain`) become configuration `slots` |
| `verify` operation per provider | `../connectors/providers/alertmanager.toml:11` | `auth.evidence` `verify_operation` |
| `expose = false` | `../connectors/providers/grafana.toml:91` | curation `expose` |
| `[[auth]]` declarations: bearer 50, basic 6, signing 3, named header 17, query parameter 2 | `../connectors/catalog/*.catalog.json` `auth[].scheme` | `auth.profile` per declaration; two new schemes (§5) |
| `[[config]]` fields with `binds = endpoint.origin`, `approval = operator`, `secret`, `level` | `../connectors/providers/alertmanager.toml:39-48` | configuration schema per bundle using `urn:connectors:config:v1:*` imports plus `slots` |
| `channels`, `events`, `discoveries`, `graphs` in the document schema | `../connectors/catalog/connector-document.schema.json` top-level fields | deferred (§9) |

## 3. Contracts needed and why

| Contract | Profile / use | Why |
|---|---|---|
| `catalog` | `index`, `service` | list, describe, locate bundles with provenance; optional |
| `operations` | `generic-http` | the realization for the 1,001 HTTP operations without provider code |
| `operations` | `mutation` | the 449 write operations: `external_write`, risk, idempotency, approval binding |
| `datasource.records` | `generic-http-page` | operations whose mapping declares pagination (8 old) |
| `auth.profile` | one per old `[[auth]]` declaration | schemes and acquisition flows from the declaration, not from OpenAPI security schemes alone |
| `auth.acquisition` | `static_entry` (11 old `connect_session` entries), `oauth2_authorization_code` (5 providers), `oauth2_client_credentials` (1) | token entry and OAuth flows for catalogued providers |
| `auth.capability` | `http-bearer`, `http-basic`, `http-signing`, plus `http-header` and `http-query` (to add) | credential placement for the engine |
| `auth.evidence` | `verify_operation` | the old `verify` read |
| `auth.connection` | `configured` first; `managed` later | one connection per loaded provider bundle |
| `auth.custody` | `read_only` | token custody |

Not needed: `sessions`, `media`, `resource_discovery`, `route.mediated_http`, `datasource.logs`, `datasource.series`. `events` is deferred.

## 4. Operation map

Catalog service:

| New id | Contract / profile | Old |
|---|---|---|
| `catalog.providers.list` | records | `catalog::providers()` |
| `catalog.provider.describe` | operations | `catalog::provider(id)` document |
| `catalog.operations.list` | records | `catalog::operations_of(provider)` |
| `catalog.bundle.describe` | operations | `connectors.lock` provider entry |
| `catalog.sources.status` | operations | `SOURCES.toml` entry |

Catalogued providers: ids are not mapped one by one here. Rule: the old `rename` value with its provider prefix stripped is the default new id inside adapter `<provider>`; curation may override. Examples:

| Old id (source) | New | Contract / profiles |
|---|---|---|
| `alertmanager-alerts-list` (`providers/alertmanager.toml:27-28`) | `alertmanager` / `alerts.list` | operations `generic-http` |
| `zendesk-ticket-audit-list` (`providers/zendesk.toml:122-124`) | `zendesk` / `ticket.audit.list` | operations `generic-http` |
| `airtable-record-get` (`providers/airtable.toml:275`) | `airtable` / `record.get` | operations `generic-http` |
| `airtable-record-create` (`providers/airtable.toml:346`) | `airtable` / `record.create` | operations `generic-http` + `mutation` (`external_write`, risk medium, idempotency `none`) |

## 5. Auth

| Old scheme (count) | `auth.profile` scheme | Capability | Note |
|---|---|---|---|
| `bearer` (50) | `http_bearer` | `http-bearer` | prefix from the declaration |
| `basic` (6) | `http_basic` | `http-basic` | user half from `user_env` / `user_suffix` becomes a configuration field |
| `signing` (3: twilio, slack, stripe) | `http_signing` | `http-signing` | canonicalization deferred with those providers (`contracts/auth/capability/v1alpha1/semantics.md:124`) |
| `header` with a name (17, e.g. `x-api-key`, `DD-API-KEY`, `X-Figma-Token`) | `http_header` (new) | `http-header` (new) | header name from the declaration, value from custody |
| `query` with a name (2: `key`, `token`) | `http_query` (new) | `http-query` (new) | value never appears in provenance, logs, or errors |

The catalog service itself has no provider credential; the host's caller authorization applies to its operations as to any read.

## 6. Configuration outline

`provider` realization, one process per bundle:

```json
{ "service": { "id": "connectors.zendesk" },
  "http": { "base_url": "https://acme.zendesk.com", "credential": { "kind": "environment", "name": "ZENDESK_TOKEN" }, "bearer": true },
  "adapter": {
    "bundle": { "path": "/srv/connectors/catalog/zendesk", "digest": "sha256:…" },
    "slots": { "subdomain": "acme" },
    "expose": ["ticket.show", "ticket.audit.list"],
    "limits": { "response_bytes": 4194304, "timeout_s": 30 } } }
```

`catalog` realization:

```json
{ "service": { "id": "connectors.catalog" },
  "adapter": { "index": { "kind": "path", "path": "/srv/connectors/catalog/index.json" }, "verify_digests": true } }
```

`index` realization: no process; the client or host reads `index.json` and each bundle's `manifest.json` directly, verifying digests.

The bundle's own `configuration_schema` (generated, as `adapters/gitlab/generated/descriptor.json` carries today) is composed with the outline above; provider-specific fields such as `slots` come from the old `[[config]]` declarations.

## 7. Discovery and routes

The catalog is a service-discovery source (`docs/design.md:319-323`): it names services and their artifacts; it does not decide trust. A composition (`docs/design.md:834`) selects bundles by digest and binds endpoints. No mediated routes: a catalogued provider that needs one (Loki through Grafana) is not a generic-engine case and stays with its dedicated adapter design.

## 8. Specification profile

`connectors.adapter/v2` extended, or `/v3` if the extension breaks the closed schema:

| Extension | Reason (old evidence) |
|---|---|
| `upstream` optional; `sources[]` with origin, kind, url, revision, sha256, license, transforms | 48 hand-authored providers; 28 sources with origin vocabulary (`SOURCES.toml`) |
| mapping `method` any of GET, POST, PUT, PATCH, DELETE | 470 non-GET operations |
| mapping `body`: JSON template with `$param` splices; `headers` | 291 body templates; 50 header templates (`connector-document.schema.json` `http_request`) |
| mapping `error_envelope` (`code_pointer`, `message_pointer`), `pagination`, `rate_limit` | 172, 8, and rate-limit declarations in the old documents |
| curation block per operation: `expose`, `effects`, `risk`, `idempotency`, `requires_auth`, `placement` | `[[patch.operations]]` fields; OpenAPI supplies none (`docs/design.md:900-906`) |
| Swagger 2.0 exact projection as a recorded transform | 2 old inputs |
| default `prepare`/`finish` realization for `generic-http` | today required handwritten (`spec-kinds/adapter/v2/semantics.md:20-21`) |

Pipeline per provider: `adapters/<id>/spec/adapter.json` → `connectors-spec --generate` → `adapters/<id>/generated/` (descriptor, coverage, manifest, ESS lowering, upstream pin) → `connectors-build catalog build` → `catalog/index.json`. Toolchain pin: ESS 0.20.0 (`crates/connectors-spec/toolchain.json`). Normal Cargo builds read committed outputs and never invoke ESS or the network (`docs/gitlab-generation.md`).

First slice, chosen to cover each ingest shape once: `alertmanager` (1 GET, vendor OpenAPI 3), `zendesk` (35 operations, vendor spec, POST bodies), `airtable` (4 operations, hand-authored, JSON body envelope), `argocd` (Swagger 2.0). Everything else is inventory until curated (`docs/design.md:1100`).

## 9. Deferred

`events` and `channels` (old inbound declarations), `discoveries` inside bundles, `graphs`, `cdp_v1`/`audio_v1`/`sql_v1`/`sip_v1` operations, OCI distribution of bundles, a successor for the published reader crate, model/tool projection of the catalog.

## 10. Evidence required

- Fixture: generic-engine suite over status, envelope, pagination, body template, limits, redirect refusal, encoding of path parameters; determinism (two generations, identical digests); no-network sentinel over generation and every catalog operation; digest tamper; format-version refusal; catalog independence (`docs/design.md:1006`); value-freedom grep over every output.
- Live: one spec-backed provider through its bundle (Alertmanager on a local instance), one hand-authored provider against a sandbox account, one Swagger 2.0 provider.
- Decoupling: no `adapters/*` crate depends on `adapters/catalog`; `adapters/catalog` depends on no provider crate; the host builds and serves a direct adapter with no catalog crate in its graph; the client discovers a direct service without an index.

Crate name for the engine: `connectors-generic-http`, linked only by `adapters/catalog`.
