# catalog/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** catalog. No siblings. Companion profile defined here: `operations/v1alpha1` `generic-http` (the realization that makes a catalogued operation callable without handwritten provider code).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `catalog/v1alpha1` |
| Realizations | `index` (a directory of bundles plus one index file, no process), `service` (an adapter service answering the operations below over the ordinary wire) |
| Companion profile | `operations/v1alpha1` `generic-http`; paged variant `datasource.records/v1alpha1` `generic-http-page` |
| Relation to `operations/v1alpha1` | catalog operations are ordinary read operations; the catalog is optional and grants nothing (`docs/design.md:39,147,325,1006`) |

A catalog answers three questions: which reviewed adapter definitions exist, with what provenance, and which artifact (by digest) realizes each. It never answers whether a caller may invoke anything; execution authority is unchanged whether a service was found directly or through a catalog (`docs/design.md:1006`). Bundles are data; the executable they name is separate trust (`docs/design.md:808`).

The pipeline this contract describes has three parts, matching the old repository's three parts: ingest (third-party or hand-authored declaration → pre-compiled bundle), distribution (index and service), and generic execution (a bundle's `generic-http` operations run by one engine). The adapter that hosts the last two is [adapters/catalog/design.md](../../../adapters/catalog/design.md).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `providers/<id>.toml`: `[spec]` pin (path, source_url, upstream_version, fetched_at, sha256) plus `[[patch.operations]]` curation (select, rename, direction, risk, idempotency, effects, placement, capabilities) | `../connectors/providers/alertmanager.toml:13-18,26-37` | preserve as the v2 adapter document's `upstream` and `mappings` (`adapters/gitlab/spec/adapter.json`) plus a per-operation curation block the v2 kind lacks today |
| Hand-authored providers with no vendor document: 48 of 65 providers, 279 of 1,017 operations; body fields declared with `wire` paths | `../connectors/providers/airtable.toml:1-11,352-388`; counts over `../connectors/catalog/*.catalog.json` | preserve: the adapter kind must accept mappings without `upstream` (today `upstream` is required, `spec-kinds/adapter/v2/schema.json` `required`) |
| `catalog build`: TOML + spec → canonical `catalog/<id>.catalog.json` (18 top-level fields, `catalog/connector-document.schema.json`) → `catalog.pack` + `connectors.lock`; hermetic, deterministic, all-or-nothing, explicit, engine-free | `../connectors/crates/catalog-build/src/lib.rs:11-17,34-40` | preserve the five invariants; replace document plus pack with the per-adapter `generated/` bundle (`adapters/gitlab/generated/`) plus one index |
| `catalog-reader`: embedded offset-indexed pack, vendored SHA-256, refusal by name of a newer container or schema version or digest mismatch | `../connectors/crates/catalog-reader/src/lib.rs:12-40` | preserve refusal by name; drop the custom pack and the embed-into-every-consumer model (`docs/design.md:79,1095`) |
| `catalog` typed view: `Risk` low/medium/high, `Idempotency` idempotent/non_idempotent/conditional; no field a secret value could live in | `../connectors/crates/catalog/src/lib.rs:60-66,87-93`; `crates/catalog/README.md:30` | map to the mutation profile's `risk` and `idempotency.kind` (`natural`/`none`/`keyed`); preserve value freedom |
| `integration-catalog`: one adapter for every declared provider; `connector-resolve`: template + input + credential → request plan; acquisition, datasources and events explicitly excluded | `../connectors/crates/integration-catalog/src/lib.rs:1,28-36`; `crates/connector-resolve/src/lib.rs:1` | preserve as the `generic-http` profile realized by the catalog adapter; keep the exclusions |
| `connectors.lock`: per provider generator, `toml_sha256`, `ir_sha256`, spec pins, artifact digests; `[pack]` digest | `../connectors/connectors.lock:1-27` | preserve the facts in the index; the pack digest becomes the index digest over every bundle file |
| `SOURCES.toml`: 28 sources with kind, origin, refresh policy, pins, consumers; origins vendor 7, repository-authored 22, vendor-derived 4, mixed 1 | `../connectors/SOURCES.toml:1-9`; `specs/*.provenance.toml` | preserve as the bundle's `sources[]` record with the same origin vocabulary |
| Source transforms recorded as provenance kinds, including `swagger-2-to-openapi-3-exact-projection` | `../connectors/specs/slack.provenance.toml`; Swagger 2.0 inputs `specs/argocd/swagger-v3.5.1.json`, `specs/slack/web-api-1.7.0-2026-08-15.swagger.json` | preserve: a transform is a named, digest-recorded step between vendor bytes and the ingested document |
| `verify` operation per provider | `../connectors/providers/alertmanager.toml:11` | preserve as the descriptor's `auth.evidence` `verify_operation` reference |
| `expose = false` operations exist but are not published | `../connectors/providers/grafana.toml:91` | preserve: `expose` in curation; an unexposed operation is absent from the descriptor |
| Request template vocabulary: `method`, `url`, `query`, `headers`, JSON `body` template with `$param` splices; `error_envelope` (`code_pointer`, `message_pointer`, 172 operations); `pagination` (8 operations); `rate_limit` (bucket, per_seconds, requests) | `../connectors/catalog/connector-document.schema.json` `$defs` | preserve in the specification kind's mapping vocabulary (section 8); the generic engine consumes it |
| Method split over 1,017 operations: GET 547, POST 250, PUT 94, DELETE 92, PATCH 18, none 16; 291 body templates; drivers `http_v1` 1,001, `sql_v1` 8, `cdp_v1` 5, `audio_v1` 2, `sip_v1` 1 | counts over `../connectors/catalog/*.catalog.json` on 2026-09-08 | the generic profile covers `http_v1` only; the 16 non-HTTP operations belong to the SQL and media adapters |

