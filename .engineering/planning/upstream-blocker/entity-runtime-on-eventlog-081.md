---
format: aep.planning-md/3
id: upstream-blocker:entity-runtime-on-eventlog-081
kind: upstream-blocker
status: open
title: Entity Runtime has no release on Eventlog 0.8.1 yet
relations:
- blocks: story:entity-runtime-eventlog-081-pin
revision: 1
---
## What blocks

story:entity-runtime-eventlog-081-pin moves the Entity Runtime pin to the release that builds
against Eventlog 0.8.1. That release does not exist yet: it is the Entity Runtime story
`story:entity-runtime-builds-against-eventlog-0-8-1` (active) in
https://github.com/beyond10x/entity-runtime. This store cannot hold an edge to another
repository's artifact, so this blocker stands for that `depends_on`.

## Clears when

Entity Runtime publishes that release as a tag and a GitHub Release on
https://github.com/beyond10x/entity-runtime/releases.
