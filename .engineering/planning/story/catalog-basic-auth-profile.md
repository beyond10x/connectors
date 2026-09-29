---
format: aep.planning-md/3
id: story:catalog-basic-auth-profile
kind: story
status: draft
title: The catalog provider authenticates with HTTP basic (identity plus API token)
relations:
- decomposes: epic:catalog-knowledge-sources
scope:
- confidence: cited
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 4
---
## Problem

The catalog provider's auth profile carries one token in one header
(`docs/local-catalog-provider.md`, Limits). Jira Cloud, Confluence Cloud and Zendesk API tokens
need HTTP basic: `Authorization: Basic base64(<account>:<token>)`.

## Surfaces

- `adapters/catalog/src/local.rs`: `AuthConfig` and the header construction beside `bearer`.
- `adapters/catalog/tests/local_runtime.rs` (and its `local_runtime/` modules): the fixture harness.
- `docs/local-catalog-provider.md`: the Limits section and the configuration reference.

## Acceptance

- A native catalog configuration can declare a basic profile, and the configuration loads.
- `connections connect` accepts the credential document `{"account": "...", "token": "..."}`; one
  test connects through `--credential-file` and one through `--credential-stdin`.
- A fixture-server test records the exact `Authorization` header of the identity read and of one
  operation, and asserts it equals `Basic base64(account:token)`.
- The same test connects with a wrong token and asserts the connect refusal code.
- A test with a profile whose `minimum_scopes` the fixture does not grant asserts the connect
  refusal code for insufficient scopes.
- A test reads the written configuration files, the provider child's argv and its environment
  (`/proc/<pid>/cmdline`, `/proc/<pid>/environ`) and asserts neither the account nor the token
  appears in any of them.
- The Limits section of `docs/local-catalog-provider.md` names the basic profile.
