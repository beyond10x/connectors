---
format: aep.planning-md/3
id: verification-report:connectors-v0741-release-scope
kind: verification-report
status: accepted
title: connectors 0.74.1 release scope
summary: Scope, exclusions and evidence of the 0.74.1 cut
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-22T20:39:03Z", actor: "human:timo", revision: 3}
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

Verified 2026-11-22 after publication:

- Release pull request #273 merged as `28c0dd7a7c07fd7ce2acc660a1095b87918c2d41`; its tree
  `cabbdee5f639` equals the pull request's tree, whose required checks passed.
- Tag `v0.74.1` by `b10x-bot[bot]` at 18:56 CET; it peels to the merge commit, which is an
  ancestor of `origin/main`.
- Release page https://github.com/beyond10x/connectors/releases/tag/v0.74.1, author
  `b10x-bot[bot]`, not a draft, Latest.
- Release run 45052366789: all 7 jobs succeeded, no rerun.
- The page's 5 assets: the 4 archives pass `sha256sum -c SHA256SUMS`.
- Image `ghcr.io/beyond10x/connectors:0.74.1`, index
  `sha256:fa30f6b9500e7c03fe08090ca9c76e0096a2da589a06f55217a6076ff99ec4d9`, platforms
  linux/amd64 and linux/arm64, label `org.opencontainers.image.revision`
  `28c0dd7a7c07fd7ce2acc660a1095b87918c2d41`.
