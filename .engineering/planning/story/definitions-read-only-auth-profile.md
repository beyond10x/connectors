---
format: aep.planning-md/3
id: story:definitions-read-only-auth-profile
kind: story
status: draft
title: The definitions format declares a read-only auth profile
relations:
- informed_by: story:google-read-only-profiles-and-company-domain
revision: 1
---
## Gap

story:google-read-only-profiles-and-company-domain gave Gmail, Calendar and Drive a read-only
default profile in the engine's built-in providers
(`adapters/catalog/providers/google-*/connector.json`, `default_profile` and a `read_only`
profile). The definitions format cannot say the same: its Auth type has no `read_only` and no
per-profile operation set (connectors-definitions `spec/domains/definition.yaml:55-70`, `:213-240`,
`:348`), and the engine's definition reader rejects such a field (`crates/connectors-catalog/src/definition.rs:232`,
`:793`). An instance installed from connectors-definitions therefore has no read-only profile; a
profile marked only `default: true` would offer the writes on a read-only scope.

## Acceptance

- The definitions format and the engine's definition reader accept `read_only: true` on an auth
  profile; under it every write operation is withheld, as for the built-in providers.
- connectors-definitions `google-gmail.yaml`, `google-calendar.yaml` and `google-drive.yaml` declare
  `google.oauth.readonly` as the default read-only profile, and an instance installed from them
  offers no write under it (test in the engine against the definitions fixture).
