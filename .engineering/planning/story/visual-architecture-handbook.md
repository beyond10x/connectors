---
format: aep.planning-md/1
id: story:visual-architecture-handbook
kind: story
status: implemented
title: The public handbook explains the current Connectors system
relations:
- derived_from: epic:public-surface
scope:
- confidence: cited
  path: .github/workflows/b10x-docs-bundle.yml
- confidence: cited
  path: README.md
- confidence: cited
  path: b10x.docs.yaml
- confidence: cited
  path: docs/architecture
- confidence: cited
  path: docs/design/01-domain-model.md
- confidence: cited
  path: docs/design/02-architecture.md
- confidence: cited
  path: docs/design/18-governed-outbound-mcp-services.md
revision: 9
---
## Outcome

Publish a visual architecture handbook for the current Connectors implementation. The operator accepted a six-chapter handbook, a redesigned overview, and a concise ESS/Entity Runtime status explanation; implementation and website publication are authorized in this interactive session.

## Grounding

`README.md` currently leads with engineering/deployment detail. `b10x.docs.yaml` publishes it, seven guides, and three numbered design documents. `docs/design/19-the-cli-surface.md` records the generated CLI's limits. `ess/system/` and `ess/system.yaml` are separate specifications. Runtime authority remains in the named Rust owner crates.

## Acceptance

- The landing page explains the system and links six ordered architecture chapters: system overview; connections/authority/credentials; commands/interfaces; events/state; deployment/runtime; specification status.
- Diagrams and tables describe current implementation with source links and accessible text explanations. Existing provider-guide and design URLs survive.
- The public manifest exposes the handbook, with meaningful titles, descriptions, audience metadata, and navigation order.
- Connectors and Website gates pass. Rendered desktop/mobile and light/dark views are inspected, including diagram rendering, navigation, and search.
- The source commit is published through organization bot tooling, Website inputs and the Atlas snapshot are refreshed through their deterministic tools, and the published handbook's source revision is verified.

## Scope

- cited: README.md, b10x.docs.yaml, docs/design/01-domain-model.md, docs/design/02-architecture.md, docs/design/18-governed-outbound-mcp-services.md
- new: docs/architecture/*.md
- cited: Website sources.lock.json and data/bootstrap, Atlas documentation publication commands

Public APIs, runtime behavior, ESS completion, and Entity Runtime adoption are unchanged. Existing website components and passive Markdown/Mermaid are sufficient.

## Validation evidence

Implemented and published on 2026-09-05.

- Connectors source a71b2adc50356dfd92e81687a519cd38e1a0ac48 contains the redesigned overview, six handbook chapters, preserved legacy URLs, public metadata, and the Atlas-generated bundle trigger. `bash scripts/gate.sh` passed all workspaces, 65 providers/68 artifacts, link/story checks, ESS validation, and clap drift checks. The pinned Docs System manifest validator passed.
- Website commits 43016633f0cc54d57c77ab573dd02ea037ba5f90 and 64383b0009c8d9124790caa273a01415b6b523bc refresh the deterministic remote source lock. Atlas generated the four-file snapshot in Website a6648e91c391e0cd87d596e9074bdf02b5512784. `npm run gate` passed: 348 indexed pages, search checks, responsive navigation checks, 347 routes, 1300 files, 22697 crawled references, and production provenance. Atlas `docs verify-portal` passed against the built artifact.
- Browser review covered all seven pages at 1440x1000 and 390x844 in light and dark themes: 28 combinations, rendered Mermaid diagrams, no page overflow or browser errors, ordered mobile chapter navigation, and search results for the specification chapter. Diagram content also has adjacent prose/table explanations.
- ESS fd06a4d61bfb7b4990617810655dc181d6a3ab00 repairs the five invalid publication records. All six ESS records validate; `task check` and `task site-build` passed. Both Connectors and ESS immutable bundle workflows succeeded.
- Atlas publication run 33935198485 completed successfully, including an independent Website build and artifact verification. Live v2 provenance names Connectors a71b2adc50356dfd92e81687a519cd38e1a0ac48 and ESS fd06a4d61bfb7b4990617810655dc181d6a3ab00; source-set digest 951f2fc1a26456b6ed41e58c7415f25f4b4a744b39ae6f8e92743681d8aaf31a. The production Website runtime remains the Atlas-pinned 98f14dd2dc8bed13932b2ba72d5a8138f11ea309; content publication uses immutable bundles.
- Atlas `docs verify-pages --root-only` passed 26 delivery routes. The seven handbook/overview routes, retained Slack/GitLab guides, and three numbered design routes all returned HTTP 200. Live specification text and exact source revision were verified at https://beyond10x.github.io/docs/connectors/architecture/specification/ and https://beyond10x.github.io/.well-known/b10x-docs.json.

The full local organization reconciliation check still reports pre-existing aep/docs catalog/primary-manifest drift. It is not a delivery failure; the artifact and live delivery gates above pass. No unrelated primary checkout was changed.

## Delivery dependency repair

Atlas publication run 33934538272 fails before rendering because five published ESS change records violate b10x-change/v1. Validate the full remote change corpus and repair only those records: bounded summaries, declared kind/journey vocabulary, known surface ids, and observed release source metadata. Authored scenarios appear in the published 0.16.0 release; 0.15.0 has no GitHub Release, so preserve the record id and cite 0.16.0 instead of inventing a release. ESS `task check` and `task site-build` are required before publishing this delivery dependency.

This repair is necessary for the authorized Connectors website publication. It changes no runtime, schema, or release tag.
