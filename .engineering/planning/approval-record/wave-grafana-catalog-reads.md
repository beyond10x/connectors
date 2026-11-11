---
format: aep.planning-md/3
id: approval-record:wave-grafana-catalog-reads
kind: approval-record
status: draft
title: Non-interactive wave readings for the Grafana catalog reads
tags:
- non-interactive
relations:
- decides: story:catalog-grafana-reads
revision: 1
---
Non-interactive wave, session 2c881569-7f3b-4a58-ae7a-367874f446ae, brief relayed by `b10x`.
No operator answered during the wave; the coordinator took the readings below. Each is
reversible inside the wave branch or a draft pull request.

## Readings taken

1. Source document. Grafana's Swagger 2.0 document (`public/api-merged.json`, v13.3.1) has no
   `host`; `crates/connectors-catalog/src/swagger.rs` refuses an absent `host`, and no amendment
   verb sets one. The wave pins Grafana's OpenAPI 3 document from the same tag instead:
   `public/openapi3.json`, 794,914 bytes, SHA-256
   `89ff3e97e4d0e47ddd5a0bc757f2be2c909e655d275486a776a85fbde44dc661`, `servers: [{url: /api}]`,
   vendored as `adapters/grafana/upstream/openapi3.json`. A scratch lowering of the definition over
   it gave 314 operations, 0 unsupported, no warning, and the five selected operations as GET.
2. `adapters/grafana/design.md` says the native adapter's OpenAPI document is repository-authored.
   The story states it changes nothing in that design; the catalog provider pins the upstream
   document and design.md is not edited.
3. Licence. The pinned document and its `LICENSE` are AGPL-3.0; both are vendored with a README
   naming tag, URL, digest and size, as the Opsgenie source is.
4. Paging. A declared paging style reaches the served selection as a bound only when it has both
   `limit` and `max`; `GET /api/search` declares no maximum. The paging acceptance is met by a
   test that walks two recorded fixture pages with caller-supplied `page` and `limit` and stops at
   the documented end condition (a page shorter than `limit`).
5. Publication. wave.md's grant names no push; the operator brief asks for draft pull requests.
   The brief is more specific: the wave branch and the definitions branch are pushed and opened
   as draft pull requests. Merge into the base branch, tags and releases are out of scope.

## Authorised commits

One commit per unit on `wave/grafana-catalog`, the closing store commit, and one commit in
`connectors-definitions` on its own branch. No base merge.

## Pre-flight

- Tree `wt-757820594599`, branch `wave/grafana-catalog`, HEAD `47166ae5d8`, clean.
- `/`: 44G free, 95% used.
- Session lease held by this session.
