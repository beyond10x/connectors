---
format: aep.planning-md/3
id: decision-blocker:checkpoint-offline-edit-detection
kind: decision-blocker
status: cleared
title: Nobody has decided whether checkpointed opens may skip detecting offline file edits
relations:
- blocks: story:metadata-invoke-cost-flat-in-store-size
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-07T17:26:05Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"approval":1}}}
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

## Decision — 2026-10-07

Decided 2026-10-07: option B, under conditions that keep the integrity guarantee unchanged, so
no weaker guarantee is accepted.

1. Only the owner, holding a handle for its whole run, may open from a checkpoint without full
   verification, and it performs one FullVerification open when it starts.
2. Every other open (a command run without the owner, a second process, recovery) keeps a
   FullVerification open, as today.
3. Tests: an offline raw edit of the SQLite file is refused before the first command after it,
   once on the owner path (edited while the owner is stopped, then started) and once on the
   direct path.
4. `CHANGELOG.md` and the store contract say which open verifies what.

A path that cannot meet 1–3 stops the story and goes back as a new decision.
