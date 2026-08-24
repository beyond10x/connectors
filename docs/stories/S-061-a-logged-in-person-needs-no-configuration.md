---
id: S-061
title: "A logged-in person needs no configuration"
pillar: Platform
status: backlog
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [config, integrations]
---

# A logged-in person needs no configuration

## Goal

Today a person needs curl and token plumbing to use the hosted plane. Give the local
connectors CLI a zero-config hosted mode: `connectors login` drives the identity loopback
flow (the login metadata already publishes the hosted endpoint), the session lands in the OS
keyring, and every subsequent CLI command — and the stdio MCP bridge (S-056 folds in here) —
exchanges it for short-lived scoped tokens transparently. The person types their Google
password once and then uses discovered databases, Kubernetes, and monitoring through CLI or
MCP with nothing configured locally.

## Acceptance

- On a machine with no prior state: `connectors login <origin>` completes the browser flow,
  and a following catalog/search/invoke command against the hosted deployment succeeds with
  no flags beyond the origin (which discovery may also supply); tokens refresh across the
  300-second expiry without re-login, proven by an integration test with a fake identity.
- The thin-CLI architecture fence stays green — the behaviour lands behind connectors-client
  per its prescription.

## Progress

- 2026-08-24 — filed from design 15 (Timo: "users, when logged in with Google SSO via us,
  don't have to configure any of these things — they can just use it").
