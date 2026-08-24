---
id: S-067
title: "The server serves its own OpenAPI contract"
pillar: Platform
status: ready
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
