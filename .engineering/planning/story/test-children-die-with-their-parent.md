---
format: aep.planning-md/3
id: story:test-children-die-with-their-parent
kind: story
status: active
title: A killed test leaves no owner, adapter or keyring running
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-11T19:51:16Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-11T19:51:17Z", actor: "human:timo", revision: 3}
---
## Finding

Test fixtures and owners are reaped only in `Drop` (crates/connectors-host reap.rs and the fixture
copies). A test process killed with SIGKILL, or a build slot timeout, runs no `Drop`, so its owner,
adapter and keyring children keep running; on 2026-11-11 Timo saw orphaned connectors processes at
100% CPU on the shared workstation.

## Outcome

On Linux every child a test or an owner starts asks the kernel to end it when its parent dies
(`PR_SET_PDEATHSIG` with SIGKILL, set in the child before exec), so a killed test leaves no
process behind.

## Acceptance

- A test that starts an owner and an adapter through the usual fixture, then SIGKILLs its own
  harness process, finds within 2 s that no process of that fixture remains.
- The non-Linux paths are unchanged and still build (macOS and Windows CI lints).
