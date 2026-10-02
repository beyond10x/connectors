---
format: aep.planning-md/3
id: specification:wave-20261002b-reliability
kind: specification
status: draft
title: 'Wave 20261002b: metadata timeout safety and acceptance reliability'
relations:
- informed_by: specification:milestone-acceleration-20261002
revision: 5
---
## Authority and preflight

AEP implementing skill 0.19.1. Operator approval: approval-record:milestones-delivery-20261002, including commits and verified release. Previous goal turn made progress: it recovered the old session, verified release/upstream state and produced reviewed artifacts. Current main remains c7a9d5b1db1ff4e3b2af568db19c6eaa60d79d90. Store aep.project/5; AEP 0.68.0 for planning, project verification pins AEP 0.65.0 / ESS 0.45.0 (both cached and checked).

Primary status is exactly `?? .agents/`; no unit touches it. Work resumes the existing task-owned managed planning tree, not a dirty primary. The retained planning tree is this wave's integration tree, with no prior build output. Free disk 28 GiB; minimum reserve 8 GiB. Three source workers; initially serialize heavy compilation, use two Cargo jobs and inspect measured target sizes before allowing concurrent builds. Debug symbols and incremental caches may be disabled locally without changing assertion/optimization semantics; never share target directories. No token budget was specified by the active goal.

## Selected units

All units serve vision:independent-contract-adapters. The selected source scopes remain disjoint; the runner's added Cargo dependency/lock update is reconciled by root with release metadata. Root is the sole planning-store writer.

| Unit | Managed id and branch | Current stage |
| --- | --- | --- |
| story:bridge-drop-waits-for-dispatched-batch | cb26b-bridge; impl/cb26b-bridge | commit 91511f4379, integrated in 294477ba21; package 335 passed, independent adversary found nothing |
| story:sql-fixture-accepts-stray-connections | cb26b-sql; impl/cb26b-sql | commit ef5a08f87f, integrated in f43fdbb00f; package 19 passed, 50/50 loaded runs passed, independent adversary found nothing |
| story:ignored-suites-have-a-runner | cb26b-runner; impl/cb26b-runner | adversary 1 introduced child cleanup finding fixed; package 150 passed; final attack and operator-host run ongoing |

Publication integration is now cb26b-publish, branch delivery/reliability-20261002, HEAD 0f9d56f994009b19490f609ced3239f85a0c7953. It contains bot-authored source merges and intended v0.25.0 metadata. No branch/tag/release has been published. Stories stay active pending the required integrated gate. The clock experiment remains sequenced afterward; catalog scope/preflight is recorded but not dispatched.

Every managed tree is below the configured b10x/connectors tree root, uses target/ for build output and .local/wave-20261002b/ for scratch. Publication target and website/node_modules were transferred exclusively from the inactive original integration; no two worktrees share a mutable build directory. Root precompiles final test dependencies while runner acceptance finishes. Keep 8 GiB free and two Cargo jobs per active build.

Original connectors-plan-20261002 is private evidence, archived with worktree archive --replace at HEAD 294477ba21; archive patch SHA256 b2fee61ee81a91da6b5101aad57e8214b7f86148accac9102cc03f82e4fa19d6. Its fixture credentials remain needed for provider acceptance, so retain its lease and directory until exact fixture teardown. It owns no future public store mutation. Root leases: codex-cb26b-publish and codex-connectors-delivery-20261002; completed source trees have coordinator leases codex-cb26b-{bridge,sql}-coordinator. Worker and adversary leases belong to those agents only.

Approved short physical scratch exceptions are $HOME/.cache/cb26b-runner/tmp and $HOME/.cache/cb26b-runner-adversary/tmp, mode 700, task-owned; root cleans them after retained evidence. The runner's --tmpdir explicitly admits physical private directories and reports overly long custody paths as missing prerequisites. Symlink aliases are refused by custody and are not used.

## Completion

Package-scoped red/green, format/lint, independent adversarial cases, coordinator verification, then one integrated gate/MSRV and affected website checks on the release candidate. Bot commits and PR checks must pass before merge/tag/page. Preserve exact test summaries and outcome evidence. Do not close an upstream or product decision without its actual clearing evidence.

## Dispatch paths

All three trees were created from b3ddded7618b5a4a23fe74dbec739e0938ecc296 by worktree create under the configured managed root. Relative to that root's connectors/ directory: cb26b-bridge, cb26b-sql, cb26b-runner. Each uses target/ and .local/wave-20261002b/ for build and scratch; each scratch/brief.md is the immutable unit assignment. Branches impl/cb26b-bridge, impl/cb26b-sql, impl/cb26b-runner. Worker leases codex-cb26b-<lane>. SQL holds the first measured compile slot; bridge and runner start source discovery/test authoring immediately and wait for compilation admission. Sccache show-stats responded; current repository profiles already disable debug symbols/incremental output and retain debug assertions.

## Measured build capacity and diagnosis

SQL baseline package suite completed in 163 seconds, target 889092 KiB (about 868 MiB); local_runtime 4 passed/1 ignored and protocol 7 passed. Free disk about 27 GiB afterward. Bridge and runner package builds admitted concurrently with two jobs each, with workspace builds still coordinated. Source discovery remains parallel. SQL reproduction found valid SUT PostgreSQL CancelRequest control traffic, not proof of a foreign process; acceptance keeps malformed packet and extra-session negative controls.

