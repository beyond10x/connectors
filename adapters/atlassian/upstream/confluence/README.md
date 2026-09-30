# Pinned Confluence Cloud source

`confluence-v2.json` is the unmodified official Confluence Cloud REST API v2
OpenAPI document, retrieved on 2026-09-30:

- [Exact source](https://developer.atlassian.com/cloud/confluence/openapi-v2.v3.json)
- SHA-256: `edb639bbc700ee451a996acd2568e51db4ceab954449427537df30f0ce20ca08`
- Size: 614,311 bytes.
- OpenAPI `3.0.3`; `info.version` `2.0.0`; server `https://{your-domain}/wiki/api/v2`.
- The document declares no license. It declares terms of service only
  (`info.termsOfService`,
  <https://developer.atlassian.com/platform/marketplace/atlassian-developer-terms/>);
  no licence text is vendored.

The publisher serves this URL unversioned, so the digest above, not the URL,
identifies the source. `confluence-source-hashes.json` records the same digest
and length for `vendor/confluence-v2.json.gz`, a gzip copy of the same bytes; the
repository gate's archived upstream source check re-derives both from that
archive. `adapters/catalog/tests/confluence.rs` asserts that this file, the
manifest record, this README and the committed bundle index name one digest.

The document's operation paths are relative to its server path `/wiki/api/v2`;
the bundle records each one below it (`/wiki/api/v2/pages`), which is the path a
request sends.

The catalog provider compiles this document into
`../../../catalog/generated/bundles/confluence.bundle.json`; the reviewed
read-only selection set is `../../../catalog/providers/confluence/operations.json`,
described in [the Confluence guide](../../../../docs/catalog-confluence.md). Jira
and Confluence share the `atlassian.basic` authentication profile. Atlassian does
not endorse this adapter. Refreshing the source means replacing this file, its
archive, both records and the bundle together.
