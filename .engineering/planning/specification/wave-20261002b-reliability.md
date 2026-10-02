---
format: aep.planning-md/3
id: specification:wave-20261002b-reliability
kind: specification
status: draft
title: 'Wave 20261002b: metadata timeout safety and acceptance reliability'
relations:
- informed_by: specification:milestone-acceleration-20261002
revision: 1
---
## Authority and preflight

AEP implementing skill 0.19.1. Operator approval: approval-record:milestones-delivery-20261002, including commits and verified release. Previous goal turn made progress: it recovered the old session, verified release/upstream state and produced reviewed artifacts. Current main remains c7a9d5b1db1ff4e3b2af568db19c6eaa60d79d90. Store aep.project/5; AEP 0.68.0 for planning, project verification pins AEP 0.65.0 / ESS 0.45.0 (both cached and checked).

Primary status is exactly `?? .agents/`; no unit touches it. Work resumes the existing task-owned managed planning tree, not a dirty primary. The retained planning tree is this wave's integration tree, with no prior build output. Free disk 28 GiB; minimum reserve 8 GiB. Three source workers; initially serialize heavy compilation, use two Cargo jobs and inspect measured target sizes before allowing concurrent builds. Debug symbols and incremental caches may be disabled locally without changing assertion/optimization semantics; never share target directories. No token budget was specified by the active goal.

## Selected units

All serve vision:independent-contract-adapters. Existing typed scopes are cited with explicitly marked inferred test/module files. The computed draft waves, collisions and unassessed lists were retained verbatim in the task scratch waves-before.json before activation. The three selected units share no edit paths. The clock experiment shares metadata paths and is excluded until bridge cleanup lands. Unrelated unassessed backlog items are not scheduled.

| Unit | Managed id and branch | Stage |
| --- | --- | --- |
| story:bridge-drop-waits-for-dispatched-batch | cb26b-bridge; impl/cb26b-bridge | preparing |
| story:sql-fixture-accepts-stray-connections | cb26b-sql; impl/cb26b-sql | preparing |
| story:ignored-suites-have-a-runner | cb26b-runner; impl/cb26b-runner | preparing |

Integration is managed id connectors-plan-20261002 on plan/milestones-20261002. Unit paths are returned by worktree create and recorded before dispatch. Each build is <unit-tree>/target; each scratch is <unit-tree>/.local/wave-20261002b. Coordinator scratch is .local/wave-20261002b in the integration tree. Coordinator is sole planning-store writer. aep:implementor then aep:adversary procedures run through Codex subagents, since this host has no plugin agent-type selector. All committed executable code is Rust with clap derive for CLIs; no Python.

## Completion

Package-scoped red/green, format/lint, independent adversarial cases, coordinator verification, then one integrated gate/MSRV and affected website checks on the release candidate. Bot commits and PR checks must pass before merge/tag/page. Preserve exact test summaries and outcome evidence. Do not close an upstream or product decision without its actual clearing evidence.
