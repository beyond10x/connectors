---
format: aep.planning-md/1
id: story:ess3-format-adoption
kind: story
status: draft
title: Adopt the ess/3 format across every root under a pinned ESS 0.23
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Acceptance

Every ESS root in this repository declares `format: ess/3` and validates under the
pinned toolchain, `crates/connectors-spec/toolchain.json` selects an ESS release
that accepts `ess/3`, and the full repository gate passes with `--msrv` — so that
the format bump is established by a run rather than by the version strings having
been edited.

The set of roots is re-derived at the time of the work, not copied from this
story: it was seven on 2026-09-12 and the count has changed twice.

## Where this came from

Branch `feat/adopt-ess3`, commit `5038977`, preserved from a managed worktree that
had held it uncommitted since 2026-09-11 and was retired on 2026-09-12. The commit
bumps the pin from ESS 0.22.2 at `6b666e58` to 0.23.0 at `c8023067`, and rewrites
`format: ess/1` to `format: ess/3` in `ess/system.yaml` and six adapter roots.

Eight one-line changes. No content migration, no gate run, no evidence.

## What nobody has established

**What `ess/3` changes beyond the version string.** Nothing in that branch, and
nothing in this repository, says whether a root that declares `ess/3` must also
change its content. A format bump that is only a version string is the cheap case
and it is the one nobody has checked. `UNMAPPED:` until somebody reads the ESS
release.

**Whether the pinned 0.23.0 commit accepts the roots as they stand.** The pin and
the format lines were changed in the same eight edits, so neither was validated
against the other.

**What the bump does to the seven adapter roots' independent compilation.** The
adapter boundary gate compiles each root independently; nothing has run it under
0.23.0.

## Why it is stale as written

`adapters/mcp/spec/ess/` was created on 2026-09-12 by `story:mcp-domain-model`,
declares `format: ess/1`, and is the seventh adapter root. The branch sweeps six.
Replaying those eight files would leave one root behind and the boundary gate
compiles all of them, so the omission would surface as a confusing failure in a
root nobody had touched.

## Scope

- `crates/connectors-spec/toolchain.json` — the single ESS pin
- `ess/system.yaml` — the shared root
- `adapters/*/spec/ess/system.yaml` — every adapter root, count re-derived at the
  time of the work

## Out of scope

Changing what any model declares. This story moves a format and a pin; a model
that has to change its content to satisfy `ess/3` is a finding that belongs to the
root that owns it.
