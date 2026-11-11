---
format: aep.planning-md/3
id: story:owner-cancel-frees-queue-slot
kind: story
status: draft
title: A cancelled owner job frees its queue slot at once
tags:
- from-finding
relations:
- informed_by: review-result:owner-cancel-adversary-r1-20261111
revision: 1
---
## Finding

A cancelled queued owner job keeps its place in the instance's 16-slot queue
(`crates/connectors-host/src/local/owner/supervisor.rs:702`, `sync_channel(16)`) until the worker
reaches it and drops it (`supervisor.rs:1105`). Behind one long running read, cancelled jobs can
fill the queue, and another read is then refused `capacity` although nothing live is waiting.

Found by review-result:owner-cancel-adversary-r1-20261111. The owner contract was reworded to
state the slot is held until the worker reaches the job.

## Outcome

A cancel frees the cancelled job's queue slot at once.

## Acceptance

- With one long read running and the queue filled by jobs that are then cancelled, a new read is
  admitted, not refused `capacity`; the test uses at most the queue's capacity plus two sessions.
- owner.md states that a cancel frees the slot.
