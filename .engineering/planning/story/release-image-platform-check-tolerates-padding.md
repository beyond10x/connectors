---
format: aep.planning-md/3
id: story:release-image-platform-check-tolerates-padding
kind: story
status: active
title: The release image job checks platforms whatever the inspect padding, and a rerun accepts its own published image
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: .github/workflows/release-binaries.yml
- confidence: inferred
  path: AGENTS.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T12:50:59Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-10T12:51:01Z", actor: "human:timo", revision: 4, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

Release 0.70.0, run 42814664625 attempt 2, job 129838474619 (image job of
`.github/workflows/release-binaries.yml`): the image was built and pushed
(`ghcr.io/beyond10x/connectors:0.70.0`, index digest
`sha256:00606821462e2324e129f799511c94ca4092ae8ab1535b8ba6519a429036ee4b`, equal to the build's digest), and
the job then failed in "The pushed image and its digest": lines 401-402 grep
`Platform:  linux/amd64` / `Platform:  linux/arm64` with two spaces, while `docker buildx imagetools inspect`
prints `Platform:    linux/amd64` (four spaces). The pull-request path never runs this step, so nothing
caught it before a release.

Rerun behaviour: attempt 1 (job 129831288807) failed at the push with `denied: permission_denied:
write_package` (the package had not yet granted this repository Actions write access; the operator added
`beyond10x/connectors` with Write under the package's "Manage Actions access" on 2026-11-10). So when
attempt 2 ran "Refuse a version the registry already holds", 0.70.0 was not in the registry and the step
passed. A further rerun of this release would now fail at that step, because the version exists, although
the registry holds exactly what this commit builds.

## Acceptance

- The platform check matches the inspect output whatever its padding (for example
  `grep -Eq '^[[:space:]]*Platform:[[:space:]]+linux/amd64$'`), and the digest equality check stays.
- The parse lives in one script the release step calls, and the pull-request path of the workflow runs
  the same script over a captured fixture of real `imagetools inspect` output (the 0.70.0 output); a red
  run with the old two-space pattern against that fixture is the proof.
- A rerun of a release whose version is already in the registry does not fail when the registry's image
  is the one this commit builds (its `org.opencontainers.image.revision` label, or its digest, matches),
  and still refuses one that differs; it pushes nothing in either case.
- `AGENTS.md` § Cutting a release states the precondition that the `connectors` container package grants
  this repository Write under "Manage Actions access" before the first push.
