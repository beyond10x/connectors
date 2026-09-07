---
format: aep.planning-md/1
id: story:release-0-7-0
kind: story
status: implemented
title: Publish Connectors v0.7.0
relations:
- derived_from: runbook:cli-ten-slack-first
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: WHATS-NEW.md
- confidence: cited
  path: b10x.docs.yaml
- confidence: cited
  path: catalog
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates
revision: 8
---
## Outcome

Publish the approved Connectors v0.7.0 release from the nine implemented CLI stories and all changes since the latest published v0.6.0 release.

## Acceptance

The exact approved release source passes all twelve workspace gates, shared checks and four native builds, and GitHub publishes v0.7.0 with all four Unix archives and SHA256SUMS whose downloaded bytes and native Linux version/help are verified.

## Delivery

Update package and internal dependency identities together, refresh all Cargo locks without changing external dependencies, regenerate the canonical catalog through its owner, and publish a dated changelog and bot-authored source and annotated tag through the existing delivery workflow.

The several-credentials story stays excluded; personal OAuth remains unsealed and development-only. Existing unrelated obligations retain their recorded state.

This story has one delivery task. The decomposition critic panel is skipped because there are fewer than two children to compare.

## Consumer release highlights

The operator additionally requested a root-level document with user-friendly release explanations. Add WHATS-NEW.md for the latest release, link it from README.md, include it in release archives and link to it from the documentation overview, and preserve CHANGELOG.md as the detailed historical record. Publish the source first, then refresh and deliver the Website documentation through the existing owners.

Use the approved tag workflow itself for the full twelve-workspace gate and four native builds; publish the release only after those jobs pass. The release source branch starts at the approved main commit and is integrated into main after its gates are green.
