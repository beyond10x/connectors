# Pinned Opsgenie REST API source

`swagger.json` is the unmodified Opsgenie REST API Swagger 2.0 document from
`opsgenie/opsgenie-oas` at commit `71c55343d03076e5544c484ed32b37c3fbc54037`
(committed 2021-02-19), retrieved on 2026-10-14:

- [Exact source](https://raw.githubusercontent.com/opsgenie/opsgenie-oas/71c55343d03076e5544c484ed32b37c3fbc54037/swagger.json)
- SHA-256: `60901e17d0d4ba28b8d13d3d78f6a5d329b93008c3c1710dfe6a0b6c786ef071`
- Size: 659,535 bytes; Git blob `ca0c91ad0d9f3153f8288a6d70965fe6c9fc7086`, the
  blob the repository's tree names at that commit.
- Swagger `2.0`; `info.title` `Opsgenie REST API`; `info.version` `2.0.0`; host
  `api.opsgenie.com`, scheme `https`, no base path; 170 operations; one security
  scheme, `GenieKey` (`apiKey` in the `Authorization` header).
- [License at the same revision](https://raw.githubusercontent.com/opsgenie/opsgenie-oas/71c55343d03076e5544c484ed32b37c3fbc54037/LICENSE),
  Apache-2.0, retained as `LICENSE` (SHA-256
  `b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1`, Git blob
  `8dada3edaf50dbc082c9a125058f25def75e625a`). The document carries no
  `info.license` of its own; the licence is the repository's.

`opsgenie-source-hashes.json` records both files with their URL, commit, Git
blob, SHA-256 and length. The document is pinned as published. Gitleaks 8.30.0
reports no finding in it, and it carries no e-mail address and no `GenieKey`
value.

The catalog does not read Swagger 2.0 directly: `crates/connectors-catalog`
projects it to OpenAPI 3.1.0 under `swagger2-openapi/2` (rule table in
`src/swagger.rs`) into `../generated/opsgenie.openapi.json`, with its
projection record beside it. The document shares parameters and responses
through `#/parameters/` and `#/responses/` references, which the projection
inlines, and repeats 13 schemas of `definitions` under `parameters`, where no
operation references them; the record lists those as ignored.
Of its 170 operations, 99 project and 71 are listed unprojected in the record
with their pointers and reasons: 64 writes whose request body has no media type
(the document declares `consumes` on one operation only), the attachment
upload, whose `formData` file has no exact projection, and six policy
operations whose `teamId` query parameter is written with a `schema`. One of
those six names `listAlertPolicies`, which `GET /v1/policies` also names; that
one projects. The alert and incident reads the catalog ships all project.
`crates/connectors-catalog/tests/swagger.rs` asserts the digest above and
those counts and reasons.

The document lacks the incident API's
`GET /v1/incidents/{identifier}/associated-alert-ids`.
`opsgenie.amendments.json` adds it as one cited GET operation, bound to the
projection's SHA-256; the pinned bytes and their digest are unchanged. The
catalog provider's Opsgenie bundle and selection are in `adapters/catalog`
([guide](../../../docs/catalog-opsgenie.md)). Opsgenie does not endorse this
adapter. Refreshing the source means replacing this file, its record and the
digests the tests assert together.
