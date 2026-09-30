# Pinned Jira Cloud platform source

`jira-platform-v3.json` is the unmodified official Jira Cloud platform REST API
v3 OpenAPI document, retrieved on 2026-09-29:

- [Exact source](https://developer.atlassian.com/cloud/jira/platform/swagger-v3.v3.json)
- SHA-256: `655b4790a5c8543c81755c8250c6a8b8f58ee62712168a39d628d6d927549709`
- Size: 2,478,466 bytes.
- OpenAPI `3.0.1`; `info.version` `1001.0.0-SNAPSHOT-dbfd2235e09199a2fb4752c2cbd5853d9102be9a`.
- The document declares its license as Apache 2.0
  (`info.license`, <http://www.apache.org/licenses/LICENSE-2.0.html>).

The publisher serves this URL unversioned, so the digest above, not the URL,
identifies the source. `jira-source-hashes.json` records the same digest and
length for `vendor/jira-platform-v3.json.gz`, a gzip copy of the same bytes; the
repository gate's archived upstream source check re-derives both from that
archive. `adapters/catalog/tests/jira.rs` asserts that this file, the manifest
record, this README and the committed bundle index name one digest.

The catalog provider compiles this document into
`../../catalog/generated/bundles/jira.bundle.json`; the reviewed read-only
selection set is `../../catalog/providers/jira/operations.json`, described in
[the Jira guide](../../../docs/catalog-jira.md). Jira and Confluence
([`confluence/`](confluence/README.md)) share the `atlassian.basic`
authentication profile. Atlassian does not endorse this
adapter. Refreshing the source means replacing this file, its archive, both
records and the bundle together.
