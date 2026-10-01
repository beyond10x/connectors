# Pinned Google Discovery sources

Four Google API Discovery documents (`discovery#restDescription`,
`discoveryVersion` `v1`), each copied unmodified from
[`googleapis/google-api-go-client`](https://github.com/googleapis/google-api-go-client)
at commit `ec13a0cd76ecc7fede8932d040a3156804515da9`, retrieved on 2026-09-30:

| Directory | Document | Discovery `revision` |
|---|---|---|
| [`calendar/`](calendar/README.md) | Calendar API v3 | `20260708` |
| [`drive/`](drive/README.md) | Google Drive API v3 | `20260916` |
| [`gmail/`](gmail/README.md) | Gmail API v1 | `20260727` |
| [`slides/`](slides/README.md) | Google Slides API v1 | `20260921` |

`LICENSE` is that repository's own licence file at the same commit
(BSD-3-Clause), copied unmodified:

- [Exact source](https://raw.githubusercontent.com/googleapis/google-api-go-client/ec13a0cd76ecc7fede8932d040a3156804515da9/LICENSE)
- SHA-256: `110244b02140866ee37d17fa7449436a377ec3b85a481fbb208f4c87964382de`
- Size: 1,475 bytes.

A Discovery document is not an OpenAPI document. `connectors-build discovery`
projects one into OpenAPI 3.0.3 under the rule table of
`crates/connectors-catalog/src/discovery.rs`, and `connectors-build catalog
--derived-from` records the projection in the bundle it builds. The provider
stories consume these pins and do not re-pin them. Google does not endorse this
adapter. Refreshing a source means replacing its file, its archive, both records
and every bundle derived from it together.
