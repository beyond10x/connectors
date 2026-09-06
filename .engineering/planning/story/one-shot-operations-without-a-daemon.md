---
format: aep.planning-md/1
id: story:one-shot-operations-without-a-daemon
kind: story
status: active
title: One-shot operations without a daemon
tags:
- ready
- wave-cli
scope:
- confidence: inferred
  path: crates/connectors-cli/README.md
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-cli/tests/one_shot_operations.rs
- confidence: cited
  path: crates/connectors-console/src/doctor.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: inferred
  path: crates/connectors-runtime/src/lib.rs
- confidence: cited
  path: crates/connectors-runtime/src/one_shot.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-kubernetes/src/local.rs
- confidence: cited
  path: crates/integration-monitoring/src/backend.rs
- confidence: cited
  path: crates/integration-platform/src/lib.rs
- confidence: cited
  path: crates/integration-slack/src/backend.rs
- confidence: cited
  path: crates/integration-slack/src/backend/open.rs
- confidence: cited
  path: crates/integration-slack/src/backend_tests.rs
- confidence: cited
  path: crates/server/src/local.rs
- confidence: cited
  path: crates/service/src/runtime.rs
- confidence: inferred
  path: ess/generated/clap
- confidence: cited
  path: ess/system/domains/connection.yaml
revision: 35
---
# Story: one-shot operations without a daemon

## Goal
`connectors operation describe|invoke --config <file> --state-root <dir>` is a client to the personal
daemon's control socket (`crates/connectors-cli/src/lib.rs`, `LocalClient::new(state_root.join(
"connectors.sock"))`); with no daemon it fails `connector-unreachable: local Connector transport
failed: No such file or directory`. The runtime that owns the keyring handles, the egress rules, the
description leases and the audit trail is composed only by `connectors serve` (`PersonalRuntime::bind`).
A caller that is itself a short-lived program — a sync round started by a timer, a CI job, an agent
verb such as `brain acquire` — therefore needs a long-running process beside it before it can read
one page, and `doctor`'s "one-shot commands work" reads as if it did not.

## Shape
Either of two, decided in review:
- **Ephemeral runtime.** When the socket is absent and `--config`/`--state-root` are given, the
  `operation` and `connection` verbs compose the personal runtime in-process for the duration of
  the command (same code path as `serve`, no socket published, leases valid within the process),
  then release the state root. Events still need `serve`; `doctor` says so.
- **`connectors serve --until-idle <secs>`.** A verb spawns the daemon when the socket is absent and
  it exits after a quiet period; concurrent callers share it. Fewer code paths, one more process.

In both, credentials never leave the connectors process; a caller still sees only the protocol.
`doctor`'s wording changes to name which verbs need the daemon.

## Acceptance
- With no daemon, `operation describe` and `invoke` with `--config`/`--state-root` succeed against
  a fixture provider; the state root holds no socket afterwards.
- With a daemon running, behaviour is unchanged (the socket is used).
- `event receive` without a daemon is refused naming `connectors serve`.
- `connectors doctor` names the verbs that need the daemon.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 2 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/connectors-runtime/src/composition.rs` — cited.
- `crates/server/src/local.rs` — cited.
- `crates/connectors-runtime/src/lib.rs` — inferred.
- `crates/connectors-console/src/doctor.rs` — cited.
- `crates/connectors-cli/tests/one_shot_operations.rs` — inferred.
- `crates/connectors-cli/README.md` — inferred.

Medium confidence. Choose ephemeral composition for ordinary bounded operations; no socket or idle process remains. Refactor existing composition and dispatch owners, do not duplicate runtimes. Measure two-process describe/invoke lease behavior before inventing durable state: registry re-describes and catalog refs are deterministic. Session-shaped operations may require an explicit daemon.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 3 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.

## First review correction and final attack

First attack review-result:cli-one-shot-adversary-1-20260906 is retained verbatim. The coordinator classifies its stale Monitoring citation finding as introduced: the unit added five lines before every cited function; exact base/source comparison and the first batch gate establish that change, without claiming a new base execution. The invalid extra-input-key probe remains INFEASIBLE and its original red and exact coordinator correction are retained.

Correction committed as17763bbc1552fcda2b9a323bfdd04f5fcef9bd22, bot author and committer verified. It retains eight adversary cases and changes37 model lines only in numeric source citations: introduced Monitoring/Registry shifts plus ten separately identified pre-existing Kubernetes offsets. The static audit inspected32 direct citations reaching four of19 changed unit paths; no lifecycle, fields, refusal vocabulary or test assertions were weakened.

After Monitoring correction the complete root workspace passed1141,0failed,4ignored (exit0), versus the first attack's1140pass/1fail. After the final Registry/Kubernetes citation-only corrections, all15 targeted ESS citation/claim fences passed,58nonselected (exit0); the expected ESS0.18.0 binary validated all10files. The shell's older PATH-first ESS refused the newer command/model; use the expected installed binary for final checks, do not alter the model to fit an older CLI. Affected full lanes from attack1 remain626passed,2ignored; final full attack must add its own cases and rerun the affected lanes.

Reports and exact patches are under `~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/`; root correction logs under `~/.cache/cw6/p/`. Second/final test-only adversary receives the complete unit base3df1cd2df32472a4afc8feeaac8c46c9de8d1b55 to corrected17763bbc, preserved reports, and existing managed triple. No third attack or whole-wave closure is implied.
