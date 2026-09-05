---
format: aep.planning-md/1
id: story:visual-architecture-handbook
kind: story
status: active
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
revision: 6
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

The six chapters and redesigned overview describe the 0.6.3 implementation with source links. Existing guide and numbered design URLs remain declared. The bundle workflow was regenerated with the exact Atlas renderer from a8fb936ddcb35c8971311610e5c63cc86d612fab; reproducing its previous inputs first matched the original bytes, and the only resulting workflow change adds docs/architecture/*.md.

- `bash scripts/gate.sh` passed all workspaces, catalog verification (65 providers, 68 artifacts), documentation checks, ESS validation and clap drift checks.
- `aep artifact validate` passed with pre-existing historical closure notices.
- The pinned Docs System manifest validator accepted the public manifest.
- `git diff --check` passed.

Website source refresh, rendered desktop/mobile and light/dark review, delivery gates, and live source-revision verification remain in progress. Publication uses the current Atlas source-bundle delivery path; retained source locks support deterministic local verification.
