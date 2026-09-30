# Pinned Google Calendar API source

`calendar-api.json` is the unmodified Calendar API v3 Discovery document from
`googleapis/google-api-go-client` at commit
`ec13a0cd76ecc7fede8932d040a3156804515da9`, retrieved on 2026-09-30:

- [Exact source](https://raw.githubusercontent.com/googleapis/google-api-go-client/ec13a0cd76ecc7fede8932d040a3156804515da9/calendar/v3/calendar-api.json)
- SHA-256: `ae11869097c421ddd12e16b6cd4710ed7500379bbf669957b7014efb5b3a2698`
- Size: 169,809 bytes.
- `discoveryVersion` `v1`; `version` `v3`; Discovery `revision` `20260708`;
  `rootUrl` `https://www.googleapis.com/`, `servicePath` `calendar/v3/`.
- Licensed under that repository's BSD-3-Clause licence, `../LICENSE`.

The URL names a commit, so it serves these bytes for as long as the commit
exists; the digest above identifies the source either way.
`calendar-source-hashes.json` records the same digest and length for
`vendor/calendar-api.json.gz`, a gzip copy of the same bytes; the repository
gate's archived upstream source check re-derives both from that archive.
`crates/connectors-catalog/tests/discovery.rs` (`google_sources_pinned`) asserts
that this file, its archive, the manifest record and this README name one digest
and one size.

The document's operation paths are relative to `rootUrl` + `servicePath`; the
projection's one server is `https://www.googleapis.com/calendar/v3`. Refreshing
the source means replacing this file, its archive, both records and every bundle
derived from it together.
