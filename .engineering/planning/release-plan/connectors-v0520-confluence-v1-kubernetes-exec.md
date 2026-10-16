---
format: aep.planning-md/3
id: release-plan:connectors-v0520-confluence-v1-kubernetes-exec
kind: release-plan
status: active
title: 'Release 0.52.0: Confluence REST v1, Slack user-token history, Kubernetes exec credentials and the clock quorum'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-16T02:05:22Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.52.0 of waves 20261015a and 20261015b, with the approval clock quorum already on
`main` (pull request https://github.com/beyond10x/connectors/pull/172): Confluence REST v1 reads as
the provider configuration `confluence-v1`, from a contract this repository authored, with the steps
that move a local install onto it; Slack history and threads on a user token; a Slack `ok: false`
answer as a typed refusal; Kubernetes statefulsets, nodes and custom resources; Kubernetes
connections through a pinned exec credential plugin on Linux; the approval clock answering from a
quorum of Roughtime sources; the `live-read-loop` acceptance runner.

Minor rather than patch: new operations, a new provider configuration, new native configuration
fields (`exec_plugin`, `node_reads`, `custom_resources`) and new refusals.

## Scope

The stories moved to implemented with this release:

- story:approval-clock-quorum
- story:confluence-v1-authored-contract
- story:kubernetes-eks-exec-auth-and-kinds
- story:slack-ok-false-is-a-refusal
- story:slack-user-token-history-reads

## Evidence

- The repository gate with MSRV (`connectors-build gate --msrv`) on d77fe9108: all checks
  passed; 2317 tests passed, 0 failed, 106 ignored; the Rust 1.88 and 1.91 checks, every adapter's
  library boundary, the generated documentation, CLI contract and entity drift checks, the ESS
  conformance suites and `aep plan artifact validate` exit 0.
- The run before it on 7ba6a5436 failed only at the library boundary step: the Kubernetes library
  built without default features did not enable serde_json's `raw_value`, which the exec credential
  parser uses. Fixed in d77fe9108.
- The exec credential plugin's second fix pass (an independent security review: a plugin leaving
  its process group, admitted code-loading environment names, the set-ID list wording, the
  descriptor-marking cap, the kubeconfig buffer) ran its packages green: 719 passed, 0 failed, with
  clippy, format and drift checks.
- The pull request's CI is the gate for the merge into `main`.


## Completion boundary

- story:sustained-live-read-loop stays active: the runner shipped, but its acceptance is a
  10-minute live run with no manual revalidation, which waits on
  story:connection-lease-holds-unattended.
- story:confluence-token-kind-binding stays active: the host does not yet probe a token and bind
  REST v1 or v2; the operator chooses `confluence-v1` in the configuration.
- No live Confluence site was read through REST v1, and no live EKS cluster through the exec
  plugin; both were verified against local fixtures and recorded sessions.
- The exec plugin's limitations are in its contract: a descendant that leaves the plugin's process
  group is not followed; on kernels before 5.11 descriptors above 2^20 are not marked
  close-on-exec.
- The clock quorum is two of two configured sources; Slack writes stay behind it.
- The Windows clock quorum flake is story:clock-quorum-windows-unreachable-flake.
- macOS release signing waits on the signing secrets.
