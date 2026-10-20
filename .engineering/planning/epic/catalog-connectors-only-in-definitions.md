---
format: aep.planning-md/3
id: epic:catalog-connectors-only-in-definitions
kind: epic
status: draft
title: Catalog connectors live only in connectors-definitions
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- serves: vision:independent-contract-adapters
revision: 4
---
## Outcome

Every declarative connector lives in one place only, `beyond10x/connectors-definitions`. This
repository keeps the lowerer, the engine, the host and the native connectors; it carries no provider
data and no copy of a definition. A connectors install gets its catalog connectors from a tagged
connectors-definitions release.

## Why now

The operator, 2026-10-20: "i do not want any connector like jira which technically should just be a
yaml file in 2 places at once - it should only live in connectors-definitions in my opinion - the
same should be true for all other similar connectors".

architecture-decision-record:provider-definitions-and-runtimes (accepted 2026-10-17) already decides
it: D1 "Connector definitions move to their own repository", its Consequences "This repository loses
the provider data and the per-provider tests that only exercise data", and D5 "`connectors` installs
definitions explicitly and verifies them" (a named tag, `index.json`, a lock file, a local cache;
install grants nothing). Neither consequence is done, and no story tracked it before this epic.

Measured on origin/main 9201b1177: the 0.55.0 Jira change (story:write-postflight-present-check)
had to be made three times, in `adapters/catalog/providers/jira/operations.json`, in the fixture
copy `crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml`, and in
connectors-definitions `connectors/jira.yaml` (pull request
https://github.com/beyond10x/connectors-definitions/pull/4).

## In scope

- What connectors carries today, all to leave this repository:
  - 14 provider directories under `adapters/catalog/providers/` (`operations.json`,
    `connector.json` each);
  - 14 lowered bundles and `index.json` under `adapters/catalog/generated/bundles/`;
  - the copy of the definitions tree under `crates/connectors-catalog/tests/fixtures/definitions/`
    (15 connector files, `auth/`, `upstream/`);
  - per-provider tests that only exercise data (ADR Consequences).
- D5: a command that installs a named connectors-definitions tag, verifies it against `index.json`,
  records it in a lock file and caches it; instance configuration names the locked index.
- The engine and lowerer tests keep a small synthetic definition of their own instead of a provider
  copy.
- connectors-definitions publishes what D5 installs: a tag, a release asset with the files of the
  tagged tree, and `index.json`.

## Not in scope

- Native connectors (Kubernetes, SQL, MCP, anything that starts a host process or reads host
  credentials): they stay here (D1).
- Signing beyond digests (ADR "Not decided here").
- D10 pages per connector: not needed for each definition to live in one place only. D10 stays
  decided (ADR) and is planned on its own.

## Order

Set with the b10x-chef-orchestrator session on 2026-10-20, after the v0.55.0 release:

1. D5: install from a connectors-definitions release (fetch a tag, verify `index.json`, lock file,
   cache).
2. A check that every in-tree provider (the 14 directories and `slack-user`) exists in
   connectors-definitions at the pinned tag, red if not.
3. Tests read the installed release instead of `tests/fixtures/definitions`.
4. The build embeds the pinned release, and then `adapters/catalog/providers`,
   `adapters/catalog/generated/bundles` and the fixture copy are deleted. First-run is option C of
   `decision-blocker:catalog-first-run-source`, decided by the operator on 2026-10-20: the release
   named by the committed pin is fetched and digest-checked into a cache before cargo runs, never
   committed, and the binary carries it. An instance resolves from that embedded release, or from
   the locked index after an explicit D5 install of another tag.

## Open questions

- Decided 2026-10-20, option A of `decision-blocker:slack-openrouter-pinned-source`: `slack`,
  `slack-user` and `openrouter` are served by name from the embedded release, and Slack stays two
  connectors. The Slack and OpenRouter guides move to `--connector`, and an instance configured by
  path moves through `setup migrate`, with a test that one migrates
  (`story:instances-resolve-through-locked-index`).
- Whether the `connector.json` byte-equality checks (D11) move to connectors-definitions' gate.
- connectors-definitions has no tag and no release yet (`git tag` and `gh release list` empty on
  2026-10-20); `story:definitions-release-index` cuts the first one after
  https://github.com/beyond10x/connectors-definitions/pull/4 merges.
