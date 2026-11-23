---
format: aep.planning-md/3
id: verification-report:connectors-v0742-release-scope
kind: verification-report
status: draft
title: connectors 0.74.2 release scope
summary: Scope, exclusions and evidence of the 0.74.2 cut
revision: 1
---
## Scope of connectors 0.74.2

Released 2026-11-23 under Timo's instruction of that day: "We always merge what we can merge, and
if you're done, cut a new release", and host-operator's "Ship #279/#277 and 0.74.2 as planned".

- Pull request #274: ESS 0.97.3 -> 0.97.4, which removes a synthesis slowdown and keeps suites,
  refusals and notes byte for byte.
- Pull request #276: a pull request that changes only planning or prose files passes its required
  checks without building or testing (CI only).
- The gate and CI pin aep 0.80.0 (was 0.71.3), so a planning store with a redacted record
  validates (was pull request #279).
- Planning records only: the 2026-11-23 review findings (#278), the complete Slack Web API epic
  (was #277) and two dev-operator gaps (was #280). All are drafts; none is implemented.
- Patch release: no CLI, wire or provider behaviour changes.

Pull requests #279, #277 and #280 were folded into the release pull request on 2026-11-23, when
the organization's runner queue held about 45 queued runs (host-operator, 04:37 CET: "Push
integration branches once, not per step").

## Not in this release

- Issue #267 (metadata store growth per read).
- The 14 draft stories under epic:review-20261123-findings.
- The 7 draft stories under epic:slack-complete-web-api.

## Evidence

The release pull request's required checks on the release commit, then the tag, release page,
archives, `SHA256SUMS` and image, verified after publication.