## 3. Types

### 3.1 Catalog operations

`catalog.providers.list` (paged like `datasource.records`):

```json
{ "limit": 100, "cursor": null }
```

```json
{
  "items": [
    {
      "adapter": "alertmanager",
      "version": "0.1.0",
      "kind_version": "connectors.adapter/v2",
      "contracts": ["operations/v1alpha1", "datasource.records/v1alpha1"],
      "operations": { "exposed": 1, "unresolved": 0, "refused": 0 },
      "sources": [ { "origin": "vendor", "kind": "openapi", "url": "https://github.com/prometheus/alertmanager/blob/v0.31.0/api/v2/openapi.yaml", "revision": "v0.31.0", "sha256": "0fac1b1f…", "license": "Apache-2.0" } ],
      "bundle": { "format": "connectors.generated-bundle/v1", "digest": "sha256:…" },
      "toolchain": { "ess": "0.20.0", "rustfmt": "1.9.0-stable", "generator": "connectors-spec 0.1.0" }
    }
  ],
  "next_cursor": null,
  "complete": true,
  "provenance": { "instance": "catalog", "resource": "index", "observed_at_unix_ms": 0, "source_revision": "sha256:…" }
}
```

`catalog.provider.describe` `{ "adapter": "alertmanager" }` → the bundle's descriptor (shape of `adapters/gitlab/generated/descriptor.json`: `adapter`, `configuration_schema`, operations) plus a coverage summary per operation (`generated`, `requires_implementation`, `refused`, from `adapters/gitlab/generated/coverage.json`, format `connectors.import-coverage/v1`) and the curation block per operation.

`catalog.operations.list` `{ "adapter": null, "contract": "operations/v1alpha1", "profile": "mutation", "effects": ["external_write"], "limit": 100, "cursor": null }` → items:

```json
{ "adapter": "zendesk", "id": "ticket.create", "contract": "operations/v1alpha1", "profile": "mutation",
  "effects": ["external_write", "network"], "risk": "medium", "idempotency": { "kind": "none" },
  "requires_auth": [{ "profile": "zendesk.api_token", "scopes": [] }], "realization": "generic" }
```

`catalog.bundle.describe` `{ "adapter": "zendesk", "digest": "sha256:…" }` → manifest fields (`format`, `specification_sha256`, `upstream_sha256`, `ess`, `rustfmt`, `files` with per-file digests, as in `adapters/gitlab/generated/manifest.json`) plus `locations: [ { "kind": "path" | "oci" | "https", "reference": "…" } ]`. Bytes are not served over this contract; a host fetches by location and verifies the digest.

