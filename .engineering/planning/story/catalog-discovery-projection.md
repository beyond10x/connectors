---
format: aep.planning-md/3
id: story:catalog-discovery-projection
kind: story
status: implemented
title: Project a Google Discovery document into OpenAPI exactly, and record the derivation
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: adapters/google/upstream
- confidence: inferred
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: crates/connectors-catalog/Cargo.toml
- confidence: inferred
  path: crates/connectors-catalog/src/discovery.rs
- confidence: inferred
  path: crates/connectors-catalog/src/lib.rs
- confidence: inferred
  path: crates/connectors-catalog/src/pipeline.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/discovery
- confidence: inferred
  path: crates/connectors-catalog/tests/discovery.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:14:01Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-09-30T13:14:02Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:31Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":10,"verification":1}}}
---
## Outcome

`connectors-build discovery --source <discovery.json> --out <openapi.json>` projects a Google
Discovery document (`discovery#restDescription`, `discoveryVersion: v1`) into an OpenAPI 3.0.3
document that the existing ingest (`crates/connectors-catalog/src/lib.rs:128-146`) accepts, and the
bundle records the derivation. The projection is exact: every Discovery method becomes one
operation or one named exclusion, and every construct outside the rule table is refused by name.

## Sources (this story pins all four)

`googleapis/google-api-go-client` at `ec13a0cd76ecc7fede8932d040a3156804515da9`, BSD-3-Clause:
`drive/v3/drive-api.json`, `slides/v1/slides-api.json`, `calendar/v3/calendar-api.json`,
`gmail/v1/gmail-api.json`, pinned into `adapters/google/upstream/<api>/` with a `vendor/*.json.gz`
copy, `<api>-source-hashes.json`, a README (URL at the commit, SHA-256, size, Discovery `revision`)
and that repository's LICENSE at `adapters/google/upstream/LICENSE`, in the layout of
`adapters/atlassian/upstream/`. The source-hash gate (`crates/connectors-build/src/source_hashes.rs`)
covers them. The provider stories consume these pins and do not re-pin them.

## Design

- New `crates/connectors-catalog/src/discovery.rs`: `project(bytes) -> Result<Projection, Refusal>`
  where `Projection { openapi: Vec<u8>, record: ProjectionRecord }`. Pure, no clock, no network;
  output is canonical JSON with sorted keys, so two runs give identical bytes.
- `SourceRecord` (`lib.rs:77-90`) gains `derivation: Option<Derivation>` with
  `skip_serializing_if = "Option::is_none"`: `{ from_file, from_sha256, from_bytes, format:
  "google-discovery/v1", discovery_revision, projector: "discovery-openapi/1" }`. Existing bundles
  keep their bytes (`adapters/catalog/tests/bundle_drift.rs` proves it).
- `connectors-build catalog` gains `--derived-from <discovery.json>`, which recomputes the
  projection, refuses unless it equals `--source` byte for byte, and fills `derivation`
  (`crates/connectors-build/src/main.rs:76-90,168-186`). The subcommand `discovery` writes the
  projected document plus `<name>.projection.json` (the `ProjectionRecord`).
- `ProjectionRecord`: Discovery method count, projected operation ids, excluded methods with reason,
  rewritten `{+x}` paths, excluded global parameters.

## Rule table

