---
format: aep.planning-md/3
id: story:catalog-selection-fixed-body-value
kind: story
status: draft
title: A catalog selection fixes a body member to one value
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:fluxplane-plugin-parity
revision: 1
---
## Outcome

A catalog selection can fix a request body member to one value, so that a guarded variant of an
existing write sends exactly that value and the caller cannot change it.

## Why

`story:parity-gitlab-mr-writes` left two GitLab variants unselected:

- merge when the pipeline succeeds: `putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge` with
  `auto_merge: true` (the pinned document marks `merge_when_pipeline_succeeds` deprecated). Without
  the value fixed, the same request merges at once, and this variant's guard deliberately skips the
  succeeded-pipeline check. Its postflight also needs to accept one of two answers (auto-merge set,
  or state `merged` when the pipeline had already passed).
- reopen: `putApiV4ProjectsIdMergeRequestsMergeRequestIid` with exactly `state_event: reopen`,
  preflight state `closed` and the pinned sha, postflight state `opened`.

`gitlab.mr.merge` and `gitlab.mr.update` stay partial on the parity page until then.

## Acceptance

- Spec first: the fixed body member, and a postflight check that accepts one of several values,
  are modelled in the catalog selection format and its ESS model, validated with the newest `ess`,
  before the engine changes.
- Both GitLab variants answer through `connectors operations invoke` against a fixture; a caller
  value for a fixed member is refused before any request.
- Existing selections keep their bytes and their configuration revisions.
