# Pinned Tavily API source

`tavily-openapi.json` is the unmodified Tavily API OpenAPI document, retrieved on 2026-10-05:

- [Exact source](https://docs.tavily.com/documentation/api-reference/openapi.json)
- SHA-256: `b84132a5df9fa476cc38aac79083a28085ea2356e379af34f323975d640b91db`
- Size: 156,727 bytes.
- OpenAPI `3.0.3`; `info.title` `Tavily Search and Extract API`, `info.version` `1.0.0`; server
  `https://api.tavily.com/`; security scheme `bearerAuth` (HTTP bearer).
- The document declares no licence and no terms of service; no licence text is vendored.

`tavily-source-hashes.json` records the same digest and length for
`vendor/tavily-openapi.json.gz`, a gzip copy of the same bytes; the repository gate's archived
upstream source check re-derives both from that archive.

The adapter does not compile this document; it is the reviewed source of the mapping in
[the profile](../contracts/websearch/v1alpha1/semantics.md). Tavily does not endorse this adapter.
Refreshing the source means replacing this file, its archive, the record and the profile's pinned
digest together, and reviewing the mapping against the new document.
