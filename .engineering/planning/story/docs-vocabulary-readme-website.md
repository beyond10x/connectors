---
format: aep.planning-md/3
id: story:docs-vocabulary-readme-website
kind: story
status: implemented
title: README introduces the connector vocabulary once; AGENTS.md and the website link to it
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:connector-instance-derived-fields
- depends_on: story:setup-migrate-adapter-entries
- depends_on: story:cli-connector-instance-selectors
- depends_on: story:connections-reconnect-verb
- depends_on: story:connection-stale-state
- depends_on: story:cli-text-without-adapter
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: crates/connectors-docs/src
- confidence: inferred
  path: website/data
- confidence: inferred
  path: website/docs/concepts
- confidence: inferred
  path: website/docs/getting-started.md
- confidence: inferred
  path: website/docs/guides
- confidence: inferred
  path: website/docs/index.md
- confidence: inferred
  path: website/docs/reference
- confidence: inferred
  path: website/product.json
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T19:07:44Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-16T19:07:44Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:44Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Context

Epic acceptance: no user-facing guide says "adapter" for the configured thing. `README.md` uses the
word 24 times, `AGENTS.md` 7, and the hand-written website sources (`website/docs/index.md`,
`getting-started.md`, `concepts/`, `guides/`, `reference/adapters/*/index.md`, `website/product.json`
7, `website/data/terminal.json` 4) describe configuration and the CLI in the old vocabulary.

This story owns the introduction of the terms. It follows the repository's docs workflow (the
`docs` skill): `website/docs`, `README.md` and `AGENTS.md` change as one set. `README.md` is for
humans, `AGENTS.md` for agents; each links rather than duplicates.

## Change

- `README.md` introduces provider, connector, auth profile, instance, connection and operation once,
  in a table, in the words of `docs/design/connector-vocabulary.md` § Decision, with the Gmail and
  GitLab example and the configuration sketch in its final form
  (story:connector-instance-derived-fields).
- `AGENTS.md` links to that section and states the rule for agents: "adapter" is the internal name
  of an implementation and its process, never of the configured thing. It does not restate the
  definitions.
- Hand-written website sources link to the README terms and are rewritten where the vocabulary
  makes them stale: configuration examples, CLI commands (`--connector`/`--instance`,
  `connections reconnect`, `instances …`, `setup migrate`), the Stale state.
  `website/data/terminal.json` is re-recorded from a run, as `website/README.md` requires of every
  command shown. `website/docs/concepts/adapters.md` and `website/docs/reference/adapters/*/index.md`
  describe implementations (ADR decision 6): rewritten only where they show configuration or
  commands, and not in the checked set.
- Generated pages (`website/docs/reference/cli.md`, `reference/contracts/`, model pages,
  `status.md`) are not edited; they are regenerated with
  `cargo run --locked -p connectors-docs -- generate`.
- Content the vocabulary change does not make stale is left alone.

### The vocabulary check

This story adds `cargo run --locked -p connectors-docs -- vocabulary [--no-pins] [--one-instance]
<file>...`, which the guide stories run unchanged. It exits non-zero when, in any given file:

1. a line contains "adapter" (case-insensitive) and none of the allowlist: `adapters/` (a
   repository path), `adapter_id`, `` `adapter` `` (the JSON field kept for the deprecation window),
   or one of the words `deprecated`, `legacy`, `internal` on the same line (the deprecated flag and
   command path, the legacy `[adapters.<alias>]` table, and the statement that adapter is the
   internal name);
2. a file other than `README.md` holds a Markdown table row whose first cell is `provider`,
   `connector`, `auth profile`, `instance`, `connection` or `operation` (the terms are defined in
   `README.md` only);
3. with `--no-pins`, a line contains `configuration_revision`, `sha256` or `bundle_directory`;
4. with `--one-instance`, more than one line starts with `[connectors.` and contains `.instances.`;
5. `README.md` is among the given files and lacks a `## Vocabulary` section holding a table row for
   each of the six terms, or `AGENTS.md` is among them and holds no link to `README.md#vocabulary`.

## Domain relations

None asserted beyond what the README quotes from the ESS model of Provider, Connector and Instance
(drafted under `ess/domains/` in this branch); the README links to that model's generated page
rather than restating cardinalities.

## Scope

- `README.md`, `AGENTS.md`
- `website/docs/index.md`, `website/docs/getting-started.md`, `website/docs/concepts/*.md`,
  `website/docs/guides/*.md`, `website/docs/reference/adapters/*/index.md`
- `website/product.json`, `website/data/terminal.json`
- `crates/connectors-docs/src/` (the vocabulary check)
- regenerated: `website/docs/reference/`, `website/docs/status.md`, `website/data/status.json`

## Acceptance

`connectors-docs vocabulary` exits 0 over `README.md`, `AGENTS.md`, `website/product.json`,
`website/data/terminal.json`, `website/docs/index.md`, `website/docs/getting-started.md`,
`website/docs/guides/*.md` and `website/docs/concepts/*.md` except `concepts/adapters.md`; by rule 5
it exits non-zero on today's tree, where `README.md` has no vocabulary section.

## Notes

- Lands in the same wave as, or right after, the CLI and configuration stories it describes.
- Guides under `docs/` are in story:docs-local-guides-vocabulary, story:docs-catalog-guides-vocabulary,
  story:docs-pinned-catalog-guides-vocabulary, story:docs-google-one-instance-per-account and
  story:docs-google-publishing-advice, which link to the README section this story writes and run
  the vocabulary check it adds.
