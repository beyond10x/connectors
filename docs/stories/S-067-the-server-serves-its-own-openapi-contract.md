---
id: S-067
title: "The server serves its own OpenAPI contract"
pillar: Platform
status: done
epic: public-surface
areas: [server, docs]
---

# The server serves its own OpenAPI contract

## Goal

Consumers of the hosted API — the CLI, the harness, external teams — have no
machine-readable contract to generate against or validate with. The server knows its
surface precisely (five strict envelope endpoints, the MCP transport, health probes), and
the repo already pins contracts as committed artifacts; the OpenAPI document joins that
family.

## Acceptance

- `GET {base}/openapi.json` serves a committed OpenAPI 3.1 document, unauthenticated,
  `application/json`, immutable per build (ETag or content hash header).
- The document covers: `/operations`, `/connections`, `/catalog`, `/events`,
  `/datasources` (envelope request/response schemas incl. the closed method sets and
  refusal codes), `/mcp` (JSON-RPC shape, the three meta-tools), the health probes, and
  the bearer security scheme naming the identity audience and scopes.
- Drift is impossible silently: a test suite validates every example in the document
  against the real `protocol` serde types — each request example must deserialize
  (deny_unknown_fields), each refusal example must match a real error shape, and a
  deliberately wrong example must fail the suite.
- No new dependency; the document is authored as a file and served verbatim from a new
  `crates/server/src/hosted/docs.rs`, with only route registration touching `hosted.rs`.
- The document is b10x-branded; no b10x string at the surface (check-brand stays
  clean).

## Notes

- utoipa/schemars were considered and rejected: new dependencies plus derive annotations
  across the protocol crate, to document Rust types rather than the wire contract the
  envelope validation actually enforces. The committed-artifact + example-validation
  pattern is the same discipline as the golden vectors and spec hashes.

## Progress

- 2026-08-24, implemented on `impl/S-067`. Every acceptance item is satisfied:
  - The committed artifact is `crates/server/src/hosted/docs/openapi.json`, derived once
    from the frozen wire schemas under `contracts/connector-*/v0alpha1/` (their `$defs`
    inlined as OpenAPI components, `$refs` rewritten) and patched with the S-049
    `session_signal` method/result variant, which postdates the v0alpha1 freeze. It is
    governed like every owned JSON document: registered in `json-schemas.toml` against the
    scoped structural schema `crates/server/src/hosted/docs/openapi.schema.json`.
  - `GET {base}/openapi.json` is served by `crates/server/src/hosted/docs.rs`
    (`include_str!`, unauthenticated, `application/json`, quoted-SHA-256 `ETag`); only
    `mod docs;` and one route line touched `hosted.rs`.
  - Drift tests: `crates/server/src/hosted/tests/docs.rs` — every request example
    deserializes with the exact protocol envelope type **and** passes its `validate()`;
    every refusal example deserializes into the closed error enums (catalog's open string
    code is pinned to the four codes the hosted route produces); the operation examples must
    keep covering not_granted / approval_required / invalid_input / result_too_large /
    unavailable / stale_authority; an unknown field at envelope and params depth is proven
    refused; every documented route answers non-404 from the real router (with a 404
    control); the MCP request examples are answered by the live transport; the five
    `protocol` consts and the identity audience + six scopes are pinned.
  - No new dependency (`sha2` and `serde_json` were already server dependencies);
    check-brand stays clean — the only b10x strings at the surface are the registered
    wire ids (`b10x.connector-*.v0alpha1`, `urn:b10x:connectors`).
- Not done, deliberately: `If-None-Match`/304 handling (acceptance asks for the immutable
  content-hash header only) and the connect-session/oauth browser routes stay outside the
  machine contract.

## Superseded by

`story:the-server-serves-its-own-openapi-contract` in the AEP planning store, at
`.engineering/planning/story/the-server-serves-its-own-openapi-contract.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
