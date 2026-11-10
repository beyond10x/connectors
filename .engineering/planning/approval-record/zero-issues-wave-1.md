---
format: aep.planning-md/3
id: approval-record:zero-issues-wave-1
kind: approval-record
status: draft
title: Wave 1 of the zero-issues plan approved with no operator present
tags:
- non-interactive
relations:
- decides: story:adapter-private-protocol-refusal-named
- decides: story:authored-scenarios-known-failing
- decides: story:bundle-index-refuses-ambiguous-rows
- decides: story:catalog-guard-optional-preflight-values
- decides: story:catalog-oversized-body-one-classification
- decides: story:cli-connect-admission-timeout-under-load
- decides: story:clock-quorum-test-tolerates-a-slow-loopback-source
- decides: story:closed-body-load-refuses-ambiguous-pointers
- decides: story:connection-lease-tests-pin-the-window
- decides: story:custody-keeps-out-of-the-session-keyring
- decides: story:definition-import-merges-shared-auth
- decides: story:definition-refusal-keeps-deprecations
- decides: story:docs-provider-pages-shared-guides
- decides: story:docs-specification-sidebar
- decides: story:endpoint-publish-holds-one-writer
- decides: story:feed-cursor-unadvanced-is-unavailable
- decides: story:generated-output-regenerates-in-a-fresh-checkout
- decides: story:gmail-attachment-read-flaky-under-load
- decides: story:host-gate-load-sensitivity-w3
- decides: story:http-empty-query-no-trailing-question-mark
- decides: story:ignored-journeys-run-in-ci
- decides: story:legacy-alias-sharing-an-instance-name
- decides: story:live-acceptance-without-serve
- decides: story:mcp-projection-cases-carry-read-or-mutating
- decides: story:mcp-reply-after-hangup-owner-latency-under-load
- decides: story:metadata-conformance-emit-manifest-format
- decides: story:mysql-query-read-provenance
- decides: story:oauth-browser-empty-connection-fixture
- decides: story:owner-bounded-generic-read-inline-schemas
- decides: story:owner-read-answer-size-envelope
- decides: story:pool-idle-checks-wait-outside-the-pool-lock
- decides: story:provider-programs-run-on-macos
- decides: story:readme-back-under-its-size-limit
- decides: story:slack-move-names-four-writes
- decides: story:tests-stop-the-owners-they-start
- decides: story:upgrade-refusal-test-port-reuse
- decides: story:websearch-contract-states-host-deadline
revision: 1
---
## Which stop

The stage-1 proposal of wave 1 of the plan in `epic:zero-open-issues-and-store-defects` § Wave plan
(skill aep:implementing 0.26.0, wave mode).

## Why it is bypassed: non-interactive

The coordinator is a sub-agent dispatch with no operator turn; its brief (operator the operator, 2026-11-09)
says: "The operator's goal is the acceptance of these stories and the approval of each wave." Goal,
verbatim: "zero remaining issues on github, all bugs fixed from planning store".

## What was decided

- Wave 1 runs the 37 stories listed under § Wave 1 of the epic, as `aep plan artifact waves --kind
  story --status active` placed them; `story:macos-release-provider-starts-through-the-private-copy`
  left it (open `hardware-blocker:macos-adopter-provider-sigkill`).
- Check path: the verb (aep 0.79.0), over typed scope written by the earlier coordinator; most
  entries are `inferred`, so each implementor confirms its surface before building on it.
- At most two sub-agents at a time (operator limit for this repository), every one on model opus
  (operator: "make sure to use non-fable sub-agents"). Implementor `aep:implementor`, then
  `aep:adversary` per unit. Units fork from the integration branch head at dispatch, so two units
  never share a tree and a later unit sees every earlier merge.
- Pre-flight: free disk 71 GB of 848 GB (92 %); one build of host and CLI test binaries 191 s and
  2.9 GB with sccache on port 4301; host load about 35 on 20 cores.
- `story:provider-programs-run-on-macos` (#229) is tested on the pull request's macOS runner, so the
  pull request is opened as a draft before the first unit merges.

## Commits this bypass authorises

One commit per unit on its branch, the merges into `wave/zero-issues`, the store commits that record
evidence and moves, pushes of `wave/zero-issues` through `b10x-gates bot`, and the one pull request
from it to `main`. No tag, no version bump, no release.

## What the operator would have been asked

Approve wave 1 as listed, at two concurrent units.
