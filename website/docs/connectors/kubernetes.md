---
title: Kubernetes
sidebar_position: 14
description: Read configured namespaces and resource kinds of a Kubernetes cluster with a saved bearer token, and run a command in a pod under approval.
lede: One Kubernetes instance per cluster and scope reads the namespaces and resource kinds its native file names, and runs one approved command in a pod when that is switched on.
source: docs/local-kubernetes-cli.md, docs/local-approvals.md, adapters/kubernetes/contracts/mutations/v1alpha1/semantics.md
connectors: "kubernetes"
---

# Kubernetes

The `kubernetes` connector is served by its own executable, `connectors-kubernetes`, not by the
catalog provider. The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Kubernetes guide](https://github.com/beyond10x/connectors/blob/main/docs/local-kubernetes-cli.md)
has every field, operation and refusal.

No declaration ships for it, so the instance is pinned: it names its executable, that
executable's SHA-256 and the configuration revision the executable prints for your native file.
Commands that need a cluster are shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `resources.list`, `resources.get`, `namespaces.list`, `deployments.history`, `endpoints.discover`; when switched on in the native file: `hosts.discover`, `pods.logs`, `contexts.list`, `nodes.list`, `custom_resources.list` and the release reads `helm_releases.history`, `helm_releases.status`, `helm_releases.values`, `helm_releases.manifest` | `pods.exec`, when `pod_exec` is on: one argument vector in one named container, bounded in time and output |

Reads cover only the configured namespaces and the kinds `pods`, `services`, `deployments`,
`endpointslices`, `replicasets`, `statefulsets` and `events`; there is no wildcard.

## Prerequisites at the provider

A bearer token for an identity the cluster accepts, such as a service account's token, allowed
to create a SelfSubjectReview (connecting runs one; a cluster that forbids it refuses the
connection) and to `list` and `get` the kinds you configure. `pods.exec` needs the cluster's
permission on the `pods/exec` subresource. Build the executable:

```sh
cargo build --release --locked -p connectors -p connectors-kubernetes
```

## Enable the connector and add an instance

Write an owner-only native file, `kubernetes.json`, with your cluster and scope:

```json
{
  "format": "connectors-kubernetes-local/1",
  "instance": "kubernetes-local",
  "api_base": "https://cluster.example:6443/",
  "namespaces": ["default"],
  "resource_kinds": ["pods", "services", "deployments", "endpointslices"],
  "discover_hosts": false,
  "helm_release_reads": "off"
}
```

Print its bootstrap, hash the executable, and pin both in the instance:

```sh
target/release/connectors-kubernetes --local-config /absolute/path/kubernetes.json --print-local-bootstrap
sha256sum target/release/connectors-kubernetes
```

```toml
[connectors.kubernetes]
enabled = true

[connectors.kubernetes.instances.cluster]
alias = "cluster"
instance_id = "kubernetes-local"
adapter_id = "kubernetes"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.kubernetes.instances.cluster.permissions]
profiles = ["kubernetes.token"]
operations = ["resources.list", "endpoints.discover"]

[connectors.kubernetes.instances.cluster.executable]
path = "/absolute/path/connectors-kubernetes"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/kubernetes.json"]
```

Copy `configuration_revision` from the bootstrap verbatim; it is not the SHA-256 of any file. An
operation left out of `operations` is refused. In the guide's run:

```console
$ target/release/connectors --output json instances list | jq -c '.result[] | arrays | .[]'
{"connector":"kubernetes","instance":"cluster","instance_id":"kubernetes-local","restart":"never","startup":"on-demand"}
```

## Connect

```sh
connectors connections connect --connector kubernetes --instance cluster \
  --profile kubernetes.token --credential-prompt
```

The saved identity is the username the cluster reports, such as
`system:serviceaccount:observability:reader`. For automation, an owner-only `{"token":"..."}`
file goes to `--credential-file`. A kubeconfig user that authenticates through an `exec` plugin
is covered in the full guide.

## A first read

```sh
connectors invoke kubernetes/cluster resources.list \
  --input-json '{"namespace":"default","kind":"pods","limit":50}'
```

An RBAC denial appears when the read is dispatched, as a `forbidden` refusal.

## A first guarded write

`pods.exec` is the one write. Set `"pod_exec": true` in the native file, copy the new
configuration revision, select `private_protocol = "connectors-private/2"` and add `pods.exec` to
`operations`. Create the signing key (`connectors approvals key-init --connector kubernetes
--instance cluster`), add an `[approval_clock]` table
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)) and
publish the policy:

```sh
printf '{"operations":["pods.exec"]}\n' > policy.json
connectors approvals policy-set --connector kubernetes --instance cluster --input-file policy.json
```

`exec.json` names the pod, the container and the argument vector; no shell is added:

```json
{"namespace":"default","pod":"api-7d4b9c","container":"api","command":["cat","/etc/os-release"],"timeout_seconds":10}
```

Take the connection, and the schema and revision of `pods.exec` from `invoke --describe`, then
prepare, issue and invoke:

```sh
connection=$(connectors --output json connections list --connector kubernetes --instance cluster \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json invoke kubernetes/cluster pods.exec --describe \
  | jq -r .result.schema)
revision=$(connectors --output json invoke kubernetes/cluster pods.exec --describe \
  | jq -r .result.revision)
connectors --output json approvals prepare --connector kubernetes --instance cluster \
  --connection "$connection" --operation pods.exec \
  --schema "$schema" --revision "$revision" --input-file exec.json
connectors approvals clock-check --connector kubernetes --instance cluster
connectors approvals issue --connector kubernetes --instance cluster \
  --connection "$connection" --operation pods.exec \
  --schema "$schema" --revision "$revision" --input-file exec.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-exec-1.json"
connectors invoke kubernetes/cluster pods.exec \
  --input-file exec.json --approval-file "$HOME/private/proof-exec-1.json" \
  --idempotency-key exec-1
```

A non-zero exit is an applied attempt with that `exit_code`. A lost stream is `unknown`: the
command may have run, and it is never sent again.

## Reconnect when the connection is Stale

A token the cluster no longer accepts makes the connection `stale`, and an invoke answers
`next_action: repair_connection`. Publish a new token for the same username:

```sh
connectors --output json connections list --connector kubernetes --instance cluster \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector kubernetes --instance cluster \
  --expected-revision "<revision from the list>" --credential-prompt
```

A token of a different username is refused as an identity mismatch and the saved one is kept.

## Limits

- Real-cluster acceptance ran against k3s on 2026-10-02 (the [full guide](https://github.com/beyond10x/connectors/blob/main/docs/local-kubernetes-cli.md#run-the-bounded-real-provider-acceptance) shows how to run it):
  selected reads, an RBAC refusal and the local lifecycle; `pods.logs`, `contexts.list`
  and `pods.exec` are verified against recorded answers and streams, not a live cluster.
- No per-operation permission pre-check: the cluster's own RBAC decision is reported.
- No watch, log following, interactive exec, port forwarding or other mutation; of Helm, only
  release-state reads.
- The token's expiry is not observed; an expired token is reported when next used.
- After rebuilding the executable, update `sha256` and stop the running process before the next
  command.
