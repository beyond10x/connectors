---
format: aep.planning-md/3
id: decision-blocker:consumer-launch-design
kind: decision-blocker
status: open
title: The consumer launch needs three design decisions before it can be built
relations:
- blocks: story:launch-consumer-with-connection-credential
revision: 1
---
## What stops the story

The scoped tree does not support the story's Work as written (see the story's "Open design questions"). Three are design decisions for this repository's owners: how a launch obtains the binding without starting the adapter, which process may read the connection credential, and how the CLI takes a trailing argument list (filed upstream as beyond10x/ess#466).

## What would clear it

A recorded decision for each of the three, written into the story's Work and the custody and CLI contracts it names.
