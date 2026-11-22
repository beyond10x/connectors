---
format: aep.planning-md/3
id: approval-record:zero-issues-wave-b
kind: approval-record
status: draft
title: Wave B of the zero-issues epic approved with no operator present
tags:
- non-interactive
relations:
- decides: story:adapter-private-protocol-refusal-named
- decides: story:catalog-guard-typed-comparison
- decides: story:closed-body-load-refuses-ambiguous-pointers
- decides: story:definition-import-merges-shared-auth
- decides: story:definition-refusal-keeps-deprecations
- decides: story:legacy-alias-sharing-an-instance-name
- decides: story:sql-length-units
- decides: story:readme-back-under-its-size-limit
- decides: story:slack-move-names-four-writes
- decides: story:mcp-projection-cases-carry-read-or-mutating
revision: 1
---
## Which stop

The stage-1 proposal of wave B of `epic:zero-open-issues-and-store-defects` (skill aep:implementing,
wave mode), after wave A merged as pull request 244 (`23b9f1bc4`).

## Why it is bypassed: non-interactive

The coordinator is a sub-agent dispatch with no operator turn. The caller, 2026-11-10: "continue
epic:zero-open-issues-and-store-defects with wave B ... Wave B = up to 10 stories, as many as the
collision check allows; pick those with no credential/hardware blocker first ... One new PR for wave B
from a fresh integration branch off origin/main".

## What was decided

- `aep plan artifact waves --kind story --status active` on `23b9f1bc4`: 46 active stories of the
  epic; 44 implementable now, 2 behind open blockers that stay open
  (`hardware-blocker:macos-adopter-provider-sigkill`, `credential-blocker:live-read-loop-no-configured-connections`).
  Derived wave 1 holds 30 mutually non-colliding stories (by their recorded, partly inferred scope),
  later derived waves 9, 3, 1, 1, 1.
- Wave B takes ten stories of derived wave 1 that fix behaviour or a stated contract, preferring
  deterministic defects over load-timing flakes: `story:adapter-private-protocol-refusal-named`,
  `story:catalog-guard-typed-comparison`, `story:closed-body-load-refuses-ambiguous-pointers`,
  `story:definition-import-merges-shared-auth`, `story:definition-refusal-keeps-deprecations`,
  `story:legacy-alias-sharing-an-instance-name`, `story:sql-length-units`,
  `story:readme-back-under-its-size-limit`, `story:slack-move-names-four-writes`,
  `story:mcp-projection-cases-carry-read-or-mutating`.
- Integration branch `wave/zero-issues-b` off origin/main `23b9f1bc4`; one pull request for the wave
  through the bot; the full repository gate once at the wave's end; no tag, no release.
- Builds: one at a time through `b10x-build-slot`, one shared target directory, named test binaries only.
