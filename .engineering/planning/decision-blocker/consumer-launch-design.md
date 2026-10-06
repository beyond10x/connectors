---
format: aep.planning-md/3
id: decision-blocker:consumer-launch-design
kind: decision-blocker
status: cleared
title: The consumer launch needs three design decisions before it can be built
relations:
- blocks: story:launch-consumer-with-connection-credential
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-06T03:04:41Z", actor: "agent:claude", revision: 3}
---
## What stops the story

The scoped tree does not support the story's Work as written (see the story's "Open design questions"). Three are design decisions for this repository's owners: how a launch obtains the binding without starting the adapter, which process may read the connection credential, and how the CLI takes a trailing argument list (filed upstream as beyond10x/ess#466).

## What would clear it

A recorded decision for each of the three, written into the story's Work and the custody and CLI contracts it names.

## Decision (2026-10-06)

Decided 2026-10-06 by the coordinating session under the operator's delegation ("for decisions, until 10am, take all decisions yourself").

| # | question | decision |
|---|---|---|
| 1 | trailing argument list | No `-- <args…>` passthrough. A consumer's whole argv is pinned in its `[consumers]` entry; the launch takes only `--consumer <name>` and `--connection <id>`. The CLI stays expressible in `ess-cli/1` as it is, so the story no longer waits for beyond10x/ess#466. |
| 2 | exec and stdio | The handler starts the consumer as a child with stdin, stdout and stderr inherited, waits for it, and exits with the consumer's exit status; the structured reply goes to stderr only when the launch itself is refused. |
| 3 | lapsed evidence | Keeps the existing contract: `unavailable` (readiness). The Acceptance names `unavailable`, not `not_granted`. |
| 5 | binding without starting the adapter | From the saved connection record: provider authority and profile are read from the registry entry the connection was saved with. No adapter process is started for a launch. |
| 6 | which process reads the credential | The owner process, as every connection-credential read is today. It writes the secret into a sealed memfd (`F_SEAL_WRITE`, `F_SEAL_GROW`, `F_SEAL_SHRINK`, `F_SEAL_SEAL`) and passes it to the consumer as fd 3; the CLI process never holds the secret. |
| 7 | how fd 3 is passed | The existing pattern: `Command` with the descriptor placed at fd 3 before exec; no `execveat`. |
| 8 | config format | A new `connectors-local/3` that adds `[consumers]`; `/1` and `/2` stay readable. |
| 9 | release | The release updates the website guides and CLI examples, per `AGENTS.md`. |

The custody and CLI contract text (`contracts/cli/v1alpha1/semantics.md` and the custody notes) is written by the story's implementation, from this table.
