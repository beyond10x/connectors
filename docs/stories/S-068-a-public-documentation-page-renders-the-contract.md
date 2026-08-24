---
id: S-068
title: "A public documentation page renders the contract"
pillar: Platform
status: ready
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
