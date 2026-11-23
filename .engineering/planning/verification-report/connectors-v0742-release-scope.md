---
format: aep.planning-md/3
id: verification-report:connectors-v0742-release-scope
kind: verification-report
status: accepted
title: connectors 0.74.2 release scope
summary: Scope, exclusions and evidence of the 0.74.2 cut
revision: 4
transitions:
- {from: "draft", to: "accepted", at: "2026-11-23T05:25:18Z", actor: "human:timo", revision: 3}
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

Verified 2026-11-23 after publication:

- Release pull request #281 merged as `50ba58849b02c5ed532cbdcb72b802dda1116f5a`; its tree `39951194aed9` equals the pull request's tree, whose required checks passed.
- Tag `v0.74.2` by `b10x-bot[bot]` at 05:42 CET; it peels to the merge commit, which is an
  ancestor of `origin/main`.
- Release page https://github.com/beyond10x/connectors/releases/tag/v0.74.2, author
  `b10x-bot[bot]`, not a draft, Latest, published 05:44 CET.
- Release run 45113657301: created 04:44 UTC, ended 05:23 UTC, all 7 jobs succeeded, no rerun.
- The page has 5 assets: the 4 archives pass `sha256sum -c SHA256SUMS`.
- Image `ghcr.io/beyond10x/connectors:0.74.2`, index
  `sha256:2c72e1c0db82f9d38b52180123f0e1e557fffd6f599fbf5cacbe0e0d4e6d1db8`, platforms
  linux/amd64 and linux/arm64, label `org.opencontainers.image.revision`
  `50ba58849b02c5ed532cbdcb72b802dda1116f5a`.
