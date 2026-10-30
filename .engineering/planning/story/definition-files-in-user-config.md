---
format: aep.planning-md/3
id: story:definition-files-in-user-config
kind: story
status: draft
title: Definition files named in ~/.b10x/connectors.toml become instances read at run time
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local/definition.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: inferred
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/definitions.rs
- confidence: inferred
  path: crates/connectors-host/tests/security_definitions.rs
- confidence: inferred
  path: ess/domains/cli.yaml
- confidence: inferred
  path: website/docs/guides/invoke-a-definition-file.md
- confidence: inferred
  path: website/docs/reference/contracts/cli.md
- confidence: inferred
  path: website/docs/reference/contracts/model/domains/connectors-cli.md
revision: 11
---
## Outcome

A user names connector definition files in a connectors configuration file, by path, and each
named file becomes a connector instance the running binary serves: any file of the
`urn:beyond10x:connectors-definition:1` format, from connectors-definitions or written by the user,
with no connectors-definitions release, no install and no rebuild.

## Why now

The operator, 2026-10-29, approving `story:definitions-install` (verbatim): "in generall, i would
like to just point it to some yaml files, even custom not from the repo, user provided, referenced
in connectors config (.b10x/connectors.toml with profiles)".

SM-D209 (2026-10-29) requires definitions to be read at run time. `connectors operations invoke
--definition <file>` already lowers one file at run time into `<state>/definitions/<digest>/` as an
instance named `definition` (`crates/connectors-host/src/local/definitions.rs:1-13`); the D218
units keep that as `connectors invoke --definition <file> <operation>`. What is missing is a
standing reference: today the file is named on each command line, and no configuration entry names
a file.

## Context

- The configuration today is `$XDG_CONFIG_HOME/connectors/config.toml`
  (`crates/connectors-host/src/local/config.rs:1133`). Invoked definition instances are held
  beside it in `definitions` (`config.rs:41-46`), never written to the operator's file.
- The collection's other tools read `~/.b10x/<tool>.toml`: `llm` takes `~/.b10x/llm.toml`
  (llm `website/docs/guides/use-the-llm-command.md:40`). Harness does not read a
  repository-local `./.b10x/harness.toml` until a trust decision says how (harness
  `website/docs/reference/configuration.md:279`, `story:repo-local-profiles` there).
- An instance carries an auth `profile` (`Instance::profile`, `config.rs:432-434`); a definition
  declares its profiles (`crates/connectors-catalog/src/definition.rs:1844-1856`).
- The `invoke --definition` store holds one entry per connector and admits only the instance names
  `definition` and `definition-<16 hex>` (`definitions.rs:62-70`, `:135-139`, `:157`); one bad
  entry refuses the whole file, and `read_at` then drops every held entry (`:219-227`, `:369-371`).
  This story needs its own entry format.
- A definition's `source` names a vendored document under a definitions root, today the file's
  grandparent directory or `--definitions-root` (`apps/connectors/src/local/definition.rs:281-288`).
- Lowering refusals collapse to `invalid_input` today (`definition.rs:259`, `:603`), although
  `Refusal::name` exists (`crates/connectors-catalog/src/definition.rs:995`).

## Decisions

- SM-D211 (coordinating session, 2026-10-29), revised by the operator's answer below: the base file
  is the user-level `~/.b10x/connectors.toml`, the collection's convention. "With profiles" is read
  as: each entry names its file, its instance alias, the auth profile it binds and the definition's
  `config` values (such as `hostname`).
- D223 (orchestrator, 2026-10-29): "story:definition-files-in-user-config reads the user-level
  `~/.b10x/connectors.toml` only. A repo-local file is out, because it needs a trust decision."
  Superseded by D224 the same day.
- D224 (operator, 2026-10-29, relayed verbatim by the coordinating session): "A by default, with
  extendsion by <pwd>...". The reading the orchestrator takes, conservative where those words stop:
  - `~/.b10x/connectors.toml` is always read.
  - `<pwd>/.b10x/connectors.toml` extends it and only adds entries.
  - A pwd entry whose alias the user file already has is refused by name. It never overrides, and
    no other entry changes.
  - A pwd entry grants nothing. An instance it adds needs its own `connections connect`, and it
    never reuses a credential bound to a user-file alias (`receives_reuse`,
    `crates/connectors-host/src/local/definitions.rs:301-332`).
- The amendment to ADR D12 (`architecture-decision-record:provider-definitions-and-runtimes`,
  `:191-243`) this story needs before code covers the pwd layer and its trust rule. It rests on the
  operator's answer quoted in D224 and cites it and the D224 rules as already decided; the operator
  is not asked again. The story stays a draft and stays out of wave 20261029-definitions-runtime.

## Acceptance

- A `~/.b10x/connectors.toml` entry naming a definition file outside any repository, with the
  directory its `source` documents resolve against, makes its connector's operations listable and
  invocable under the entry's alias, with no install and no rebuild.
