---
format: aep.planning-md/3
id: story:sustained-live-read-loop
kind: story
status: draft
title: A 10-minute live read loop runs with no manual revalidate
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:connection-lease-holds-unattended
- depends_on: story:upgrade-moves-catalog-adapters
- depends_on: story:slack-user-token-history-reads
- depends_on: story:slack-ok-false-is-a-refusal
- depends_on: story:refusals-name-the-field
revision: 1
---
## Outcome

Connectors serves an unattended read workload: a 10-minute loop of reads across the parity
operations a downstream consumer uses, against live providers, with no manual revalidate, ends with
every read answered.

## Today

A downstream consumer's live test of 0.50.0 (2026-10-14, read calls only) passed 15 of 71
operations, 229 of 515 by call weight. Recorded fixtures cannot fail the reasons it found: the
connection lease, a stale provider binary after upgrade, a user token with no operations, `ok:false`
answered as success, refusals that name no field.

## Change

A live acceptance runner in Rust (`connectors-build`, clap derive) that runs a declared read loop
for a bounded time against connections the operator names, records each call's outcome, latency and
refusal, and fails on any unanswered read or any manual step. It reads no secret into its report and
writes its report outside the repository. It runs on demand, never in CI.

## Acceptance

- A 10-minute run on the operator's host against the read operations of Slack, Jira, GitLab,
  Kubernetes and SQL connections ends with 0 unanswered reads and 0 revalidations by the caller,
  and its report is attached to the release that ships the reliability stories.
- Depends on the lease, upgrade, Slack user-token, `ok:false` and field-name stories.
