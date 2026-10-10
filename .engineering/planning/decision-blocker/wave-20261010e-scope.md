---
format: aep.planning-md/3
id: decision-blocker:wave-20261010e-scope
kind: decision-blocker
status: cleared
title: Which parity units wave 20261010e delivers
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-10T14:23:49Z", actor: "human:timo", revision: 2}
---
## Question

Which parity units wave 20261010e delivers, by call count on `docs/fluxplane-plugin-parity.md`.

## Options

| option | what | cost |
|---|---|---|
| A | binary responses in the catalog engine (story:catalog-binary-responses), then story:parity-slack-file-reads (file download 11, info 7, list 3) and story:parity-jira-attachment-download (2); story:parity-kubernetes-pod-logs (5) and story:parity-kubernetes-contexts (5): 33 calls | one tree, units built in turn, at most 10G |
| B | A plus story:parity-kubernetes-exec (`kubernetes.pod.exec`, 9) as a mutation under the approval, attempt and bounded-output rules of decision-blocker:helm-execution-family; port-forward (4) left out: 42 calls | as A |
| C | A plus story:parity-homer-sip-reads (33): a new native adapter and a new login-exchange auth profile | as A, plus a credential mechanism not yet reviewed |

Held, not offered: story:parity-jira-issue-writes (326) and story:parity-slack-message-writes (162) wait on work outside this wave; Slack file upload and delete (31) stay with the Slack writes; story:parity-confluence-cql-search-v1-held (8) waits on vendoring the Confluence v1 document; story:parity-alertmanager-alerts (2) has no modelled alert record; story:parity-kubernetes-secret-read (1) needs a disclosure decision.

Recommended: A.

## Decided

Option B, 2026-10-10: the catalog binary responses with an adversary review, the Slack file reads, the Jira attachment download, the Kubernetes pod logs and contexts, and `kubernetes.pod.exec` as a mutation under the execution-family rules (approval, attempt, bounded output) with an adversary review. Port-forward stays out. Homer stays held: a new login-exchange credential profile is the operator's question. One tree, units built in turn with `cargo clean` between them, at most 10G on disk. Package gates run locally and the full gate in the pull request's CI. The wave is released as 0.45.0 after it merges on green CI.
