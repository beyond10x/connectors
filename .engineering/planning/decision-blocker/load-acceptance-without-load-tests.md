---
format: aep.planning-md/3
id: decision-blocker:load-acceptance-without-load-tests
kind: decision-blocker
status: open
title: A load-30 gate acceptance cannot run where load tests are forbidden
relations:
- blocks: story:host-gate-load-sensitivity-w3
withholds: test_result
revision: 1
---
## Question

story:host-gate-load-sensitivity-w3 accepts on "each named test passes the full gate at load 30 or
more". Timo forbids load tests on the shared workstation (relayed by session brain, 2026-11-11),
and no other host is named for it.

## What would clear it

One of: a host or CI runner where the gate may run under load 30, or a changed acceptance line
(for example, each named test bounds every wait it makes, shown by reading and a deterministic
seam), decided by Timo.
