---
format: aep.planning-md/3
id: upstream-blocker:entity-runtime-on-eventlog-081
kind: upstream-blocker
status: cleared
title: Entity Runtime has no release on Eventlog 0.8.1 yet
relations:
- blocks: story:entity-runtime-eventlog-081-pin
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T04:08:38Z", actor: "human:timo", revision: 3}
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

## Cleared

Cleared 2026-10-08: Entity Runtime 0.30.1 is released
(https://github.com/beyond10x/entity-runtime/releases/tag/0.30.1, published 2026-10-08T04:05:42Z
by b10x-bot[bot], read with `gh release view`). Its `crates/entity-eventlog/Cargo.toml` pins
`eventlog-core`, `eventlog-sqlite` and the other Eventlog crates at tag 0.8.1.
