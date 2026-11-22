---
format: aep.planning-md/3
id: approval-record:zero-issues-single-defect-wave
kind: approval-record
status: draft
title: The single defect wave of the zero-issues epic approved with no operator present
tags:
- non-interactive
relations:
- decides: story:adapter-private-protocol-refusal-named
- decides: story:authored-scenarios-known-failing
- decides: story:catalog-guard-typed-comparison
- decides: story:catalog-oversized-body-one-classification
- decides: story:catalog-validates-enum-and-date-time
- decides: story:cli-connect-admission-timeout-under-load
- decides: story:clock-quorum-windows-unreachable-flake
- decides: story:clock-quorum-test-tolerates-a-slow-loopback-source
- decides: story:closed-body-load-refuses-ambiguous-pointers
- decides: story:connection-lease-tests-pin-the-window
- decides: story:definition-connect-begin-carries-selection
- decides: story:custody-keeps-out-of-the-session-keyring
- decides: story:definition-import-merges-shared-auth
- decides: story:docs-provider-pages-shared-guides
- decides: story:definition-reader-refuses-what-the-feed-engine-refuses
- decides: story:docs-specification-sidebar
- decides: story:generated-output-regenerates-in-a-fresh-checkout
- decides: story:exec-plugin-busy-reaches-the-operator
- decides: story:gmail-attachment-read-flaky-under-load
- decides: story:host-gate-load-sensitivity-w3
- decides: story:ignored-journeys-run-in-ci
- decides: story:legacy-alias-sharing-an-instance-name
- decides: story:lease-early-recollection-relaunch-margin
- decides: story:live-acceptance-without-serve
- decides: story:mcp-projection-cases-carry-read-or-mutating
- decides: story:metadata-conformance-emit-manifest-format
- decides: story:oauth-browser-empty-connection-fixture
- decides: story:owner-bounded-generic-read-inline-schemas
- decides: story:owner-cancels-a-bounded-read
- decides: story:owner-path-tests-hold-under-load
- decides: story:owner-invoke-frame-effects-boundary
- decides: story:owner-recovers-after-failed-acquisition
- decides: story:pool-idle-checks-wait-outside-the-pool-lock
- decides: story:readme-back-under-its-size-limit
- decides: story:required-parameter-refuses-empty
- decides: story:configuration-change-error
- decides: story:slack-move-names-four-writes
- decides: story:required-body-key-refuses-null
- decides: story:sql-length-units
- decides: story:mcp-reply-after-hangup-owner-latency-under-load
- decides: story:definition-refusal-keeps-deprecations
- decides: story:tests-stop-the-owners-they-start
- decides: story:owner-errors-are-readable
- decides: story:upgrade-refusal-test-port-reuse
- decides: story:windows-clock-check-bracket-flake
- decides: story:slack-file-read-checks-exact-path
- decides: story:tavily-crawl-cut-short-is-not-complete
revision: 1
---
## Which stop

The stage-1 proposal of the single defect wave of `epic:zero-open-issues-and-store-defects` (skill
aep:implementing, wave mode), after wave A merged as pull request 244 (`23b9f1bc4`). It replaces
`approval-record:zero-issues-wave-b`, whose ten stories are part of this wave.

## Why it is bypassed: non-interactive

The coordinator is a sub-agent dispatch with no operator turn. The operator's order, relayed by the
caller on 2026-11-10, verbatim: "connectors -> ok good - lets at least fix all bugs/defects and
dispatch them as another single wave".

## What was decided

- Scope: every remaining defect story of the epic: the 46 active stories under
  `epic:zero-open-issues-and-store-defects`, and the two drafts filed from wave A findings,
  `story:slack-file-read-checks-exact-path` and `story:tavily-crawl-cut-short-is-not-complete`.
  No feature stories, no other epics.
- One stays behind its open blocker: `story:macos-release-provider-starts-through-the-private-copy`
  (`hardware-blocker:macos-adopter-provider-sigkill`). 47 stories are implemented in this wave.
- One wave, one integration branch `wave/zero-issues-b` off origin/main `23b9f1bc4`, one pull request
  through the bot. The ten-story cap does not apply. Stories that collide on a file run serially in
  the order of the derived waves of `aep plan artifact waves --kind story --status active` (lanes),
  never as further waves or pull requests.
- A story the defect no longer reproduces on is proven with a test or command output and moved with
  that evidence.
- Builds: one at a time through `b10x-build-slot`, one shared target directory, named test binaries
  only; the full repository gate once at the end; CI once on the pull request; no tag, no release.
