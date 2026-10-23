---
format: aep.planning-md/3
id: story:definitions-release-index
kind: story
status: implemented
title: connectors-definitions publishes a tagged release whose index.json names every file SHA-256 and the commit
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T12:37:09Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-23T12:37:09Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-23T12:43:11Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A tag pushed to `beyond10x/connectors-definitions` from 0.3.0 on publishes the three things the installer
fetches, and the first such tag exists:

1. the tag itself;
2. a release asset holding the files of the tagged tree under `connectors/`, `auth/` and
   `upstream/`;
3. `index.json`, naming the tag, the tagged commit and the SHA-256 of every file in that asset.

This story owns what is published. `story:definitions-install` consumes exactly these three and
adds nothing to the publishing side.

## Context

- ADR D4: "Each tag publishes the definitions, the vendored sources and one `index.json` naming
  each file's SHA-256 and the commit."
- connectors-definitions has no tag and no release: `git ls-remote --tags origin` and
  `gh release list` are empty on 2026-10-20. Its only workflow is `.github/workflows/check.yml`
  (`task check`), and `Taskfile.yml` has the tasks `schema` and `check` only (origin/main
  `a092421`).
- An index that names the commit it describes cannot be a file inside that commit, so
  `index.json` is written after tagging and published beside the tag, not committed (inferred
  from the wording of D4).
- The index format is declared where the definition format is: ESS under `spec/` of
  connectors-definitions, projected to JSON Schema (inferred from D2: "The schema's source is ESS
  (`spec/domains/definition.yaml` in the definitions repository)"). It carries a format version
  and the features the release requires, so the D5 refusals "an unknown format version" and "an
  unmet feature" each have a field to read (inferred from D5).
- A note for the installer, not a choice left open here: the Connectors host HTTP client follows
  no redirect and uses no proxy (`crates/connectors-host/src/http.rs:178-179`), and release asset
  downloads may answer with a redirect (not verified in this session).
- Every write to that repository goes through `b10x-bot[bot]` (its `AGENTS.md`, Delivery). The
  release process this story adds becomes that repository's own.
- The first tag carries what this repository ships. Measured on 2026-10-20 by byte comparison of
  `crates/connectors-catalog/tests/fixtures/definitions/` here with connectors-definitions
  origin/main: every connector, auth and upstream README file is equal except
  `connectors/jira.yaml`, which lacks the 0.55.0 present checks of
  https://github.com/beyond10x/connectors-definitions/pull/4 (open on 2026-10-20). That pull
  request merges before the first tag; a tag without it lowers Jira differently from this tree,
  and `story:definition-tests-read-pinned-release` would show it red.

## Domain relations

- Release to index entry: one-to-many; the release owns its entries; an entry never changes for
  a given tag and does not exist without its release. Decided by ADR D4 (accepted 2026-10-17).
  No `ess/1` document declares it; this story declares it in connectors-definitions `spec/`.

## Scope

All paths are in `beyond10x/connectors-definitions`.

- `.github/workflows/check.yml` (cited: the only workflow; the release job sits beside it)
- `.github/workflows/` release workflow on tag push (inferred)
- `Taskfile.yml` (cited: `schema` and `check` only today; a task writing `index.json` and the
  asset, inferred)
- `spec/domains/` and `generated/` (inferred: the index format, projected by `task schema`)
- `README.md`, `AGENTS.md` (inferred: how a release is cut)

## Not in this story

- D10 pages per connector: not needed for each definition to live in one place only (epic, Not
  in scope).
- Signing beyond digests (ADR, Not decided here).
- Lowering in the definitions gate with a pinned Connectors release (design section 4), and
  whether the D11 byte-equality checks move there (epic open question).

## Acceptance

- The first tag of `beyond10x/connectors-definitions` that publishes an index (0.3.0; 0.1.0 and 0.2.0 predate this story and stay without one) has a release asset and an `index.json`
  whose entries match the SHA-256 of every file under `connectors/`, `auth/` and `upstream/` in a
  checkout of that tag, and whose commit is the tagged commit.
- At that tagged commit, `connectors/jira.yaml` carries the present checks of pull request 4:
  `after: {expect: {/id: "?", /key: "?"}}` on `issue.create`, and `/id: "?"` in the `after` of
  `issue.comment.add`.
