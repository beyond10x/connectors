# Pinned Gmail API source

`gmail-api.json` is the unmodified Gmail API v1 Discovery document from
`googleapis/google-api-go-client` at commit
`ec13a0cd76ecc7fede8932d040a3156804515da9`, retrieved on 2026-09-30:

- [Exact source](https://raw.githubusercontent.com/googleapis/google-api-go-client/ec13a0cd76ecc7fede8932d040a3156804515da9/gmail/v1/gmail-api.json)
- SHA-256: `7591d33ec87e4ef83551f71630213a89ac349becebe9bd9b3e4ce3851dede0fe`
- Size: 217,686 bytes.
- `discoveryVersion` `v1`; `version` `v1`; Discovery `revision` `20260727`;
  `rootUrl` `https://gmail.googleapis.com/`, `servicePath` empty.
- Licensed under that repository's BSD-3-Clause licence, `../LICENSE`.

The URL names a commit, so it serves these bytes for as long as the commit
exists; the digest above identifies the source either way.
`gmail-source-hashes.json` records the same digest and length for
`vendor/gmail-api.json.gz`, a gzip copy of the same bytes; the repository gate's
archived upstream source check re-derives both from that archive.
`crates/connectors-catalog/tests/discovery.rs` (`google_sources_pinned`) asserts
that this file, its archive, the manifest record and this README name one digest
and one size.

The service path is empty, so each operation path carries its own
`gmail/v1/` prefix and the projection's one server is
`https://gmail.googleapis.com`. Refreshing the source means replacing this file,
its archive, both records and every bundle derived from it together.
