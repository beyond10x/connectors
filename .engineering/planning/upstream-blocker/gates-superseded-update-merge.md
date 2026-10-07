---
format: aep.planning-md/3
id: upstream-blocker:gates-superseded-update-merge
kind: upstream-blocker
status: open
title: Gates refuses a branch update that a later update of the same pull request superseded
relations:
- blocks: specification:wave-20261007a-feed-gitlab
revision: 1
---
## What stops delivery

Pushing `wave/20261007a` on 2026-10-07 was refused by the Gates pre-push hook (0.1.14):
`b10x-gates: updated pull request summary does not bind this accepted head`.

Gates walks every commit from the enrolled baseline to the pushed head. A GitHub-committed
commit that is not a pull request's terminal merge must be a branch update whose merged pull
request still has it as its head (`beyond10x/gates` `src/published_merge.rs`,
`verify_update_branch`). `6a0fa5a5d`, GitHub's first "update branch" merge of pull request
#126, no longer is: #126 was updated again (`f7efee0f9`) to satisfy strict status checks and
then merged (`8d511078a`). Both are on `main`, so every push whose range includes `main` is
refused.

## What would clear it

A Gates release that admits an update merge superseded by a later update of the same pull
request (the final head descends from it, and each update between is GitHub-committed and
associated with that pull request), installed in this repository with `b10x-gates install`.
