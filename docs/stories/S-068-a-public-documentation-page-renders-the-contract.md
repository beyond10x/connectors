---
id: S-068
title: "A public documentation page renders the contract"
pillar: Platform
status: done
epic: public-surface
areas: [server, docs]
depends_on: [S-067]
---

# A public documentation page renders the contract

## Goal

A person handed the connectors URL has nothing to read. `GET {base}/docs` renders the
public documentation: how to log in and mint a token, every endpoint with a working curl
example, the MCP entry point, datasources, and the refusal codes — one self-contained
HTML page, no external assets.

## Acceptance

- `GET {base}/docs` serves one self-contained HTML page, unauthenticated: zero external
  requests (no CDN scripts, no fonts, no images fetched elsewhere) — the connect
  completion page precedent.
- Content sections: authentication (identity login → access token mint with audience and
  scopes), the five envelope endpoints with request/response examples lifted from the
  S-067 document (single source of truth — examples are extracted, not duplicated), the
  MCP endpoint with an initialize/tools-call example, datasources incl. the read verbs,
  and the refusal code table.
- The page links `openapi.json` and renders its version; a drift test asserts every
  example shown on the page exists in the OpenAPI document.
- Static and tenant-free: the handler reads no state, takes no auth, and cannot render
  request-derived content beyond the base path.
- b10x-branded; check-brand stays clean.

## Progress

- 2026-08-24 — implemented on `impl/S-068`. `GET /docs` renders in `hosted/docs.rs` from the
  embedded S-067 artifact (`OnceLock` render, content-hash ETag, strict no-script CSP);
  `hosted.rs` gained only the route line. Every JSON block on the page carries a
  `data-example="<path> <kind> <name>"` marker, and the drift tests in `hosted/tests/docs.rs`
  trace each one back to the document's own example, ban foreign absolute URLs and every
  resource-fetching construct, and require the version, the `openapi.json` link, the audience,
  and every refusal-code enum. Auth section: identity login → `POST /v1/access-token`
  (audience + scopes extracted from the artifact's bearer description; mint request shape
  verified against beyond10x/identity `src/lib.rs` `AccessTokenRequest`).
