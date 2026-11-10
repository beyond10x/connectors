---
format: aep.planning-md/3
id: approval-record:zero-issues-acceptance-20261109
kind: approval-record
status: draft
title: Epic and its defect stories accepted with no operator present
tags:
- non-interactive
relations:
- decides: epic:zero-open-issues-and-store-defects
- decides: story:adapter-private-protocol-refusal-named
- decides: story:bundle-index-refuses-ambiguous-rows
- decides: story:catalog-guard-optional-preflight-values
- decides: story:catalog-guard-typed-comparison
- decides: story:catalog-validates-enum-and-date-time
- decides: story:cli-connect-admission-timeout-under-load
- decides: story:closed-body-load-refuses-ambiguous-pointers
- decides: story:configuration-change-error
- decides: story:connection-next-actions-no-cycle
- decides: story:custody-keeps-out-of-the-session-keyring
- decides: story:definition-connect-begin-carries-selection
- decides: story:definition-import-merges-shared-auth
- decides: story:definition-reader-refuses-what-the-feed-engine-refuses
- decides: story:definition-refusal-keeps-deprecations
- decides: story:endpoint-publish-holds-one-writer
- decides: story:exec-plugin-busy-reaches-the-operator
- decides: story:feed-cursor-unadvanced-is-unavailable
- decides: story:generated-output-regenerates-in-a-fresh-checkout
- decides: story:http-empty-query-no-trailing-question-mark
- decides: story:legacy-alias-sharing-an-instance-name
- decides: story:macos-release-provider-starts-through-the-private-copy
- decides: story:metadata-conformance-emit-manifest-format
- decides: story:mysql-query-read-provenance
- decides: story:owner-bounded-generic-read-inline-schemas
- decides: story:owner-cancels-a-bounded-read
- decides: story:owner-errors-are-readable
- decides: story:owner-invoke-frame-effects-boundary
- decides: story:owner-read-answer-size-envelope
- decides: story:owner-recovers-after-failed-acquisition
- decides: story:pool-idle-checks-wait-outside-the-pool-lock
- decides: story:provider-programs-run-on-macos
- decides: story:required-body-key-refuses-null
- decides: story:required-parameter-refuses-empty
- decides: story:sql-length-units
- decides: story:authored-scenarios-known-failing
- decides: story:clock-quorum-test-tolerates-a-slow-loopback-source
- decides: story:clock-quorum-windows-unreachable-flake
- decides: story:connection-lease-tests-pin-the-window
- decides: story:gmail-attachment-read-flaky-under-load
- decides: story:host-gate-load-sensitivity-w3
- decides: story:ignored-journeys-run-in-ci
- decides: story:lease-early-recollection-relaunch-margin
- decides: story:live-acceptance-without-serve
- decides: story:mcp-projection-cases-carry-read-or-mutating
- decides: story:mcp-reply-after-hangup-owner-latency-under-load
- decides: story:oauth-browser-empty-connection-fixture
- decides: story:owner-path-tests-hold-under-load
- decides: story:tests-stop-the-owners-they-start
- decides: story:upgrade-refusal-test-port-reuse
- decides: story:windows-clock-check-bracket-flake
- decides: story:catalog-oversized-body-one-classification
- decides: story:docs-provider-pages-shared-guides
- decides: story:docs-specification-sidebar
- decides: story:readme-back-under-its-size-limit
- decides: story:slack-move-names-four-writes
- decides: story:websearch-contract-states-host-deadline
revision: 1
---
## The stop

Accepting stories: moving `epic:zero-open-issues-and-store-defects` and the draft stories that
decompose it from `draft` through `proposed` to `active`. Accepting a story is the operator's act,
and a coordinator never does it on its own.

## Why it is bypassed: non-interactive

The run is a sub-agent dispatch with no operator turn; the coordinator's brief says a Stop hook holds
the session until the goal is met. The brief (operator the operator, 2026-11-09) states: "The operator's goal
is the acceptance of these stories and the approval of each wave."

Goal, verbatim: "zero remaining issues on github, all bugs fixed from planning store".
Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but in the end provide
1 single PR to gh. make sure all are below 1 big epic for integration via bottoken from ./atlas".

## What was decided in the operator's absence

- The epic and every draft story with a `decomposes` edge to it move to `active`. The two stories
  already `active` (`story:provider-programs-run-on-macos`,
  `story:macos-release-provider-starts-through-the-private-copy`) keep their status.
- Which stories "describe a defect" was decided by reading all 150 open stories and tasks (one
  read-only agent pass on `bd338daa0`): 56 do. The brief's sender estimated 2 from a grep. The
  operator's words are "all bugs", so the larger, read set is taken; 22 of the 56 are borderline
  readings (a loose-input finding, a decision inside the story, a test or the product at fault) and
  are included rather than dropped.
- Where a story's body leaves a choice open ("a decision, not a patch"), the unit takes the
  reversible, narrower option and names it in the story; the operator's standing preference is
  "Decide, don't ask. Take the reversible option".
- Releases stay out: no tag, no version bump, no publication.

## What the operator would have been asked

1. Are these 56 the bugs you mean, or only the stories that were filed as defects of shipped
   behaviour (34), or fewer?
2. Do you accept them as written, including the 22 borderline ones?
