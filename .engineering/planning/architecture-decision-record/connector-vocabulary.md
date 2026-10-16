---
format: aep.planning-md/3
id: architecture-decision-record:connector-vocabulary
kind: architecture-decision-record
status: accepted
title: Users configure connectors and named instances; adapter is an internal term; a refused credential makes a connection Stale
relations:
- informed_by: epic:google-workspace-reads
- decides: epic:connector-vocabulary
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-16T11:47:01Z", actor: "human:timo", revision: 2}
---
## Context

The CLI, the configuration and the guides use "adapter" for an implementation (`adapter_id =
"catalog"`), for the configuration alias passed as `--adapter`, and for a running service
(`docs/design.md:138-140`); "instance" for the same alias (`instance_id`,
`crates/connectors-host/src/local/config.rs:269`); and "provider" for one API of a company
(`google-gmail`, `adapters/catalog/src/local.rs:173`). None of them names what a user means by
"connect to Google" or "connect to GitLab".

A connection cannot gain a scope in place, so the Google guides configure a read instance and a
separate write instance per API (`docs/waves/wave-20260930c-google-oauth-discovery/README.md:38`).
Configuring one Gmail write instance took a 20-line native JSON file and a 15-line TOML entry with a
hand-copied configuration revision and executable SHA-256.

`connectors.auth_bindings.Connection` has the states `Live` and `Revoked`; a credential the provider
refuses is visible only as an invoke failure naming `repair_connection`
(`ess/domains/auth_bindings.yaml:145-149`, `docs/catalog-google-oauth.md` § Repair).

## Decision

1. The user's vocabulary is provider, connector, auth profile, instance, connection and operation,
   as defined in `docs/design/connector-vocabulary.md`.
2. A provider is the company or technology (Google, GitLab). A connector is one service or app of a
   provider with one API (Gmail, Drive, GitLab), and is enabled in configuration.
3. An instance is the user's free name for one configured connector, carrying its endpoint and an
   optional description. It belongs to one connector and is not shared across connectors. It holds
   at most one connection.
4. A connection is the stored credential of one instance, obtained through one auth profile the
   connector declares.
5. Every operation of an enabled connector is available on its instances. Configuration does not
   group or filter operations; consumer libraries project. Required-approval writes keep the
   approval policy.
6. "Adapter" is an internal term for an implementation and its process. It leaves the CLI flags,
   the configuration file and the user-facing guides.
7. `Connection` gains the state `Stale`: the provider refused the stored credential. A reconnect
   (today `connections repair`) publishes a new credential and returns it to `Live`. `Revoked`
   stays terminal and is reachable from `Live` and `Stale`. Refreshing an access token is not a
   state change.
8. `connections repair` is renamed `connections reconnect`.

## Consequences

- One instance per account replaces the read and write instance pair; its consent requests every
  scope the connector's operations need.
- Each instance is its own consent; one consent shared across connectors of a provider is out of
  scope.
- Fields the connector ships (auth profile endpoints, bundle, selection, executable, revision) are
  derived by the host instead of written by the user.
- The CLI flags, the configuration format and `ess/domains/cli.yaml` change together; this is the
  public-name migration decision `docs/design.md:152` asks for, and `epic:connector-vocabulary`
  plans it with a migration of existing configuration files.
- Google's 7-day refresh-token expiry for OAuth apps in *Testing* is unaffected; it is lifted by
  publishing the app, which a personal-use app may do without verification.
