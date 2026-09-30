# Pinned Google Slides API source

`slides-api.json` is the unmodified Google Slides API v1 Discovery document from
`googleapis/google-api-go-client` at commit
`ec13a0cd76ecc7fede8932d040a3156804515da9`, retrieved on 2026-09-30:

- [Exact source](https://raw.githubusercontent.com/googleapis/google-api-go-client/ec13a0cd76ecc7fede8932d040a3156804515da9/slides/v1/slides-api.json)
- SHA-256: `49fa379e29948696029ec8c4b334939f0bd6e74d9ef27650f523682283c1d3f1`
- Size: 252,866 bytes.
- `discoveryVersion` `v1`; `version` `v1`; Discovery `revision` `20260921`;
  `rootUrl` `https://slides.googleapis.com/`, `servicePath` empty.
- Licensed under that repository's BSD-3-Clause licence, `../LICENSE`.

The URL names a commit, so it serves these bytes for as long as the commit
exists; the digest above identifies the source either way.
`slides-source-hashes.json` records the same digest and length for
`vendor/slides-api.json.gz`, a gzip copy of the same bytes; the repository gate's
archived upstream source check re-derives both from that archive.
`crates/connectors-catalog/tests/discovery.rs` (`google_sources_pinned`) asserts
that this file, its archive, the manifest record and this README name one digest
and one size.

The service path is empty, so each operation path carries its own `v1/` prefix
and the projection's one server is `https://slides.googleapis.com`. One path,
`v1/presentations/{+presentationId}`, uses reserved expansion; the projection
writes it as `{presentationId}` and lists the rewrite in its record. Refreshing
the source means replacing this file, its archive, both records and every bundle
derived from it together.