- Editing the definition file changes what the next command serves under the same alias. An edit
  that changes the provider host or the profile's authentication of an alias that already holds a
  connection is refused by name (the registry's `AuthenticationChanged`,
  `crates/connectors-host/src/local/registry/lifecycle.rs:327-336`), never re-bound silently.
- `connections connect` stores a credential for that instance under the profile the entry names,
  and `connections launch` passes it on.
- Each entry is held on its own, apart from the `invoke --definition` store
  (`crates/connectors-host/src/local/definitions.rs:135-139`): an `invoke --definition` of the same
  connector does not replace it, and a user alias is admitted.
- A missing file, a file of another format, a file the lowerer refuses (an undeclared `{variable}`,
  an unknown `op`), a profile the definition does not declare, or an alias another instance
  already uses refuses that entry by its own name (`Refusal::name`,
  `crates/connectors-catalog/src/definition.rs:995`), and every other entry and instance still
  serves.
- A file named here grants nothing on its own: no connection, no operation admission, no OAuth
  exchange.
- `<pwd>/.b10x/connectors.toml` is read as an extension of the user file under D224, one test per
  rule:
  - with no pwd file, the user file's entries serve unchanged;
  - a pwd entry with a new alias is added beside every user-file entry;
  - a pwd entry whose alias the user file already has is refused by name, the user-file entry still
    serves as it did, and no other entry changes;
  - an instance a pwd entry adds holds no connection until its own `connections connect`, and it
    never receives a credential bound to a user-file alias.

## Not in this story

- Installing a connectors-definitions release (`story:definitions-install`).

## Scope

Derived 2026-10-29 by `story-scoper` on 46cb27d65. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `crates/connectors-host/src/local/definitions.rs`, the held-instance store `Config::read_at` merges (`:362-395`) — cited
- **Primary surface:** `apps/connectors/src/local/definition.rs`, the lowering path: `cache` `:553-637`, `native_configuration` `:690-776`, profile choice `:719-727`, endpoint from `default_endpoint` only `:697-699` — inferred
- **Files:** `crates/connectors-host/src/local/config.rs:41-46` (`Config::definitions`) and `:1112-1155` (`Paths`; the XDG default is at `:1133`) — cited
- **Files:** `crates/connectors-host/src/local/config.rs:143-183` (`Rule`, the named configuration refusals) — inferred
- **Files:** `apps/connectors/src/local.rs:38-46` (the preparation hook before the generated runner) and `:573-579` (which commands read `read_at`) — inferred
- **Symbols:** `definitions::{Entry, Entries, record, entries, admitted, is_instance, collides, receives_reuse}`, `Config::{read_at, hold}` — cited
- **Also likely:** `crates/connectors-catalog/src/definition.rs`: `endpoint()` `:2094-2120` for the entry's `config` values, and a refusal for an undeclared profile beside `Refusal` `:863` — inferred
- **Also likely:** `crates/connectors-host/tests/security_definitions.rs`, and a new test file under `apps/connectors/tests/` — inferred
- **Documents:** `contracts/cli/v1alpha1/semantics.md` (§ "Invoking a definition file" `:485`, ConfigurationRule list `:432`), `contracts/cli/v1alpha1/scenarios.md`, `contracts/cli/v1alpha1/fixtures/values.json`, `ess/domains/cli.yaml:712-720` (`ConfigurationRule` variants), the generated `apps/connectors-cli-contract/binding.json` and `website/docs/reference/contracts/{cli.md,model/domains/connectors-cli.md}`, `website/docs/guides/invoke-a-definition-file.md` — inferred
- **Confidence:** medium — the story does not decide whether the run-time read happens in the app's preparation step or in the host's `Config::read_at`; `connectors-host` does not depend on `connectors-catalog` (`crates/connectors-host/Cargo.toml:16-40`), so the host cannot lower a file today
- **Would collide with:** any unit touching `apps/connectors/src/local/definition.rs`, `Rule` or `Paths` in `config.rs`, `ess/domains/cli.yaml:720`, or the module documentation of `definitions.rs`. In flight on 2026-10-29: `unit/d218-u1-invoke` (high, `definition.rs`), `unit/d218-u2-connections` (`config.rs`, `endpoint.rs`), `unit/d218-u3-removal` (high, the same `Rule` hunk and `cli.yaml:720`), `unit/defs-u1-install` (`Paths`, the CLI contract files)
- **Safety fact:** a held entry never replaces an instance the operator's file configures: `Config::hold` skips every entry `collides` names (`definitions.rs:383-395`, `:337-348`), and an entry grants no connection, admission or OAuth exchange (`definitions.rs:12-13`) — level 2, unproven. Not checked: user aliases in `config.definitions` become credential-reuse targets (`receives_reuse`, `definitions.rs:301-332`)
- **Needs before code:** an amendment to ADR D12 (`architecture-decision-record:provider-definitions-and-runtimes`, `:191-243`), which covers only the per-command `--definition`; it must also cover the `<pwd>/.b10x/connectors.toml` extension layer and its trust rule (D224)
