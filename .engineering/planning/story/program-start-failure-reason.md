---
format: aep.planning-md/3
id: story:program-start-failure-reason
kind: story
status: implemented
title: instances status names why a program did not start and keeps its stderr
summary: 'D370 item 3: start failure reason and log file'
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-02T07:27:31Z", actor: "human:timo", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "proposed", to: "active", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 3, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "active", to: "implemented", at: "2026-11-02T15:48:05Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
# Story: program-start-failure-reason

## Outcome

When the owner cannot start an instance's program, `instances status` says which step failed and
where the program's own error output was kept, on Linux and macOS.

## Context

D370, item 3; https://github.com/beyond10x/connectors/issues/233. A failed start answers
`unavailable` at `readiness`, and `instances status` shows only `state: failed`. The owner sends the
child's stderr to `/dev/null` (`crates/connectors-host/src/local/runtime/process.rs:84`), and
`Child::spawn_until` turns every failure into `Failure::Unavailable` with no cause (`process.rs:56-200`).

## Acceptance

- A failed start records one `start_failure` value from a bounded set: `admission`, `digest`, `copy`,
  `spawn`, `exited_before_handshake` (with the exit status or signal), `handshake_deadline`,
  `handshake_mismatch`. `instances status --output json` prints it for that instance.
- The child's stderr is captured, at most 64 KiB, into an owner-only (0600) file under the state
  directory, one per instance, replaced on each start. `instances status` names its path
  as `start_log` when the last start failed.
- The file and the status never hold credential material: the capture covers only bytes the child
  wrote before its handshake finished, and stderr is closed for the child once it is ready.
- Named tests: a program that prints a line to stderr and exits 3 before its handshake shows
  `exited_before_handshake`, exit 3, and the line in `start_log`; a digest mismatch shows `digest`.

## Out of Scope

- Streaming a running program's logs.