`catalog.sources.status` `{ "adapter": "zendesk" }` → per source: pinned revision, sha256, `fetched_at`, refresh policy (`manual`, `authored-review`), and `drift: "unknown" | "none" | "upstream_changed"`. `drift` is `unknown` unless a deliberate, separately invoked refresh check recorded a result; a read never contacts a vendor.

| Field | Rule |
|---|---|
| `realization` | `generic` (complete mapping, run by the engine), `implemented` (handwritten obligations satisfied in an adapter crate), `unresolved` (coverage lists `requires_implementation` or `refused`; never advertised as callable) |
| `sources[].origin` | `vendor`, `repository-authored`, `vendor-derived`, `mixed` (old vocabulary) |
| `sources[].kind` | `openapi`, `swagger2`, `authored`, `asyncapi` (recorded, not ingested) |
| `bundle.digest` | SHA-256 over the manifest, which carries every file digest |
| `toolchain` | exact versions from the manifest; a pin change requires regeneration (`docs/gitlab-generation.md`) |

Errors: base codes; `NotFound` for an unknown adapter or digest; `Unsupported` for a bundle or index format version newer than the reader understands (refusal by name, old `catalog-reader` rule); `Unavailable` naming the file when a digest does not match; `StaleCursor` for an index generation that was replaced.

### 3.2 The `generic-http` profile

A generic read carries `profile: generic-http` and `realization: generic`. A generic mutation carries the singular `profile: mutation` and `realization: generic`, including all mutation admission/observation rules. The following is an illustrative extended read descriptor; profile availability remains subject to the auth vocabulary owner:

```json
{ "id": "alerts.list", "description": "Read alerts", "contract": "operations/v1alpha1", "profile": "generic-http", "realization": "generic",
  "effects": ["network"], "semantic_effects": [], "risk": "low", "idempotency": { "kind": "natural" }, "approval": "not_required",
  "limits": { "request_bytes": 262144, "result_bytes": 4194304, "execution_ms": 40000, "provider_ms": 30000, "connect_ms": 5000 },
  "requires_auth": [{ "profile": "alertmanager.configured", "scopes": [] }],
  "input_schema": { "type": "object", "properties": { "active": { "type": "boolean" } }, "additionalProperties": false },
  "output_schema": { "type": "object", "required": ["status", "body", "provenance"] } }
```

