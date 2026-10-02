---
format: aep.planning-md/3
id: task:gitlab-catalog-current-sandbox-read-replay
kind: task
status: active
title: Recheck the current catalog CLI against the dedicated GitLab sandbox
relations:
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T15:38:19Z", actor: "human:timo", revision: 2, correlation: "wave-20261002d-provider-acceptance"}
- {from: "proposed", to: "active", at: "2026-10-02T15:38:19Z", actor: "human:timo", revision: 3, correlation: "wave-20261002d-provider-acceptance"}
---
## Outcome

Revalidate the dedicated GitLab sandbox through a fresh private configuration of
the current production catalog CLI/child, using its protected delegated credential
and shipped selection. Retain source/binary/configuration identities and actual
outputs. This supplies current-host live read evidence alongside the catalog
worker's deterministic mutation/lifecycle matrix; it does not replace that matrix
or claim unexecuted live mutation/recovery cases.

## Acceptance

The named gitlab-catalog-current-sandbox-read-replay observation records the actual
current shipped read set: eighteen selections in
adapters/catalog/providers/gitlab/operations.json. Eleven are the historical native
surface; seven additional current reads must be reported separately. Execute each
through current production binaries against root/connectors-sandbox with exact
project/MR/pipeline/job/ref inputs, provenance and declared text trace result.
Empty permitted lists are valid only where the exact fixture has no objects; they
are not proof of nonempty payload semantics. Explicitly record missing/expired
provider prerequisites or refused calls, without a passing claim for those rows.

Fresh custody/setup, protected delegated-credential admission, revision preservation
on explicit revalidation, one owner restart and saved-credential reuse are recorded
where reachable. Every owned process is observed exited before handoff. No default
forge integration or API write is used. This current read replay does not replace
the independent catalog mutation/recovery matrix or historical live write evidence.

## Scope and authority

Read-only provider operations in the pre-existing dedicated GitLab container
connectors-gitlab-20260912 at https://localhost:8929/api/v4, project id1 and delegated
sandbox-dev identity2. Secrets stay in existing owner-only files under
$HOME/.cache/connectors-gitlab-20260912. No API writes, token rotation, GitLab
configuration changes or runner changes belong to this task. Use a fresh task-owned
local host state/custody and exact current copied production binaries; do not reuse
historical user host state. Historical scripts are reading material only.

Root runs this evidence task alongside source authoring, waiting for a build/live
window and qualified current binaries. No new runnable repository tooling or
production source is introduced. Public output owns
 docs/evidence/gitlab-current-20261002/ if execution completes; raw/private material
stays ignored. Parent authority is initiative:complete-local-connectors and
approval-record:milestones-delivery-20261002. Missing sandbox capabilities or
credentials remain named limitations. Existing historical live write evidence
is retained rather than silently invalidated or relabelled as a fresh run.
