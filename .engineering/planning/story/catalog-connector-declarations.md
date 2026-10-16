---
format: aep.planning-md/3
id: story:catalog-connector-declarations
kind: story
status: draft
title: Each catalog connector ships its declaration, so an instance needs no native JSON file
relations:
- decomposes: epic:connector-vocabulary
- informed_by: architecture-decision-record:connector-vocabulary
- depends_on: story:ess-connector-vocabulary-model
scope:
- confidence: inferred
  path: adapters/catalog/providers
- confidence: inferred
  path: adapters/catalog/src/local.rs
- confidence: inferred
  path: adapters/catalog/src/main.rs
- confidence: inferred
  path: adapters/catalog/tests
- confidence: inferred
  path: crates/connectors-catalog/src/bundle.rs
revision: 8
---
## Context

Today a catalog instance needs a hand-written native JSON file repeating what the connector already
fixes: `provider`, `bundle_directory`, `api_base`, the whole `auth` object and `operations_file`
(`docs/catalog-google-gmail.md` § Authentication). Bundles live only in the source tree
(`adapters/catalog/generated/bundles`, 1.9 MB); `release-binaries.yml` ships the
`connectors-catalog-provider` binary without them. Gmail needs two instances because
`minimum_scopes`/`requested_scopes` name `gmail.readonly` for reads and `gmail.compose` for the
writes, and neither covers the other.

This story makes each catalog connector ship its own declaration, so the provider binary can build
its native configuration from a connector name, an instance name and an optional endpoint.

## Change

- One connector declaration per catalog provider directory (`adapters/catalog/providers/<name>/`),
  except Slack (below): connector name, default endpoint, the auth profiles it offers, and per
  profile the `requested_scopes`/`minimum_scopes` that cover every operation of its selection.
  Gmail's `google.oauth` profile requests both `gmail.readonly` and `gmail.compose`.
- **Slack ships no declaration here and stays pinned.** `adapters/catalog/providers/slack/` holds two
  selections, `operations.json` (bot) and `user-operations.json` (user token, search), and an
  installation runs them as two instances with different selection files
  (`~/.config/connectors/native/slack-bot.json:24` and `slack-search.json:18` on the operator's
  machine, 2026-10-16). "Every operation of the connector is available on its instances" (ADR
  decision 5) therefore does not hold for one Slack connector. Whether Slack is one connector or
  two is left to a later story; until then Slack instances use the pinned form of
  story:connector-config-dual-read.
- The provider binary carries the bundles, selections and declarations it ships (compiled in, or
  resolved beside the executable; the implementor picks one and says which), so no file path is
  written by the user.
- A derived configuration digests exactly as an equal hand-written one: file paths are not
  digested (`adapters/catalog/src/local.rs:923-960`), so a derived Gmail read configuration with
  the old read-only scopes yields the revision the legacy JSON yields. The existing
  `--local-config <file>` path and every revision it computes stay unchanged.

## Domain relations

- The relations Connector -> Provider, Connector -> AuthProfile and Connector -> Operation, and the
  connector names, come from story:ess-connector-vocabulary-model (`connectors.declarations.Connector`);
  the declarations this story ships use those names and are cited from that model once it validates.
- AuthProfile -> AdapterSpecification, many-to-one, references: `ess/domains/auth_bindings.yaml`,
  entity `connectors.auth_bindings.AuthProfile`, relation `adapter`.
- OperationDeclaration owned by AdapterSpecification, one-to-many: `ess/domains/declarations.yaml`,
  entity `connectors.declarations.AdapterSpecification`, relation `operations`.

## Scope

- `adapters/catalog/providers/*/` (new declaration file per connector)
- `adapters/catalog/src/local.rs` (`Configuration` built from a declaration; digest unchanged)
- `adapters/catalog/src/main.rs` (argument parsing)
- a new `adapters/catalog/build.rs` or `crates/connectors-catalog/src/bundle.rs` if bundles are embedded
- `adapters/catalog/tests/` (revision-equality and Gmail scope tests)

## Acceptance

`connectors-catalog-provider --print-local-bootstrap`, given only the connector name `gmail` and an
instance name, prints the same configuration revision as a reference native JSON whose `auth`
requests both the Gmail read-only and compose scopes and whose other fields equal the
declaration's (the reference file is committed with the test).

## Notes

- Existing `--local-config` files keep their revisions: a Change obligation with its own regression
  test, not part of the acceptance.
- Out of scope: the host reading it (story:connector-instance-derived-fields), live Google
  verification (`credential-blocker:google-oauth-client` is open).
- Related: story:upgrade-moves-catalog-adapters (executable pins after an upgrade) touches how the
  host finds the provider binary; read it before choosing compiled-in vs beside-the-executable.
- `adapters/catalog/src/main.rs` is also swept by story:cli-text-without-adapter, which runs after
  this story.
