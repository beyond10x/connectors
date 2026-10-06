---
format: aep.planning-md/3
id: specification:wave-20261006a-consumer-launch
kind: specification
status: implemented
title: 'Wave 20261006a: consumer launch'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T04:36:26Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T04:36:27Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-06T06:33:47Z", actor: "agent:claude", revision: 5}
---
## Wave 20261006a: consumer launch

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront and now. do not ask for permission, you orchestrate this") and delegated every decision until 10:00 on 2026-10-06.

## Units

| unit | story | branch | worktree | build dir |
|---|---|---|---|---|
| U1 | `story:launch-consumer-with-connection-credential` | `unit/launch-consumer` | `connectors-w-launch` | `~/.cache/b10x-target/connectors-w-launch` |

## Rules

- At most two adversary passes; the design is the story's `## Design decisions (2026-10-06)`.
- Package-scoped tests in the unit; the coordinator runs the repository gate on the integration branch.

## Commits approval authorises

One commit for the unit through `b10x-gates bot`; its merge into `wave/20261006a`; the closing planning-store commit; the pull request into `main` and its merge. No release.

## Outcome

Closed 2026-10-06. One unit merged into `wave/20261006a`; the repository gate passed in the pull request's CI, run 37422093575, after one coordinator fix: `local_foundation` still listed `connectors-local/3` as a format to refuse (`3728902d57`).

| unit | story | commit | review |
|---|---|---|---|
| U1 | `launch-consumer-with-connection-credential` | `cdc1cc8183` | one security review: 7 of 8 custody invariants held; descriptors the caller left open reached the consumer, fixed (`close_range` with `CLOSE_RANGE_CLOEXEC` from fd 4); the untested capture-before-custody order kept as written |

Design decision 1 was revised the same day (`--args`, `pass_env`) so cortex can start every `ekr` subcommand through the launch. The gate ran in CI because the machine's shared disk could not hold the workspace build.
