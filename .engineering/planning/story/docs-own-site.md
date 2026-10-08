---
format: aep.planning-md/3
id: story:docs-own-site
kind: story
status: active
title: Connectors documentation on its own site with the shared look and feel
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:00:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:00:37Z", actor: "human:timo", revision: 3}
---
## Outcome

Connectors' documentation is one consistent set on its own site at
`https://beyond10x.github.io/connectors/`, with the shared look and feel of the other independent
beyond10x sites, and README.md and AGENTS.md consistent with it. Today the content is served from the
unified site at `https://beyond10x.github.io/docs/connectors/`, and the GitHub website field points there.

## Scope of this story

The repository side of the workspace docs procedure (`.agents/skills/docs/SKILL.md` at the
workspace root), stages A and B, plus the requests for stage C:

- Stage A: `website/docusaurus.config.ts` with `baseUrl: '/connectors/'` and `projectName: 'connectors'`;
  `@beyond10x/docs-system` at the commit the other independent sites pin; the page set (index,
  getting-started, concepts, guides, examples, reference, status) with front matter on every page;
  generated pages from a Rust docs crate, drift-checked by the repository gate; the
  `Documentation validation` and `Documentation site` workflows.
- Every CHANGELOG entry since the site content was last updated, v0.37.0 and this wave's Loki and
  Slack entries included, is on a page or recorded as not consumer-relevant with a reason.
- README.md links the site on its first screen; AGENTS.md and README.md name the same crates and
  commands as the site.
- The unified-site files (`b10x.docs.yaml`, `b10x-docs-bundle.yml`, `b10x-docs-pages.yml`,
  `b10x-docs-check.yml`) stay until the organisation side (stage C) has landed; deleting them is
  stage C's last step and a later change.

Out of scope: the organisation-side changes of stage C (website redirects, root caller, catalog),
which other repositories own; the GitHub website field, which follows stage D.

## Acceptance

- The docs crate's `generate --check` and the site build exit 0 on the integration branch; the
  repository gate runs `generate --check`.
- Every command shown on a page was run in the tree and its output pasted from that run.
- No page states an unshipped capability as shipped; planned items are labelled.
- After merge to `main`, the `Documentation validation` and `Documentation site` runs for the
  merge commit conclude successfully.
