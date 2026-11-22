---
format: aep.planning-md/3
id: story:status-page-current-after-a-release
kind: story
status: draft
title: The status page states what a release holds and marks nothing released as unreleased
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Noted by the release agent of 0.70.0 (pull request 245, `e0d604a86`), 2026-11-10, relayed by the
coordinator: the generated status page `website/docs/status.md` (and `website/data/status.json`,
both written by `connectors-docs` from one list) is stale after the release.

- Its intro (line 12 at `c5d21888a`) says "As of 2026-10-16" and that a release page carries the
  binaries of `connectors` and `connectors-catalog-provider` only, while the "Release binaries" row
  (line 113) says each archive holds six programs (`connectors`, `connectors-catalog-provider`,
  Grafana, Prometheus, Loki, Kubernetes).
- Rows still mark capabilities *unreleased* that shipped in a tagged release (lines 50, 61, 62, 63
  among them).

## Acceptance

- The status page's intro names the programs a release archive holds, as the "Release binaries" row
  does, and its date is the date of the list it is generated from.
- No row marks *unreleased* a capability that is in a tagged release; a `connectors-docs` check fails
  when one does (for example by comparing the markers against the CHANGELOG's released sections).
