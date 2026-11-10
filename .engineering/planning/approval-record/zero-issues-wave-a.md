---
format: aep.planning-md/3
id: approval-record:zero-issues-wave-a
kind: approval-record
status: draft
title: Wave A of the zero-issues epic approved with no operator present
tags:
- non-interactive
relations:
- decides: story:provider-programs-run-on-macos
- decides: story:connection-next-actions-no-cycle
- decides: story:http-empty-query-no-trailing-question-mark
- decides: story:bundle-index-refuses-ambiguous-rows
- decides: story:endpoint-publish-holds-one-writer
- decides: story:catalog-guard-optional-preflight-values
- decides: story:feed-cursor-unadvanced-is-unavailable
- decides: story:websearch-contract-states-host-deadline
- decides: story:mysql-query-read-provenance
- decides: story:owner-read-answer-size-envelope
revision: 1
---
## Which stop

The stage-1 proposal of wave A of `epic:zero-open-issues-and-store-defects` (skill aep:implementing
0.26.0, wave mode), on resuming after the operator's wind-down was reversed.

## Why it is bypassed: non-interactive

The coordinator is a sub-agent dispatch with no operator turn. Operator, 2026-11-10, relayed by the
caller: "do work on llm, inference, connectors - start 3 opus 5.5 /aep:implementing agents - each
processing a whole wave of up to 10 issues (the more the better) - run no expensive gates in the
middle, only at the end of the wave". The caller named the wave content: the nine issues, the two
finished units, the story of the gate failure `concurrent_endpoints_on_one_instance_agree_with_the_file`
and the miswired macOS custody test.

## What was decided

- The plan is re-cut into waves of at most ten (brief rule 12). Wave A holds ten stories:
  `story:provider-programs-run-on-macos` (#229, with the custody launch-copy test that never reached
  its child on CI run 42713022257), `story:connection-next-actions-no-cycle` (#181),
  `story:http-empty-query-no-trailing-question-mark` and `story:bundle-index-refuses-ambiguous-rows`
  (implemented on `impl/*` branches, adversary pass outstanding),
  `story:endpoint-publish-holds-one-writer` (the gate failure on PR 243 run 42713022361),
  `story:catalog-guard-optional-preflight-values`, `story:feed-cursor-unadvanced-is-unavailable`,
  `story:websearch-contract-states-host-deadline`, `story:mysql-query-read-provenance`,
  `story:owner-read-answer-size-envelope`.
- The seven issues already fixed on `main` (#180, #183, #230, #231, #232, #233, #234) need no unit;
  their proof is recorded and they are closed through the bot once the pull request carrying it merges.
- Check path: `aep plan artifact waves --kind story --status active --format json` (aep 0.79.0)
  printed no collision between any two of the ten.
- Implementors `aep:implementor` on sonnet, adversaries `aep:adversary` on opus; at most four
  sub-agents at once. Each implementor runs only its package's red and green case; the repository
  gate runs once on `wave/zero-issues` after every merge, and CI once on the one pull request.
- Pre-flight: free disk 59 GB (94 %); trees are finished with `--discard-cache` as soon as a unit
  merges.

## Commits this bypass authorises

One commit per unit on `impl/<slug>`, the merges into `wave/zero-issues`, the store commits that
record reviews, evidence and moves, pushes through `b10x-gates bot`, and one pull request to `main`
merged through the bot. No tag, no version bump, no release.

## What the operator would have been asked

Approve wave A as listed, at four concurrent sub-agents.
