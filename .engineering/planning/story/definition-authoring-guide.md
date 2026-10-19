---
format: aep.planning-md/3
id: story:definition-authoring-guide
kind: story
status: active
title: A guide to authoring a definition for an API with no catalog
relations:
- serves: vision:independent-contract-adapters
- informed_by: architecture-decision-record:provider-definitions-and-runtimes
scope:
- confidence: cited
  path: website/docs/guides/author-a-definition.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:39Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:40Z", actor: "human:timo", revision: 4}
---
## Outcome

A guide, "Author a definition for an API that ships no catalog", shows an engineer how to turn an
API's OpenAPI document into a `connectors-definition/1` file kept in a definitions root outside
this repository, connect to it with a token profile, and invoke an operation, with every command
run in the tree against a fixture API. The guide that exists
(`website/docs/guides/invoke-a-definition-file.md`) starts from `connectors-build definition
import --catalog gitlab`, a shipped selection; a team's private APIs ship no catalog, so the
authoring step is undocumented. The trial of 2026-10-18 did it by hand.

## Surface

`website/docs/guides/author-a-definition.md` (new), listed in the guides index and in the
vocabulary check's file set (`AGENTS.md`, `crates/connectors-docs/src/vocabulary.rs`). A fixture
OpenAPI document of a small API (three reads, one write) under
`crates/connectors-catalog/tests/fixtures/definitions/upstream/` and the definition that selects
from it, used by a test that lowers it, runs one read through `operations invoke --definition`
against a scripted fixture server, and asserts the guide's pasted output. No change to the lowerer
or the runtime: if the walk finds a step that needs one, that is a finding, recorded here, and the
guide says what is manual.

## Acceptance

- The guide's steps: vendor the document (gzip, SHA-256, license, `upstream/<id>/README.md` as the
  definitions repository requires), write `connectors/<id>.yaml` with `base_url`, the auth profile
  (token or OAuth) and the selected `operationId`s with bounds, validate it (`task check` in a
  definitions root, or the schema check the lowerer runs), and invoke; each command's output is
  pasted from a run in the tree.
- `cargo test -p connectors-catalog --test <the new test>` passes and fails when the guide's
  pasted output drifts.
- `connectors-docs vocabulary` and `generate --check` pass with the new page.

## Scope

Primary: `website/docs/guides/author-a-definition.md`, `website/sidebars.*` (guides index),
`crates/connectors-catalog/tests/` (new test and fixtures), `crates/connectors-docs/src/vocabulary.rs`,
`AGENTS.md` (the vocabulary file set). Collides with story:parity-page-recount-20261019 only if
both edit `AGENTS.md`; the recount does not.
