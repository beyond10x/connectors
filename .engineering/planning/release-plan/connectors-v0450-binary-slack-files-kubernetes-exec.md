---
format: aep.planning-md/3
id: release-plan:connectors-v0450-binary-slack-files-kubernetes-exec
kind: release-plan
status: active
title: 'Release 0.45.0: binary catalog reads, Slack files, Jira attachments, Kubernetes logs, contexts and exec'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261010e-scope
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-10T17:09:56Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.45.0: binary responses through the catalog engine with Slack file list, info and
download and Jira attachment content; Kubernetes pod logs, kubeconfig contexts and pod exec as a
write over a host-bounded WebSocket upgrade.

Minor rather than patch: the Slack and Jira selection sets and their descriptors grow, catalog and
Kubernetes configurations gain fields (`hosts`; `pod_logs`, `kubeconfig`, `pod_exec`), and the SDK
and host gain an upgraded-stream write. Existing configurations without the new fields keep their
revision and keep working.

## Scope

| surface | change |
|---|---|
| `adapters/catalog/src`, `adapters/catalog/spec/ess/domains/binary.yaml` | binary response kind, download, redirect admission, connection `hosts` |
| `adapters/catalog/providers/slack/operations.json`, `adapters/slack/spec/ess` | `files.list`, `files.info`, `file.download`; the file record |
| `adapters/catalog/providers/jira/operations.json` | `attachment.content` |
| `adapters/kubernetes` | `pods.logs`, `contexts.list`, `pods.exec`; ESS `logs`, `contexts`, `mutations` |
| `crates/connectors-sdk`, `crates/connectors-host`, `ess/domains/transport.yaml` | `AuthenticatedWrite::upgrade`, `ScopedHttp::into_upgrade_write`, host-enforced bounds; dependency `tokio-tungstenite` 0.29.0 (framing only) |
| guides, parity page, README, CHANGELOG, website | documentation and version 0.45.0 |

Stories: `story:catalog-binary-responses`, `story:parity-slack-file-reads`,
`story:parity-jira-attachment-download`, `story:parity-kubernetes-pod-logs`,
`story:parity-kubernetes-contexts`, `story:parity-kubernetes-exec`; decided in
`decision-blocker:wave-20261010e-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/155, merged as `d565def6f`: repository gate,
  Build documentation, planning validate and Security and privacy passed.
- Package gates on `89859c044`: fmt; clippy and test of 15 crates (every crate the wave touched and
  every dependent of connectors-host): 1,915 passed, 0 failed, 99 ignored;
  `connectors-docs generate --check` current; the shared, catalog, Slack and Kubernetes models
  valid under ess 0.57.0.
- Adversary passes `review-result:adversary-catalog-binary-responses-pass-1` and
  `review-result:adversary-parity-kubernetes-exec-pass-1`: every finding fixed, every case kept.
- Fixtures only; no live Slack, Jira or Kubernetes call. The Atlassian media redirect host is
  inferred, not verified live.

## Completion boundary

Not in this release: Slack file upload and delete and message writes
(`story:parity-slack-files`, `story:parity-slack-message-writes`), Kubernetes port-forward
(`story:parity-kubernetes-exec-portforward`), `story:parity-homer-sip-reads` (a login-exchange
credential profile is undecided), `story:parity-jira-issue-writes`,
`story:parity-confluence-cql-search-v1-held`, `story:parity-alertmanager-alerts`,
`story:parity-kubernetes-secret-read`.
