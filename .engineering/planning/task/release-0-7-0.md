---
format: aep.planning-md/1
id: task:release-0-7-0
kind: task
status: implemented
title: Cut and verify Connectors v0.7.0
relations:
- derived_from: runbook:cli-ten-slack-first
- decomposes: story:release-0-7-0
revision: 7
---
## Outcome

[Connectors v0.7.0](https://github.com/beyond10x/connectors/releases/tag/v0.7.0) is published and verified from the approved nine-story implementation. The operator approved the cut and immediate publication.

## Delivered scope

All 206 internal package/dependency references across 29 manifests, twelve Cargo lockfiles, canonical catalog documents, the generated catalog pack and connectors.lock carry consistent release identities. External dependency specifications and graphs are unchanged. Generated changes are version identities and derived digests only.

CHANGELOG.md records the full release history; WHATS-NEW.md explains the latest release in plain language and ships in every archive. The README and live documentation overview link to it. The release workflow adds this document to its archive inventory; existing product behavior is unchanged by the cut.

## Verification

Release workflow 34097008095 passed every required workspace gate, shared check and native build. All four downloaded Unix archives match SHA256SUMS and contain documentation identical to the tagged source. The downloaded Linux executable reports connectors 0.7.0 and passes its help smoke test.

The Website gate, CI, managed portal check, immutable publication 34101442041, complete artifact comparison, source-set freshness and live Pages verification passed. The separate website release-feed freshness gap remains recorded under Website story publish-connectors-release-highlights.

The earlier resource refusals and interrupted local check are retained in the journal and verification evidence; neither is a passing result. A later guarded catalog build completed successfully, and the authoritative release-profile check passed in CI. The several-credentials story remains excluded and personal OAuth remains development-only.

See verification-report:release-0-7-0 for source identity, archive digests, documentation proof and the actual organization-wide fence limitations.