| Discovery | OpenAPI 3.0.3 |
|---|---|
| `rootUrl` + `servicePath` | one `servers[].url` (absolute https; anything else refused) |
| method `id` | `operationId`, verbatim (e.g. `slides.presentations.get`) |
| `path` | the path under the server; `{+x}` becomes `{x}` and is listed in the record (narrowing: the template escapes `/`, `crates/connectors-catalog/src/template.rs:626`) |
| `httpMethod` | the operation's method (GET, POST, PUT, PATCH, DELETE; anything else refused) |
| `parameters` with `location` `path` / `query` | `in: path` / `in: query`; path parameters `required: true` |
| parameter `required`, `type`, `format`, `enum`, `default`, `pattern`, `minimum`, `maximum`, `description`, `deprecated` | parameter + `schema`, one to one (`minimum`/`maximum` are strings in Discovery and become numbers; a non-numeric value is refused) |
| `repeated: true` | `schema: {type: array, items: …}`, `style: form`, `explode: true` |
| `request.$ref` | `requestBody` `application/json` → `#/components/schemas/<name>`, `required: true` |
| `response.$ref` | `200` `application/json` → `#/components/schemas/<name>` |
| no `response` | `200` with no content, or `application/octet-stream` when `supportsMediaDownload` |
| `supportsMediaUpload` / `mediaUpload` | the metadata path is projected as above; upload protocol paths (`/upload/…`, `/resumable/upload/…`) are excluded and listed |
| `useMediaDownloadService`, `supportsSubscription`, `parameterOrder`, `flatPath` | not needed by OpenAPI; listed in the record as ignored keys |
| `scopes` | `x-google-scopes` on the operation; no `securitySchemes` |
| document `parameters` | `fields` projected on every operation; `access_token`, `oauth_token`, `key`, `alt`, `callback`, `$.xgafv`, `uploadType`, `upload_protocol`, `prettyPrint`, `quotaUser`, `userIp` excluded and listed |
| `schemas` | `components.schemas`: `type` `any` → `{}`; `integer`/`number` with `format`; `string` formats `int64`/`uint64` → `type: string` + `format`; `google-datetime` → `date-time`; `date`, `date-time`, `byte` kept; `google-fieldmask`, `google-duration` → `string` + `x-google-format`; `properties`, `items`, `additionalProperties`, `enum`, `readOnly`, `deprecated`, `description` kept; `annotations.required` → `x-google-required-for` |
| `$ref` inside schemas | `#/components/schemas/<name>`; an unresolved name is refused |
| nested `resources` | walked recursively; method ids are already fully qualified |
| any key or value not in this table | refused, naming the JSON pointer |

## Acceptance

- Golden fixtures in `crates/connectors-catalog/tests/discovery/`, one small Discovery document per
  table row, each with its expected OpenAPI output; test `discovery_rule_<row>` per row.
- `discovery_refuses_unknown_key`, `discovery_refuses_unresolved_ref`,
  `discovery_refuses_non_https_root`: refusal names the JSON pointer.
- `google_sources_pinned`: for each of the four APIs, the pinned file, its `vendor/*.json.gz`, its
  `*-source-hashes.json` and its README name one SHA-256 and one size, and
  `adapters/google/upstream/LICENSE` exists (the pattern of `adapters/catalog/tests/confluence.rs:127-170`).
- Over the four pinned documents under `adapters/google/upstream/` (no second copy):
  - `discovery_conserves_methods`: projected operations + excluded methods == Discovery method
    count, per document.
  - `discovery_refs_resolve`: every `$ref` in the output resolves.
  - `discovery_no_credential_parameters`: no parameter named `access_token`, `oauth_token` or
    `key` in any output.
  - `discovery_is_deterministic`: two projections are byte-identical.
  - `discovery_openapi_parses_independently`: each output deserializes with the `openapiv3` crate
    (new dev-dependency).
  - `discovery_ingests`: `ingest` + `extract` inventory every projected operation, and the
    inventory's `Unsupported` list is empty.
- `catalog_records_derivation`: `connectors-build catalog --derived-from <pinned drive-api.json>
  --source <its projection>` writes a bundle whose `source.derivation.from_sha256` equals the pinned
  file's SHA-256 and whose `projector` is `discovery-openapi/1`.
- `catalog_derived_from_mismatch_refused`: the same command with a `--source` that differs from the
  projection by one byte is refused and writes no bundle.
- `derivation` absent from every existing bundle: `bundle_drift.rs` reproduces the GitLab, Jira and
  Confluence bundles byte for byte.
- An `aep:adversary` pass on the projector before merge, recorded by the coordinator as a
  `review-result` that `reviews` this story, with every finding given a `review_outcome`.

Array query parameters ingest as ordinary query parameters until
`story:catalog-repeated-query-parameters` lands, so `discovery_ingests` holds without it and this
story does not depend on it.

## Out of scope

Discovery documents other than `discoveryVersion: v1`; fetching Discovery at build time (sources are
pinned files); media upload.
