# Pinned Runpod REST API source

`runpod-rest-v1.json` is the unmodified Runpod REST API OpenAPI document,
retrieved on 2026-10-08 from the URL the API serves it at:

- [Exact source](https://rest.runpod.io/v1/openapi.json)
- SHA-256: `9500a8989878d53d8731f27bf8dbbd57801b328c760bdb32c38ba36d5cb580db`
- Size: 154,609 bytes.
- OpenAPI `3.0.3`; `info.title` `Runpod API`; `info.version` `0.1.0`; server
  `https://rest.runpod.io/v1`; one security scheme, `ApiKey` (`http`, `bearer`).
- The document declares no licence and no terms of service; no licence text is
  vendored.

The publisher serves this URL unversioned, so the digest above, not the URL,
identifies the source; a second fetch on the same day returned the same bytes.

The document is pinned as served. Gitleaks 8.30.0 reports no finding in it.
`connectors-build redact` (rule `upstream-redaction/2`) would replace one value,
`info.contact.email`, which is the publisher's support address and not personal
data, so no redaction was applied.

`runpod-source-hashes.json` records the same digest and length for
`vendor/runpod-rest-v1.json.gz`, a gzip copy of the same bytes; the repository
gate's archived upstream source check (`connectors-build source-hashes`)
re-derives both from that archive. `adapters/catalog/tests/runpod.rs` asserts
that this file, the archive, the manifest record, this README and the committed
bundle index name one digest.

The document names `UpdatePod`, `UpdateEndpoint`, `UpdateNetworkVolume` and
`UpdateTemplate` twice each: once on `PATCH /<resource>/{id}` and once on
`POST /<resource>/{id}/update`. The inventory keeps all eight operations, and
the catalog engine refuses a selection that names one of these ids, because
the id alone does not say which request it means. None of them is selected.

The catalog provider compiles this document into
`../../catalog/generated/bundles/runpod.bundle.json`; the reviewed selection set
is `../../catalog/providers/runpod/operations.json`, described in
[the Runpod guide](../../../docs/catalog-runpod.md). Runpod does not endorse
this adapter. Refreshing the source means replacing this file, its archive, its
record and the bundle together.
