---
format: aep.planning-md/3
id: story:cli-text-without-adapter
kind: story
status: draft
title: CLI help and error messages use the connector vocabulary and never say adapter
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:cli-connector-instance-selectors
- depends_on: story:connections-reconnect-verb
- depends_on: story:setup-migrate-adapter-entries
- depends_on: story:connection-stale-state
- depends_on: story:catalog-connector-declarations
- depends_on: story:connector-instance-derived-fields
- depends_on: story:connect-refused-while-instance-holds-connection
scope:
- confidence: inferred
  path: adapters/catalog/src/main.rs
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: crates/connectors-host/src/local
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 7
---
## Context

Epic acceptance: no CLI help text or error message says "adapter". Today the root `about` reads
"Local integration access through explicitly configured adapters"
(`apps/connectors/spec/cli.yaml:5`), the generated `apps/connectors-cli-contract/help.txt` and
`README.md` repeat it, and command descriptions name adapters (`adapters list`: "List configured
adapter entries"). Error and refusal text rendered by `apps/connectors` and the host names adapters
too. This story is the text sweep after the renames (stories cli-connector-instance-selectors,
connections-reconnect-verb, setup-migrate-adapter-entries, connection-stale-state) have landed, so
the sweep runs once over the final command set.

## Change

- Rewrite every `about` and option description in `apps/connectors/spec/cli.yaml` in the vocabulary
  of `docs/design/connector-vocabulary.md`; regenerate `apps/connectors-cli-contract/` (help,
  README, completions).
- Rewrite the `--output human` rendering of failures in `apps/connectors/src/` and any user-facing refusal
  text in `crates/connectors-host/src/local/` that says adapter. Internal names (types, modules,
  log fields) stay.
- `connectors-catalog-provider --help` likewise (`adapters/catalog/src/main.rs`, after the arguments
  story:catalog-connector-declarations adds).
- Regenerate the website's generated CLI reference (`cargo run --locked -p connectors-docs -- generate`);
  `website/docs/reference/cli.md` is generated and not edited by hand (`website/README.md`
  § Edit the owning source).

### The checked text, exactly

1. Standard output of `connectors --help` and of `connectors <path> --help` for every `path` under
   `commands` in `apps/connectors/spec/cli.yaml`, deprecated aliases included.
2. Standard output of `connectors-catalog-provider --help`.
3. The standard error the CLI renders under `--output human` for every variant of `connectors.cli.FailureCode`,
   `connectors.cli.FailureStage` and `connectors.cli.NextAction` (`ess/domains/cli.yaml`), produced
   by calling the renderer once per variant.

Not checked, because they are wire values and type names, not rendered text: the fixtures in
`contracts/cli/v1alpha1/fixtures/` and these fields kept for the deprecation window: JSON `adapter`,
`adapter_id`, `default_adapter`, callable ids `adapters-*`, type names `connectors.cli.Adapter*`.

### Allowlist

A line of the checked text may contain "adapter" (case-insensitive) only if it is the deprecated
`--adapter` option or the deprecated `adapters` command path and the same line contains the word
`deprecated`.

## Domain relations

None: text only.

## Scope

- `apps/connectors/spec/cli.yaml`, `apps/connectors-cli-contract/` (generated)
- `apps/connectors/src/**` (messages)
- `crates/connectors-host/src/local/**` (user-facing refusal text only)
- `adapters/catalog/src/main.rs` (help)
- `website/docs/reference/cli.md` (regenerated)
- `apps/connectors/tests/` (absence test)

## Acceptance

A test in `apps/connectors/tests/` over the checked text listed under Change finds no line
containing "adapter" (case-insensitive) outside the allowlist.

## Notes

- Ordering: after story:catalog-connector-declarations (`adapters/catalog/src/main.rs`) and
  story:connector-instance-derived-fields (`crates/connectors-host/src/local/config.rs`,
  `runtime.rs`), both by `depends_on`; story:docs-vocabulary-readme-website regenerates the website
  after this story.

## Coordinator additions

## Coordinator additions

From the reconnect adversary (2026-10-16):

- Under `--output human`, `--help` on a deprecated name prints the deprecation line
  (`apps/connectors/src/local/select.rs:310-315`), although its doc comment says help is left as
  is; `connectors --help` lists `adapters` beside `instances` and `connections --help` lists
  `repair` and `reconnect` with one about, with no deprecation mark. This story fixes both.
- The runtime silently overwrites a real command when an alias collides with it
  (`runtime.rs:287-298`); add a guard.
- The `adapters describe` descriptor keeps `adapter` (implementation) and `instance` (instance id),
  documented as an exception in the CLI contract (commit eca9692bd); renaming those fields is in
  scope here.
