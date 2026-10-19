# Design: provider, connector, instance, connection

Status: design, 2026-10-16. Decision: `architecture-decision-record:connector-vocabulary`.
Plan: `epic:connector-vocabulary`. Not yet implemented.

## Decision

The words a user of the CLI, the configuration or the library meets are these, and no others:

| Term | Meaning | Google | GitLab |
|---|---|---|---|
| provider | The company or technology behind one or more connectors. | Google | GitLab |
| connector | One service or app of a provider, enabled in configuration. It points at one API. | Gmail, Drive, Calendar, Slides | GitLab |
| auth profile | One way to authenticate that a connector declares. | `google.oauth` | `gitlab.pat` |
| instance | The user's name for one configured connector: its endpoint, an optional description, and at most one connection. Names are free (`private`, `work`). An instance belongs to one connector and is never shared across connectors. | `gmail/private` | `gitlab/work` at a self-hosted host |
| connection | The stored credential of one instance, obtained through one auth profile. | the consent behind `gmail/private` | the PAT behind `gitlab/work` |
| operation | One callable tool of a connector. | `users.drafts.send` | `merge_request.merge` |

Two instances of one connector exist when the user connects twice, for example with two accounts or
two hosts; the instance name is what addresses the right one.

Every operation of an enabled connector is available on its instances. Connectors does not group
operations or let configuration hide a subset; a consumer library that wants a narrower surface
projects one over it. Writes keep the approval policy: a required-approval write still needs a
proof issued for its exact input.

"Adapter" leaves the user's vocabulary. It stays the internal name of an implementation (the catalog
engine, the Slack credential binary) and of the running process that serves an instance.

## Why

The vocabulary in use today gives one word several meanings and has none for what a user means by
"connect to Google":

| Today | Meaning | Where |
|---|---|---|
| adapter | the implementation of a contract (`catalog`) | `docs/design.md:138`; `adapter_id` in `crates/connectors-host/src/local/config.rs:270` |
| adapter | the configuration alias passed as `--adapter gmail-write` | `[adapters.<alias>]`, `config.rs:24` |
| adapter service | a running instance | `docs/design.md:140` |
| instance | a second name for the alias, `instance_id` | `config.rs:269` |
| provider | one API of a company (`google-gmail`), not the company | `provider` in `adapters/catalog/src/local.rs:173` |
| profile | an auth method (`google.oauth`) and a contract subset | `ess/domains/auth_bindings.yaml` `ProfileDeclaration`; `docs/design.md:137` |

The Google guides then configure two instances per API, a read instance and a write instance with a
wider scope, because a connection's scopes cannot be widened in place (the record of wave
`wave-20260930c-google-oauth-discovery`, README line 38, in history at commit `6498a881c`). Four
Google APIs with reads and writes are eight instances, eight consents and, while the OAuth app is
in *Testing*, eight weekly re-consents (`docs/catalog-google-oauth.md:113-120`).

## What changes

| Today | After |
|---|---|
| `[adapters.<alias>]` with `instance_id`, `adapter_id`, `configuration_revision`, `protocol`, `private_protocol`, `startup`, `restart`, `permissions`, `executable` (`config.rs:268-306`) | a connector enabled by name, and its instances by name with endpoint and description; everything the connector ships is derived |
| a native JSON file per instance repeating the auth profile's fixed fields (`docs/catalog-google-gmail.md` § Authentication) | the auth profile ships with the connector; an instance names it |
| `configuration_revision` copied by hand from `--print-local-bootstrap` | computed by the host |
| executable path and SHA-256 written by hand | resolved from the installed connector |
| `permissions.operations` listing every allowed operation | every operation of the connector |
| Gmail read instance (`gmail.readonly`) and write instance (`gmail.compose`) | one instance per account; its consent asks for every scope the connector's operations need |
| `connections repair` | `connections reconnect` |
| `--adapter <alias>` | `--connector <name> --instance <name>` |

A configuration sketch, not a schema:

```toml
[connectors.gmail]
enabled = true

[connectors.gmail.instances.private]
description = "personal mailbox"

[connectors.gitlab]
enabled = true

[connectors.gitlab.instances.work]
endpoint = "https://gitlab.example.com"
```

An instance without `endpoint` uses the connector's default endpoint. The auth profile is chosen at
connect time when a connector declares more than one.

## Connection lifecycle

`connectors.auth_bindings.Connection` has the states `Live` and `Revoked`, and `Live` says nothing
about whether the credential still works (`ess/domains/auth_bindings.yaml:145-149`). A credential
the provider refuses shows only as an invoke failure, `service_failure` with
`next_action: repair_connection` (`docs/catalog-google-oauth.md` § Repair).

The connection gains a state for it:

| From | To | Cause |
|---|---|---|
| Live | Stale | the provider refused the stored credential: an expired or revoked refresh token, a password change, a revoked token |
| Stale | Live | a reconnect published a new credential for the same instance and connection |
| Live | Live | a reconnect or revalidation published a new credential |
| Live, Stale | Revoked | the user revoked the connection |

Refreshing an access token is not a state change: the provider exchanges the stored refresh token
when its cached access token is within 60 s of `expires_in`, capped at 24 h
(`adapters/catalog/src/local.rs:366-368`, `:515`). Only a refusal of the refresh token itself makes a
connection Stale; no library call can renew it without the person consenting again.

The validation-evidence expiry that makes the CLI answer `pending` after `evidence_lifetime_ms`
(`docs/local-catalog-provider.md:822`, `:903`) is a second "not ready" condition and is not a
connection state. Whether it stays visible to users is an open question below.

## Consequences

- One Google consent per instance. `gmail/private` and `drive/private` are two consents.
- Google's 7-day refresh-token expiry applies to an OAuth app in *Testing* whatever the vocabulary
  (developers.google.com/identity/protocols/oauth2). An app used only by its developer may be set to
  *In production* without verification ("personal use (fewer than 100 users)",
  support.google.com/cloud/answer/13464323), which lifts the expiry. `docs/catalog-google-oauth.md:117-118`
  says publishing is "subject to Google's verification for sensitive scopes" and is corrected with
  this change.
- Renaming public names needs an explicit migration decision (`docs/design.md:152`). The CLI flags,
  the configuration file and the CLI's ESS domain (`ess/domains/cli.yaml`, field `adapter`) change
  together, with a migration of existing configuration files.

## Out of scope

- Operation groups or per-instance operation filtering. Consumer libraries project.
- One consent shared by instances of different connectors of one provider.

## Open questions

- Whether the validation-evidence `pending` condition stays user-visible, or the host revalidates
  on its own before an invoke.
- How an existing `[adapters.<alias>]` entry migrates: renamed in place by a command, or rewritten
  by the user from a printed diff.
- Whether `provider` appears in configuration at all, or only in the catalog that lists connectors.
