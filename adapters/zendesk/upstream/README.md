# Pinned Zendesk Support source

`zendesk-support.yaml` is the Zendesk Support API OpenAPI document, retrieved on
2026-10-05 from the "Download OpenAPI file" link of Zendesk's
[Support API reference](https://developer.zendesk.com/api-reference/ticketing/introduction/),
with example credentials and personal data redacted before it was committed:

- [Exact source](https://developer.zendesk.com/zendesk/oas.yaml)
- Upstream SHA-256: `3a477ea89b274f4d3731f1c7ff93dc93d4520de871ac759b06d3297798fd685d`,
  1,932,113 bytes. The upstream bytes are not committed.
- Redaction rule: `upstream-redaction/2`, applied by
  `cargo run --locked -p connectors-build -- redact --input <upstream file> --output zendesk-support.yaml`
  (`crates/connectors-build/src/upstream_redaction.rs`). The value of every key whose
  name ends in `token`, `secret`, `password`, `passwd`, `api_key`, `apikey`,
  `access_key`, `private_key` or `credential` becomes `example-redacted-token` when it
  is one scalar without whitespace (a nested mapping, a block, `null` or a boolean is
  left alone), and so does every JSON Web Token (`eyJ….eyJ…`). Every email address whose
  domain is not reserved for documentation (`example.com`, `example.net`,
  `example.org` and their subdomains, or a name under `.example`, `.test`,
  `.invalid` or `.localhost`) becomes `user@example.com`; every phone number written
  as `+` and 8 to 15 digits, or as `+<country> <3>-<3>-<4>`, becomes
  `+15555550100` or `+1 555-555-0100`, unless it is a North American number in the
  unassigned area code 555 or the fictional exchange 555-01XX. On this document it
  replaced 40 credential values, 74 email addresses and 5 phone numbers. Gitleaks
  8.30.1 reports no finding in the result.
- Redacted SHA-256: `5dd6cf1eda8febf02f8fb2f2cfa72d7cb6bc7d5bab188ba97b49ab55fc51dbf2`,
  1,931,767 bytes.
- OpenAPI `3.0.3`; `info.title` `Support API`; `info.version` `2.0.0`; server
  `https://{subdomain}.{domain}.com`, with `domain` defaulting to `zendesk`.
- The document declares no licence and no terms of service; no licence text
  is vendored.

The publisher serves this URL unversioned, so the upstream digest above, not the
URL, identifies the source. Reproducing the committed file means fetching the URL,
checking the upstream digest and running the redaction command.

`zendesk-source-hashes.json` records the URL, both digests and lengths, and the
rule. The repository gate (`connectors-build gate`, and `connectors-build
source-hashes`) checks that the committed file has the redacted digest and length
and that the rule would replace nothing more in it. `adapters/catalog/tests/zendesk.rs`
asserts that this file, the manifest record, this README and the committed bundle
index name one digest.

`zendesk-support.amendments.json` adds one parameter the document leaves out:
`per_page` on `IncrementalTicketExportCursor`, which Zendesk's
[incremental exports reference](https://developer.zendesk.com/api-reference/ticketing/ticket-management/incremental_exports/)
documents (up to 1,000, default 1,000). It is bound to the redacted SHA-256 above
and applied to the inventory when the bundle is built (`--amendments`); this
file's bytes are unchanged by it.

The catalog provider compiles this document into
`../../catalog/generated/bundles/zendesk.bundle.json`; the reviewed read-only
selection set is `../../catalog/providers/zendesk/operations.json`, described in
[the Zendesk guide](../../../docs/catalog-zendesk.md). Zendesk does not endorse
this adapter. Refreshing the source means replacing this file, its record and the
bundle together.
