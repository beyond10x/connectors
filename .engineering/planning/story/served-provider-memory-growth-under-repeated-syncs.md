---
format: aep.planning-md/3
id: story:served-provider-memory-growth-under-repeated-syncs
kind: story
status: draft
title: The served catalog provider's memory does not grow without bound under repeated full syncs
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Report

From a downstream deployment (session assembly-dev, 2026-11-11 UTC, read with kubectl and the
caller's log): image `ghcr.io/beyond10x/connectors:0.70.0`, one container serving a GitLab
instance, memory limit 3Gi, request 1536Mi.

- 19:05Z start; 21:12Z 1128Mi, no restart.
- The caller runs full syncs of 4 repositories through the instance: `project.get`, a tree listing,
  then `file.get` per file. Syncs 21:15:46Z to 21:28:51Z and from 21:45:29Z.
- 21:59:09Z exit 137, OOMKilled, 13 min 40 s into the second sync.
- After restart, one full sync (22:31:27Z to 22:35:59Z) left 971Mi at 1m CPU; not returned within
  the minutes sampled.

Not determined downstream: which operation holds the memory, response sizes, parallel reads.

## Findings in code (v0.70.0; unchanged through 3bca0b3e0db)

- The image runs `connectors-catalog-provider` in-process (`deploy/images/connectors/Dockerfile:14`,
  `adapters/catalog/src/served.rs:219-240`). A provider response is buffered whole, up to
  `RESPONSE_LIMIT` 4 MiB (`crates/connectors-core/src/lib.rs:13`), with up to 32 calls in flight
  (`crates/connectors-host/src/server.rs:325`).
- No per-call or per-input cache that is never evicted was found in the serve path. JSON schema
  validators are compiled on every call (`crates/connectors-sdk/src/lib.rs:314-320`): churn, not
  accumulation.
- The served path runs on glibc malloc (distroless `cc-debian13`, `*-unknown-linux-gnu`) and does
  not bound arenas. The owner does, after #103: `bound_allocator` sets `M_ARENA_MAX` 1
  (`crates/connectors-host/src/local/owner/transport.rs:1566-1577`). Inferred, not shown: tokio
  workers and `spawn_blocking` threads each hold an arena that keeps freed multi-MiB buffers.
- No flag or environment variable bounds a body, cache or memory; the serve path reads only
  `RUST_LOG` and `--config`. No metrics or heap-profile endpoint.

## Acceptance

- The growth is reproduced against a recorded GitLab fixture in CI (not on the shared
  workstation): RSS after N repeated full syncs of a fixed tree is measured, and the cause is named
  (allocator retention or a held allocation).
- After the fix, RSS after 10 repeated syncs stays within a stated bound of RSS after the first.
