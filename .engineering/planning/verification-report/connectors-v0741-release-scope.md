---
format: aep.planning-md/3
id: verification-report:connectors-v0741-release-scope
kind: verification-report
status: draft
title: connectors 0.74.1 release scope
summary: Scope, exclusions and evidence of the 0.74.1 cut
revision: 1
---
## Scope of connectors 0.74.1

Released 2026-11-22 under Timo's order of about 17:02 CET (relayed by host-operator): "cut
versions in the repos we delivered work to default branch and can push, tag those, cut releases".

- Pull request #272: ESS 0.87.0 -> 0.97.3. Synthesis CPU on this model 5.9 s -> 2.4 s,
  `connectors-docs generate` 714 s -> 170 s. The model declares
  `connectors.declarations.DeclareAdapterSpecification` (ESS 0.95.0 arranges a referenced row
  through its creating command); no host route sends it (Timo's pick H13 A). The feed binding
  suite holds 27 scenarios, not 28.
- Patch release: no CLI, wire or provider behaviour changes.

## Not in this release

- Issue #267 (metadata store growth per read).
- The test step does not yet fail above 120 s (635 s on #271's gate).
- The Windows "host and CLI" job fails on `failed to remove file target\debug\connectors.exe`.

## Evidence

The release pull request's required checks on the release commit, then the tag, release page,
archives, `SHA256SUMS` and image, verified after publication.
