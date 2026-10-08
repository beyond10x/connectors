---
format: aep.planning-md/3
id: story:loki-credential-free-connection
kind: story
status: draft
title: Loki through a credential-free connection scheme (not planned)
relations:
- informed_by: decision-blocker:loki-connection-auth
revision: 1
---
## Status

Not planned. Recorded so the option is not lost; it needs an operator decision before any work.

## What

A credential-free connection scheme for Loki deployments that accept unauthenticated reads
(`loki.anonymous`). The local host today admits only a profile with at least one entry field and a
bearer, basic, mTLS or session scheme (`crates/connectors-host/src/local/runtime.rs:589-596`).
Admitting a credential-free profile widens that admission for every provider, so it would be
modelled in the shared authentication contracts and the ESS specification first, with the
configured connection as the identity source.

## Why not now

`decision-blocker:loki-connection-auth` chose the admitted `http_bearer` scheme with the
configured identity probe. No anonymous Loki deployment is in use.