Outcome: `status` (provider HTTP status), `body` (provider JSON, bounded, passed through unchanged), `provenance` (`instance`, `resource` = the mapping's path with parameters redacted to their names, `observed_at_unix_ms`, `source_revision` = the bundle digest).

Status to safe `ErrorCode` for **generic reads only** (base vocabulary in `crates/connectors-core/src/lib.rs:13-27`):

| Provider result | Code |
|---|---|
| 400, 422 | `InvalidInput` |
| 401 | `Unauthorized` |
| 403 | `Forbidden` |
| 404, 410 | `NotFound` |
| 409, 412 | `InvalidInput` |
| 429 | `RateLimited`, `retry_after_seconds` from `Retry-After` when present |
| 5xx, connection refused, TLS failure | `Unavailable` |
| deadline exceeded | `Timeout` |
| non-JSON body, oversize body, redirect | `UpstreamProtocol` |

The public `Error.message` is a host-selected safe classification, bounded to 512 UTF-8 bytes. Never copy raw provider messages, codes, response bodies, URLs or credential material into it. Declared error-envelope pointers may contribute to a reviewed operation-specific classifier; they do not authorize public disclosure or select a core code automatically.

For **generic mutations**, status alone does not prove business-effect knowledge. Use the mutation contract’s admitted `not_attempted`, proven `refused`, known `applied`, or conservative `unknown` observation. Ambiguous 5xx, lost responses, malformed or oversized replies after possible dispatch give `outcome_unknown` unless independent definitive evidence exists, with a safe secondary cause. A provider 409/412 is not the receiver’s `idempotency_conflict`; that code belongs exclusively to a conflict in its admitted key namespace. Any definitive refusal mapping must prove no business effect under the selected operation semantics.

Profile selection is singular: a generic operation whose curation declares `external_write` selects `mutation` with `realization: generic` and all its rules. A mapping that declares `pagination` is exposed as `datasource.records/v1alpha1` profile `generic-http-page` (items from the declared items pointer, cursor from the page or cursor parameter, `complete` from the declared end condition) instead of `generic-http`.

## 4. Rules

- Optional: a directly configured service is fully usable without an index or a catalog service (`docs/design.md:325,1006`).
- Grants nothing: no operation here returns a credential, a credential address, an admission decision, or an endpoint the client must trust; the client's trust decision is its own (`docs/design.md:325`).
- Data, not executable: a bundle names its realization artifact by digest; installing or running it is explicit configuration (`docs/design.md:808`).
- Truthful descriptors: only `generic` and `implemented` operations appear in a descriptor; an `unresolved` operation is listed by the catalog and absent from the service (`docs/design.md:311,894`).
- Curation is mandatory: an imported operation without `expose`, `effects`, `risk` and `idempotency` is `unresolved`, because OpenAPI supplies none of them (`docs/design.md:900-906`).
- Provenance is complete: every bundle records origin, exact URL and revision, SHA-256 of the original bytes, every transform, and the vendor license file where one exists (`adapters/gitlab/upstream/LICENSE`, `spec-kinds/adapter/v2/semantics.md:16-18`).
- Hermetic reads: catalog operations and generation never contact a vendor; refresh is a separate, deliberate, reviewed action (`docs/design.md:292`; old `catalog-build` invariant).
- Deterministic: identical inputs and toolchain produce identical bundles and an identical index digest (`spec-kinds/adapter/v2/semantics.md:52-55`).
- Refusal by name: an unknown bundle or index format version is refused before any record is served.
- Engine discipline: the request is built only from the mapping, the validated input, and the host-injected credential capability; the caller supplies no URL, method, header, or path; body comes from the declared template with `$param` splices only; the response is bounded; redirects are not followed; one hop.
- Unary only: the generic profile serves unary HTTP operations; datasources beyond `generic-http-page`, events, channels, discoveries and acquisition are not served by the engine (old `integration-catalog` exclusions preserved).
- Public authentication metadata: bundle files, descriptors, indexes and catalog outputs may name safe auth-profile identifiers and configuration requirements, but contain neither credential values nor actual runtime credential locators (custody/version references, environment-variable names, filesystem paths or secret-store addresses). A schema may describe a protected configuration slot; it cannot populate that slot with a deployment's credential location. The receiving host owns those private bindings and injects an admitted capability at runtime. Reject an artifact violating this boundary before publication or serving; value freedom alone is insufficient.

## 5. Ordering and limits

| Concern | Rule |
|---|---|
| Page | 1–100 as `datasource.records` |
| Index size | unbounded, paged; one generation at a time |
| Generic request | 256 KiB including the extended envelope; selected first-profile ceiling |
| Generic result | 4 MiB including envelope and metadata; a provider body too large to fit gives safe `UpstreamProtocol` for reads; mutations preserve actual effect knowledge |
| Generic deadlines | 40 s total execution, 30 s provider within it, 5 s connect within provider time; advertised through Operation.limits. No independent budget reset or silent clipping to the legacy 20 s execution limit |
| Retry | none by the engine; `RateLimited` carries `retry_after_seconds` for the caller |
| Refresh | never on read; only by the refresh tool |

The [service compatibility limits](../../service/compatibility.md#7-limits-and-compatibility-obligations) own envelope accounting and admission. These new generic limits are not implemented legacy host capabilities. Catalog inventory reads use the ordinary read limits; only generic execution selects the generic ceilings.

## 6. Conformance scenarios (`docs/design.md:1006`)

- Catalog independence: configure a fixture service directly, discover its contracts, invoke a capability; then discover the same service through a catalog; admission outcome and invocation result are identical.
- Coverage truth: a bundle with one operation under `requires_implementation` → `catalog.operations.list` shows it `unresolved`; the descriptor omits it; invoking it yields the base unknown-operation error.
- Curation truth: an imported operation without `risk` → `unresolved`; adding the curation block makes it `generic`.
- Tamper: change one byte of a bundle file → `catalog.bundle.describe` returns `Unavailable` naming the file; the index generation is not served.
- Format refusal: an index with format version `v9` → `Unsupported` before any record is served.
- Credential-location refusal: a bundle/descriptor/index containing `{ "kind": "environment", "name": "ZENDESK_TOKEN" }` or a real custody/file locator → refused before publication or catalog disclosure, even though it contains no secret bytes. A safe auth-profile requirement and an unpopulated protected-slot schema are permitted; they grant no credential access.
- Read engine: fixture 200 with JSON → `body` structurally identical, `provenance.resource` carries parameter names not values; 429 with `Retry-After: 7` → `RateLimited`, `retry_after_seconds: 7`; 404 → `NotFound`; 500 → `Unavailable`; `text/html` body → `UpstreamProtocol`; body that exceeds the total response budget after metadata/envelope accounting → `UpstreamProtocol`; 302 → `UpstreamProtocol` and no second request.
- Engine discipline: an input containing a URL-shaped string in a path parameter is percent-encoded into its segment; the fixture observes exactly one request to the declared path.
- Paged: a mapping with `pagination.page` → `generic-http-page` returns `complete: true` when the fixture returns fewer items than the page size; the cursor is opaque and bound to the input digest.
- No network: run generation and every catalog operation with a network sentinel; zero connections (old `tests/main/no_network.rs` shape).
- Determinism: generate the same adapter twice with the pinned toolchain; manifests and index digest are identical.
- Public authentication metadata: serialize every catalog output and bundle file; neither the fixture's secret token nor its actual environment/custody/file locator may appear. Safe profile requirements remain distinguishable from private runtime bindings.

## 7. Compatibility

- New contract; new profile strings `generic-http` and `generic-http-page`.
- [Service compatibility](../../service/compatibility.md) owns the extended descriptor fields, singular profile, auth alternatives and generic limits. These require the proposed `v1alpha2` codec. Index, adapter-kind, bundle and configuration readers retain their own independent version decisions; a service-wire bump does not extend them.
- Old `catalog/<id>.catalog.json` documents and `catalog.pack` bytes are not preserved (`docs/design.md:911`). Old operation ids are preserved only where the migration contract selects them (`docs/design.md:1095`).
- The published `codewandler-connector-catalog-reader` crate has no successor in this design; its consumers need an explicit migration decision (`docs/design.md:151`).

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Specification kind: mapping vocabulary extended with any method, JSON body template with `$param` splices, header parameters, error envelope pointers, pagination, rate limit; curation block per operation; `upstream` optional for authored sources; Swagger 2.0 exact projection recorded as a transform | `spec-kinds/adapter/v2/schema.json` (`mappings` items today: `method`, `path`, `path_parameters`, `query_parameters`, `obligations`; `upstream` required), `crates/connectors-spec/src/` |
| Generator: default `prepare` and `finish` for `generic-http`; today both are required handwritten methods (`spec-kinds/adapter/v2/semantics.md:20-21`) | `crates/connectors-spec` |
| Bundle format: add `sources[]`, `curation`, `realization`, `license` to the manifest (`format` today `connectors.generated-bundle/v1`, fields `ess`, `files`, `rustfmt`, `specification_sha256`, `upstream_sha256`) | `adapters/*/generated/manifest.json` |
| Index tool: `catalog build`, `check`, `diff` over every `adapters/*/spec`; `sources refresh` as the only networked verb, never in `build.rs` | `crates/connectors-build/src/main.rs` (today: local executor only, line 1) |
| Generic engine: mapping + validated input + capability → request; response → outcome; the catalog adapter links it, no provider crate does | new crate under `crates/`, name in [adapters/catalog/design.md](../../../adapters/catalog/design.md) §10 |
| `AuthenticatedHttp` with method, body and headers; `http-header` and `http-query` credential placements (old declarations: 17 named headers, 2 query parameters) | `crates/connectors-sdk/src/lib.rs:53-56`; `contracts/auth/capability/v1alpha1/semantics.md` (reserved/unbound until the vocabulary and auth owners define support; no current descriptor may advertise it) |
| Client: typed `catalog.*` calls and bundle locate-and-verify | `crates/connectors-client` |
| Wire `v1alpha2` descriptor fields | `crates/connectors-core/src/lib.rs:70-78` |

## 9. ESS declarations and optional catalog state

The shared typed home for immutable artifact provenance is
[artifact_provenance.yaml](../../../ess/domains/artifact_provenance.yaml).
These records describe pinned declarations and bytes independently of any catalog
service. Existing `AdapterSpecification.adapter_id`, operation identities,
`UpstreamSource` and current strict readers retain their meanings. This model
does not make the proposed curation/source fields accepted by a current reader.

| Declaration | Selected identity and relation |
|---|---|
| Existing `AdapterSpecification` | Existing logical adapter id; references zero or more reusable Source records through `source_refs`; its operation ownership is unchanged |
| `Source` | Immutable record identified by an opaque `source_ref` for the exact `(url, revision)` tuple; origin, kind, original-byte SHA-256, retained license evidence and ordered transforms |
| `Bundle` | Immutable `digest` of the exact manifest bytes; references one logical AdapterSpecification plus its separately pinned specification SHA-256, and zero or more Source records |
| `Curation` | Optional embedded authored value on OperationDeclaration: expose, executable/semantic effects, risk, idempotency and realization. Absence remains unresolved for profiles requiring curation |
| `CatalogIndex` | Deferred adapter-owned publication selection. Its digest identifies one index generation; membership references reusable immutable Bundles and does not own their deletion |
| Tenancy, index publication authority and runtime retention/capacity | Deferred; no selected catalog service or publishing lifecycle is implemented or declared in shared ESS |

A Source key is an injective owner-maintained association with the exact URL and
revision, not an ambiguous concatenation or display label. A second observation
of that tuple with different pinned bytes or provenance is a conflicting source
record and refuses replacement; it requires an explicit reviewed source revision.
References never silently follow a moving source. Repository-authored inputs use
their recorded repository source and revision and do not invent vendor provenance.
The original-byte digest and each transform's ordered input/output digest remain
distinct. A transformed document cannot relabel its output as the original vendor
bytes. License evidence records retained source/license files where available;
an absent record is not a license grant or proof that no license exists.

Source reuse is many-to-many through the explicit AdapterSpecification/Bundle
reference lists: no adapter or bundle owns a Source's lifetime. The lists identify
a particular immutable declaration, not every historical revision under the same
logical adapter id. A Bundle's logical adapter reference alone cannot establish
its exact specification revision; its pinned specification digest and retained
bytes do. No existing identity is silently redefined to mean a revision key.

The Source lifecycle is the single immutable `Recorded` state and Bundle the
single immutable `Generated` state. These states classify retained declarations,
not whether a deployment installed them or an index currently selects them.
Multiple indexes or consumers may select the same Bundle digest. Replacing one
index retires that selection and does not mutate or delete the shared Bundle or
Source. The earlier proposed `generated → indexed → superseded` Bundle lifecycle
and `CatalogIndex owns Bundle` edge are superseded by this reuse rule. No
automatic deletion or cascade is selected by this model; later packaging/catalog
retirement must prove no remaining retained consumer or safety obligation.

Curation records author decisions; it does not discharge implementation
obligations or make unresolved work callable. Generic execution and new curation
or manifest syntax require their separately reviewed reader/schema binding.
Unknown/reserved realization, effect, risk or idempotency meanings refuse under
that binding; an optional ESS field is not an extension to a strict public
reader. The existing native operation profile/discriminator still selects
mutation versus read behavior.

ESS validates declared field types and explicit references. It does not enforce
tuple uniqueness, digest computation, source/license truth, manifest path safety,
cross-field curation completeness, current publication authority, file retention
or deterministic generation. Those remain named binding predicates; no storage,
catalog service, generator or public codec was implemented for this model. The
optional CatalogIndex publication lifecycle belongs under the catalog adapter
when selected, not in the shared provider-independent domain.

## 10. Open decisions

| Decision | Default taken |
|---|---|
| How a catalogued operation executes: (A) one generic executable loading bundles as data; (B) generated Rust crate per provider; (C) A for `generic-http`, B for operations bound to a semantic contract | C |
| Bundle transport | directory plus index first; `locations.kind = oci` reserved (`docs/design.md:805`) |
| Where the generic engine lives | in the catalog adapter, an ordinary provider (`docs/design.md:39`); not in the host |
| Swagger 2.0 | exact projection to OpenAPI 3.0 at ingest, recorded as a named transform |
| AsyncAPI sources (2 old, `specs/b10x.provenance.toml:73,95`) | recorded in `sources[]`, not ingested |
| Old operation-id compatibility | not preserved by default; selected per adapter at curation |
| Successor for the published reader crate | none; decision needed before any external consumer migrates |
