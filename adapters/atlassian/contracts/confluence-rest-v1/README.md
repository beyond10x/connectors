# Confluence Cloud REST v1 reads, authored here

`confluence-rest-v1.yaml` is an OpenAPI 3.1 document this repository writes itself. It
describes the Confluence Cloud REST v1 reads the catalog provider serves under the provider id
`confluence-v1`, and nothing else.

- **Why it is authored.** Atlassian's REST v1 OpenAPI document declares no licence, so it is
  not vendored, copied or fetched, at build time or at any other time. The REST v2 document
  ([`../../upstream/confluence/`](../../upstream/confluence/README.md)) is pinned as published
  and has no CQL search; a token that carries only classic scopes reads Confluence through v1
  alone.
- **How it was written.** From Atlassian's public REST v1 reference pages, in this
  repository's own words and structure. Each operation names the reference page it was
  written from in `externalDocs.url`. It holds the paths, the query parameters and the
  response fields the shipped reads rely on: paging (`start`, `limit`, `cursor`, `_links.next`),
  item identity (`id`, `key`, `title`), `version.when` and the storage body. Operation ids
  follow the reference's names so a reader can find the page; descriptions and schemas are
  this repository's.
- **What it is not.** It is not Atlassian's document and Atlassian does not endorse it. It
  carries this repository's licence (`info.license`). A read the reference documents and this
  file leaves out is not served.

The catalog pipeline compiles it into
[`../../../catalog/generated/bundles/confluence-v1.bundle.json`](../../../catalog/generated/bundles/confluence-v1.bundle.json).
Its server path is the site's `/wiki` root, so each path is recorded below it
(`/rest/api/search` as `/wiki/rest/api/search`). The selection set is
[`../../../catalog/providers/confluence-v1/operations.json`](../../../catalog/providers/confluence-v1/operations.json),
described in [the Confluence guide](../../../../docs/catalog-confluence.md#rest-v1). The bundle
is rebuilt with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider confluence-v1 \
  --source adapters/atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile atlassian.basic --replace
```

`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle a fresh run over this file
would not reproduce byte for byte, and `adapters/catalog/tests/confluence_v1.rs` pins each
selection's operation id and path, checks every operation names an `https` reference page on
`developer.atlassian.com`, and runs each read against a recorded fixture of the v1 response
shape. Changing a read means changing this file, the selection set and the bundle together.
