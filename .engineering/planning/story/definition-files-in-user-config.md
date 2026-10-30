---
format: aep.planning-md/3
id: story:definition-files-in-user-config
kind: story
status: draft
title: Definition files named in ~/.b10x/connectors.toml become instances read at run time
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
revision: 1
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
- An instance carries an auth `profile` (`config.rs:357-370`); a definition declares its profiles
  (`crates/connectors-catalog/src/definition.rs:1844-1856`).

## Decisions

- SM-D211 (coordinating session, 2026-10-29): the file is the user-level `~/.b10x/connectors.toml`,
  the collection's convention. A repository-local `./.b10x/connectors.toml` is not read by this
  story: a cloned repository could then point a configured credential's connector at a host of its
  choosing, and that needs the same trust decision harness records. "With profiles" is read as:
  each entry names its file, its instance alias, the auth profile it binds and the definition's
  `config` values (such as `hostname`). The operator may correct either reading.

## Acceptance

- A `~/.b10x/connectors.toml` entry naming a definition file outside any repository makes its
  connector's operations listable and invocable under the entry's alias, with no install and no
  rebuild; editing the file changes what the next command serves.
- `connections connect` stores a credential for that instance under the profile the entry names,
  and `connections launch` passes it on.
- A missing file, a file of another format, a file the lowerer refuses (an undeclared `{variable}`,
  an unknown `op`), a profile the definition does not declare, or an alias another instance already
  uses ends in a refusal by name, and no other instance changes.
- A file named here grants nothing on its own: no connection, no operation admission, no OAuth
  exchange.
- `./.b10x/connectors.toml` in the working directory is not read (a test proves it is ignored).

## Not in this story

- A repository-local file and its trust decision.
- Installing a connectors-definitions release (`story:definitions-install`).
