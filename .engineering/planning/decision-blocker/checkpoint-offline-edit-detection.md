---
format: aep.planning-md/3
id: decision-blocker:checkpoint-offline-edit-detection
kind: decision-blocker
status: open
title: Nobody has decided whether checkpointed opens may skip detecting offline file edits
relations:
- blocks: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
## The question

Entity Runtime 0.29.0: "A ProviderTracked open from a checkpoint no longer detects raw edits of
the SQLite file that bypass SQLite while no handle is open: only a FullVerification open or a
complete read does." This story recorded on 2026-09-29 that integrity checks stay as they are,
and the checkpoint decision enables checkpoints on every new store. Is the gap accepted, and
with what compensation?

| option | what happens | cost |
|---|---|---|
| A | accept; `setup check` performs a FullVerification open; contract and CHANGELOG state the gap | offline edits are caught only when someone runs `setup check` |
| B | A, plus the owner performs one FullVerification open when it starts | one whole-store verify per owner start; per-command cost stays flat |
| C | no checkpoints | #101 stays unfixed |

## What would clear it

A decision naming A, B or C.
