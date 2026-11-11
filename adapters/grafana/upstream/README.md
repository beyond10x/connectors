# Pinned Grafana HTTP API source

`openapi3.json` is the Grafana HTTP API OpenAPI 3 document from `grafana/grafana` at tag
`v13.3.1`, `public/openapi3.json`, retrieved on 2026-11-11, with one line removed (below):

- [Exact source](https://raw.githubusercontent.com/grafana/grafana/v13.3.1/public/openapi3.json)
- Upstream SHA-256: `89ff3e97e4d0e47ddd5a0bc757f2be2c909e655d275486a776a85fbde44dc661`
- Upstream size: 794,914 bytes.
- Committed SHA-256: `be8923b42413a640f92af54a728f5b04f936a2851a04b8dfa277ae0db3bdebdc`
- Committed size: 794,841 bytes.
- Removed member: `/components/schemas/NewApiKeyResult/properties/key/example` (JSON pointer),
  the one line `"example": "<token>",` above `"type": "string"`, so that schema property is
  exactly `{"type": "string"}`.
- OpenAPI `3.0.3`; `info.title` `Grafana HTTP API.`; `info.version` `0.0.1`; one server,
  `/api`; 314 operations; two security schemes, `basic` and `api_key`.
- [License at the same tag](https://raw.githubusercontent.com/grafana/grafana/v13.3.1/LICENSE),
  AGPL-3.0, retained as `LICENSE` (SHA-256
  `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`, 34,523 bytes). The
  document carries no `info.license` of its own; the licence is the repository's.

Why the line is removed: the member is Grafana's own example value of a service-account token, in
the schema of the answer that creates an API key. It has the shape of a real token, so the
repository's secrets scanner refuses the file. It is an example value only: it changes no
operation, parameter, type or response, and the bundle's inventory is the same with or without it;
only the source digest and size the bundle records differ. Nothing else in the file is changed.
`adapters/catalog/tests/grafana.rs` checks this without network: the committed file hashes to the
committed digest, this README records both digests and the pointer, the member is absent, and its
parent is exactly `{"type": "string"}`.

The document is otherwise pinned as published. Grafana's Swagger 2.0 document of the same tag,
`public/api-merged.json`, declares no `host`, which the catalog's Swagger projection refuses, so
the OpenAPI 3 document is the pinned source. The native `connectors-grafana` adapter does not
read this file: its OpenAPI document is authored in this repository ([design](../design.md)).

The catalog provider's Grafana bundle and selection are in `adapters/catalog`
([guide](../../../docs/catalog-grafana.md)). Grafana Labs does not endorse this adapter.
Refreshing the source means downloading the file at the new tag, removing the same member again,
and replacing this file and the digests the tests assert together
(`adapters/catalog/tests/grafana.rs`, `adapters/catalog/tests/bundle_drift.rs`).
