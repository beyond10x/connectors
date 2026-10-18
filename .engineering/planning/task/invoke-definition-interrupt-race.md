---
format: aep.planning-md/3
id: task:invoke-definition-interrupt-race
kind: task
status: implemented
title: An interrupt while a definition is prepared is caught before any child starts
summary: 'Found on macOS CI (run 39130039078): security_an_interrupt_while_a_definition_is_prepared_is_exit_130 ended by the default SIGINT action. Its provider stand-in ran exec /usr/bin/sleep, which macOS does not have, so the stand-in exited at once, pinning failed fast and the preparation returned and dropped its handler before the signal arrived. The handler was also installed only after the configuration read, which can start the installed provider to derive an unpinned instance. Fix: install the handler first in the definition preparation, end any refusal or result as interrupted when an interruption arrived during it, and use /bin/sleep in the fixture. Acceptance: the test passes 200 runs in a row under CPU load on Linux and on macOS CI.'
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- depends_on: story:invoke-definition-file
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T13:50:58Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-18T13:50:58Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-18T14:06:47Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":2}}}
---
<!-- Starting point for a `task` artifact, seeded by `aep artifact new task <name>`.
     No frontmatter here on purpose: the `---` block is written by the CLI from the id, kind, status
     and relations you gave it, and a second copy in this file would be the one that went stale.
     Delete the italic guidance as you fill each section. -->

# Task: <name>

## What

*The concrete unit of work, in the active voice. One task, one thing — if this needs an "and", it is
two tasks or it is a story.*

## Why

*Which story or specification this serves. The `decomposes` or `implements` relation carries the
edge; this says what would be lost by dropping the task.*

## Done When

*The observable condition that ends it. A task whose completion is a matter of opinion is one that
gets marked done twice and finished never.*

## Notes

*Anything the next person needs and cannot derive: the file to start in, the command to run, the
trap somebody already fell into.*
