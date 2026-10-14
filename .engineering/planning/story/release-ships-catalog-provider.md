---
format: aep.planning-md/3
id: story:release-ships-catalog-provider
kind: story
status: implemented
title: Release archives carry the catalog provider binary
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T07:02:23Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T07:02:24Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T22:06:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A release's assets carry every binary a documented local setup runs: `connectors` and
`connectors-catalog-provider` for each target, in the archives `SHA256SUMS` covers.

## Today

`.github/workflows/release-binaries.yml:116` builds only `-p connectors`, so the v0.50.0 archives
hold `connectors` alone. A local install that follows docs/catalog-slack.md or
docs/local-catalog-provider.md needs the catalog provider binary as well, and had to build it from
the tag.

## Change

The release workflow builds and packages `connectors-catalog-provider` beside `connectors` for every
target it already ships (or each native adapter binary a guide names, if the guides name more), and
the release notes and install guide say which binary each archive holds. A check in the repository
gate fails when a guide names a binary the release workflow does not package.

## Acceptance

- The next release's archives each list `connectors` and `connectors-catalog-provider`; both pass
  `sha256sum -c SHA256SUMS`, and `connectors-catalog-provider --version` reports the tag's version.
- The gate check fails on a guide naming an unpackaged binary (a test with a fixture guide).
