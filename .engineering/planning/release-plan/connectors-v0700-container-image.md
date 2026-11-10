---
format: aep.planning-md/3
id: release-plan:connectors-v0700-container-image
kind: release-plan
status: draft
title: 'Release 0.70.0: the container image and wave A defect fixes'
tags:
- zero-open-issues
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

Minor release 0.70.0, cut from `main` at 23b9f1bc4 (PR 244, wave A of
epic:zero-open-issues-and-store-defects). The operator approved it on 2026-11-10: "A: tag now from
main (23b9f1bc4). Unblocks a downstream consumer today; the defect wave goes into the following release."

- Each release tag publishes `ghcr.io/beyond10x/connectors:<version>` for `linux/amd64` and
  `linux/arm64`; 0.70.0 is the first tag whose push runs that image job.
- Ten defect fixes, listed in the CHANGELOG's 0.70.0 Fixed section.

Minor, not patch: the image is an addition. No CHANGELOG entry is breaking; a catalog selection
that binds an undeclared or doubly bound key is now refused at load, which the shipped selections
pass.

## Scope

| story | status |
|---|---|
| story:http-empty-query-no-trailing-question-mark | implemented |
| story:bundle-index-refuses-ambiguous-rows | implemented |
| story:provider-programs-run-on-macos | implemented |
| story:endpoint-publish-holds-one-writer | implemented |
| story:connection-next-actions-no-cycle | implemented |
| story:catalog-guard-optional-preflight-values | implemented |
| story:feed-cursor-unadvanced-is-unavailable | implemented |
| story:mysql-query-read-provenance | implemented |
| story:websearch-contract-states-host-deadline | implemented |
| story:owner-read-answer-size-envelope | implemented |

The container image has no story; it reached `main` from branch `build/container-image` in PR 244.

## Evidence

- `main` at 23b9f1bc4: Rust gate run 42780994700, Planning store 42780994724, Documentation
  validation 42780994581, Shared source gates 42780994949, macOS host 42780994558, Windows host
  42780994537, Custody platforms 42780994596: all success. PR 244 head 697f2e216: ten checks green.
- Image dry run on the pull request of `build/container-image` (5effc5700): release-binaries run
  42530964092 success; the workflow and `deploy/images/` are unchanged on `main` since.
- The release pull request changes only the version inputs, the CHANGELOG, the version and image
  claims in README.md, docs/running-services.md and website/docs/getting-started.md, and this
  record. Its CI is the gate for the release commit.

## Not in this release

- The second defect wave of epic:zero-open-issues-and-store-defects: 51 stories in flight on
  `wave/zero-issues-b`, and 21 draft review findings (tags `from-review/cr-NNN`) on that branch.
- Open blockers: credential-blocker:google-oauth-client,
  credential-blocker:live-read-loop-no-configured-connections,
  credential-blocker:sandbox-accounts-jira-confluence-hubspot,
  hardware-blocker:macos-adopter-provider-sigkill,
  upstream-blocker:definitions-lack-agent-projections.
- No story moves with this record. Package-registry, website deployment and Atlas registration
  are outside a release (AGENTS.md, Cutting a release).
