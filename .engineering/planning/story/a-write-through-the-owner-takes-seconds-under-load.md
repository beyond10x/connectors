---
format: aep.planning-md/3
id: story:a-write-through-the-owner-takes-seconds-under-load
kind: story
status: draft
title: A write that reaches the owner takes 7 to 12 s at host load 56 to 71
summary: Per-phase time of one refused and one admitted write through a running owner is measured; the largest phase is named
revision: 1
---
## Problem

Measured 2026-11-23 in the journey of
story:local-invoke-admits-a-keyed-write-under-a-standing-grant (one unsplit run, 196 s, host
load 56 to 71 on 20 CPUs, debug build, fake Slack API on loopback): every `connectors invoke`
of a write that reaches the owner process takes 7 to 12 s, also when the owner refuses it at
admission and no provider request is made. A command the CLI answers alone takes about 1 s.
The fixture (setup, connect, key and policy) takes 18.6 s.

Per step of that run, in seconds: a 9.8, b 1.8, c 12.1, d 4.6, e 3.8, f 15.0, g 11.3, h 15.5,
i 20.4, j 35.8, k 11.7, l 32.7, the empty proof 3.3.

Not known: where the time goes (owner start, store verification, the ledger, the key lease,
process starts of the provider), how much of it is the debug build, and the time at a host
load under 20.

## Why it matters

- The five journey tests of the standing grant ran 59 to 90 s at load 75 to 97; a test may run
  60 s.
- An unattended agent that writes under a standing grant pays this time per write.

## Outcome

The time of one refused and one admitted write through a running owner is measured per phase
on a release build at a stated host load, the largest phase is named, and either it is reduced
or the story says why it cannot be.

## Acceptance

1. A measurement names, for one write refused at admission and one admitted write through a
   running owner, the wall time per phase (CLI start, owner connect, admission, prepare,
   dispatch, ledger write, answer), release build, with the host load.
2. The same two writes on a debug build, so the share of the build profile is known.
3. If one phase is over half of the total, a change reduces it and the measurement is
   repeated; if not, the story records the numbers and is closed without a change.