## Provider acceptance preparation — 2026-10-02

Root owns one disposable container: connectors-20261002-pg, id 7097eef0c3f4eda82b574f9890db1719b0a7f96e7eaaf6cdc92686861b0f3fb6, label connectors.acceptance=codex-connectors-20261002, image sha256:d741b376874687de90374fd34f55c6b2760e8f7bd7e4ae5cd47f50757fc08cf8 (postgres:17.6-alpine3.22), bound at 127.0.0.1:33096. It has the incidents database, a SELECT-only reader role and three rows (two critical/one minor), matching the live CLI journey. Private bootstrap files are under this tree's .local/provider-acceptance-20261002 (mode700); do not publish their contents. Readiness passed; no Connectors live journey has run yet. Verify exact id and label before cleanup. Do not touch pre-existing provider containers.

Installed gnome-keyring-daemon digest matches the repository qualification exactly: b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b. The PostgreSQL journey explicitly uses plaintext loopback; it does not prove TLS. Kubernetes acceptance fixture is not started yet. Catalog production CLI port scope is now recorded on story:catalog-cli-journeys, still draft.

## Kubernetes fixture ownership — 2026-10-02

Root created connectors-20261002-k3s, exact id eb54025175eb8134d2752d11528ee5a638ca42363ddc09b242c9cc292a4c1646, image sha256:efe65d76faac869ca7329373cb5a5c5676cb3dbfc0a48d0574c6f4d7fa285d2a (rancher/k3s:v1.31.5-k3s1), label connectors.acceptance=codex-connectors-20261002. API is https://127.0.0.1:33097; node readiness passed. A private explicitly selected kubeconfig created the fixture namespace, a reader ServiceAccount and namespaced get/list access for pods/services/endpointslices only, plus one pod and service. Pod readiness is not yet asserted. Eight-hour bearer token and CA files are in .local/provider-acceptance-20261002, mode600; never print or commit token/kubeconfig. No default Kubernetes context is selected. All resources belong to this disposable container; verify exact id/label before cleanup. Live Connectors acceptance has not run yet.

## Integration checkpoint — 2026-10-02T12:30Z

Bridge unit commit 91511f4379579d0605af80420d5c34ea4874d162 merged as 294477ba213635bdc68f9285da954e308456eae2. SQL unit commit ef5a08f87f4c641325bbd46810b5aeb38666913b merged as f43fdbb00f3193be222935e62e8ce1cf82d125a1. Every direct author and committer was verified as b10x-bot[bot]. The bot wrapper supports commit/tag/push/fetch, so merge preparation used git merge --no-commit; only b10x-gates bot commit created the commits. No source has been pushed yet.

Both independent adversary passes returned nothing found; their exact reports are immutable review-result:wave-20261002b-{bridge,sql}-adversary-1. Bridge package runs 335 cases, 26 ignored. SQL package runs 19 cases; its 50 repeated local_runtime runs each passed 10 cases under actual workspace load from12:22:53–12:24:29UTC. The load ran12:22:19–12:30:13UTC and itself ended1187 passed/1 failed/45 ignored: one new runner temporary-directory test failed, then its corrected package passed. Full reports and load evidence live under docs/evidence/reliability-20261002/. These are unit proofs; the integrated gate has not run.

Runner remains in adversarial verification. Two tests reproduce descendants surviving both successful and failed fixture exits. Root awaits the immutable report before routing the fix; no runner commit or integration yet. Scope now additionally includes its Cargo.toml and Cargo.lock for a possible existing libc dependency used to preserve group-leader identity through cleanup. Its actual compiled inventory is45 ignored tests,5 helpers,0 unknown across134 executables. Do not replace these measured counts with the earlier source count.

Intended release v0.25.0 metadata is prepared, not released. Initial workspace build, website typecheck/build and public-path audit passed before final runner integration. Repeat only affected validation after final changes and run the required full gate/MSRV before publication. Catalog next-story scope has been independently audited and corrected; its ER observation/fault-injection seams still need implementation proof. Clock candidate preflight is read-only until the bridge dependency closes on the integrated gate.

## Publication checkout and path-only redaction

The previous integration tree remains private recovery evidence. Gates refused its uncommitted coordinator commit with37 personal-path findings, all machine-specific paths in reports or workspace records. No rejected commit was created or pushed. AEP cannot change immutable review bodies, so a fresh managed publication checkout cb26b-publish, branch delivery/reliability-20261002, starts at the exact merged source commit294477ba213635bdc68f9285da954e308456eae2. Root recreated the pending planning updates, evidence and review records through the AEP CLI; it did not delete or edit the original immutable records. Current revision numbers differ because the pending operations were reconstructed into this publication store.

Public report copies replace the literal operator home prefix with the portable $HOME marker. That is the only report-text transformation; test names, commands, results, assertions, timing, findings and verdicts are unchanged. Raw reports remain in the unit scratch and original private integration tree. This explicitly disclosed path redaction takes precedence over verbatim-publication guidance. No raw report containing personal paths is admitted to Git. The report provenance table in docs/evidence/reliability-20261002/README.md records raw and publication digests.

Publication checkout build target will be transferred exclusively from the inactive original integration target; no concurrent build shares it. Original private fixture credentials remain in the original tree until provider acceptance and exact fixture cleanup finish. Root lease in publication tree is codex-cb26b-publish. The original tree is retained until archive/handoff and owns no future publication.
