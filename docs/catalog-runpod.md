# Runpod pods through the catalog provider

The catalog provider creates, lists and terminates Runpod pods from the pinned
Runpod REST API OpenAPI document, with the Runpod API key held in the Connectors
keyring. A caller that starts GPU pods on demand invokes these three operations
instead of carrying its own HTTP client for the Runpod REST API and holding the
account key itself. Nothing here is Runpod-specific code: the pinned document is
compiled into a bundle, a reviewed selection set exposes one read and two
writes, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work as described there;
this page covers what differs for Runpod.

## Source and bundle

The pinned source is
[`adapters/runpod/upstream/runpod-rest-v1.json`](../adapters/runpod/upstream/README.md),
SHA-256 `9500a8989878d53d8731f27bf8dbbd57801b328c760bdb32c38ba36d5cb580db`
(154,609 bytes), retrieved on 2026-10-08 from
<https://rest.runpod.io/v1/openapi.json> and pinned as served: OpenAPI `3.0.3`,
`info.version` `0.1.0`, server `https://rest.runpod.io/v1`. The bundle is built
with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider runpod \
  --source adapters/runpod/upstream/runpod-rest-v1.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile runpod.api-key
```

It carries all 37 operations of the pinned document, none unsupported, each
under the server's `/v1` path. `adapters/catalog/tests/bundle_drift.rs`
refuses a committed bundle or index a fresh run would not reproduce byte for
byte, and the repository gate re-derives the source digest from its archived
copy (`connectors-build source-hashes`).

The document declares `UpdatePod` on both `PATCH /pods/{podId}` and
`POST /pods/{podId}/update`, and does the same for `UpdateEndpoint`,
`UpdateNetworkVolume` and `UpdateTemplate`. The bundle keeps all eight
operations. A selection naming one of these ids is refused when it loads,
because the id alone does not say which of the two requests it means. None of
them is selected.

## The shipped selection set

[`adapters/catalog/providers/runpod/operations.json`](../adapters/catalog/providers/runpod/operations.json)
exposes these three operations and nothing else. `adapters/catalog/tests/runpod.rs`
pins this exact id list with each id's `operationId`, method and path in the
pinned document, so a renamed, dropped or added id, or a source operation that
moved, fails the gate.

| id | `operationId` | request | effect |
|---|---|---|---|
| `pod.create` | `CreatePod` | `POST /v1/pods` with a JSON `body` | write, unguarded |
| `pods.list` | `ListPods` | `GET /v1/pods` | read |
| `pod.terminate` | `DeletePod` | `DELETE /v1/pods/{podId}` | write, unguarded |

`pods.list` takes every query filter the pinned document declares, by name:
`name`, `desiredStatus` (`RUNNING`, `EXITED` or `TERMINATED`), `id`,
`imageName`, `templateId`, `networkVolumeId`, `computeType`, `endpointId`, the
arrays `gpuTypeId`, `dataCenterId` and `cpuFlavorId`, and the booleans
`includeMachine`, `includeNetworkVolume`, `includeSavingsPlans`,
`includeTemplate` and `includeWorkers`. The answer is one JSON array of pods;
the document declares no paging. `pod.terminate` takes `podId` only.

`pod.create`'s `body` is closed by `body_keys` to these keys of the pinned
`PodCreateInput` schema:

| key | what it sets |
|---|---|
| `name` | the pod's name; use it to find the pod with `pods.list` |
| `imageName` | the container image |
| `gpuTypeIds` | the GPU types to rent, in order of preference |
| `gpuCount` | GPUs per pod (Runpod's default 1) |
| `containerDiskInGb` | the container disk, wiped on restart (default 50) |
| `volumeInGb` | the persistent pod volume (default 20; `0` for none) |
| `volumeMountPath` | where the pod volume is mounted |
| `ports` | exposed ports, each `<port>/http` or `<port>/tcp` |
| `env` | environment variables, an object of strings |
| `cloudType` | `SECURE` (default) or `COMMUNITY` |
| `dataCenterIds` | the data centers to place the pod in |
| `interruptible` | `true` for a spot pod |
| `dockerStartCmd` | overrides the image's start command, an array of strings (`[]` keeps the image's) |
| `dockerEntrypoint` | overrides the image's entrypoint, an array of strings (`[]` keeps the image's) |
| `networkVolumeId` | the network volume to attach; it replaces the pod volume |

A body carrying any other key, such as `templateId` or `minRAMPerGPU`, is
refused as `invalid_input` before any request. The values
are not checked by the provider; Runpod validates them and answers `400` for
one it does not accept.

## Writes: approval and outcomes

Both writes are required-approval mutations. They are available only on an
adapter entry with `private_protocol = "connectors-private/2"`, and only under
a published approval policy that names them. For an unattended caller that
holds the issuing key, publish this policy with
`connectors approvals policy-set --adapter gpu --input-file policy.json`:

```json
{"operations": ["pod.create", "pod.terminate"]}
```

Each write is then prepared and issued for its exact input
(`approvals prepare`, `approvals issue`) and invoked with the proof, as
[local approvals](local-approvals.md) and
[the guarded merge guide](local-gitlab-merge.md) describe. An approval binds
the whole input by digest: a proof issued for one pod body is refused for any
other body, and a proof for one `podId` is refused for any other.

Each write sends exactly one request and is never sent again by the provider.
`mutation.classification` reports what is known:

| operation | answer | classification | what the caller concludes |
|---|---|---|---|
| `pod.create` | `201` or `200` | `applied` | the pod exists; `result.body` is Runpod's pod, with its `id` |
| `pod.create` | a documented definite refusal (`400`, `401`, `403`, `404`, `409`, `422`, …) | `refused` | no pod was created; fix the input or the key |
| `pod.create` | a `5xx`, a timeout, or a connection lost after the request was sent | `unknown` | the pod may exist and may be billed. Do not create again blindly: list by the same `name` with `pods.list`, and create only if none is there |
| `pod.terminate` | `204` (documented) or `200` | `applied` | the pod is terminated |
| `pod.terminate` | `404` | `refused`, code `not_found` | Runpod has no pod with that id under the connection's current key: under that key it was already terminated or never existed, and this call terminated nothing. It does not prove the pod is gone: if the connection now holds a key from another Runpod account (see [Authentication](#authentication)), the first account's pods answer `404` while they are still running and billed |
| `pod.terminate` | a `5xx`, a timeout, or a lost connection | `unknown` | the pod may or may not be terminated; check `pods.list` with `id` before terminating again |

Give every pod a unique `name` so that an `unknown` create can be resolved by
one `pods.list` read.

## Authentication

A Runpod API key, sent as `Authorization: Bearer <key>`, under the profile
`runpod.api-key`; the pinned document's one security scheme, `ApiKey`, is
`http` `bearer`. Create the key in the Runpod console's settings with access to
pods. The key enters through the usual protected entry and custody, the
keyring, and never appears in the configuration.

The pinned document has no user or account read. Connecting proves the key
with `GET /v1/pods`: a `200` admits it, a `401` refuses it as an invalid
credential. The answer's body is not read, so an account with no pods connects
too. The identity is therefore the configured connection, not a Runpod
account: its kind is `runpod.connection` and its subject is the configuration's
`instance` id (identity `source` `configuration`, see
[the catalog provider guide](local-catalog-provider.md#configure-the-provider)).
Repair and revalidate cannot tell two Runpod accounts apart: any key that lists
pods is accepted as the same identity. After a repair with another account's
key, every pod the first account still runs answers `pod.terminate` with `404`
and stays billed. Use one `instance` per Runpod account, and repair it only with
a key of that account.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "runpod",
  "provider": "runpod",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://rest.runpod.io/v1",
  "auth": {
    "profile": "runpod.api-key",
    "header": "Authorization",
    "bearer": true,
    "label": "Runpod API key",
    "identity": {"source": "configuration", "path": "pods", "kind": "runpod.connection"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/runpod/operations.json"
}
```

Bind it to the local CLI as [the catalog provider guide](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
describes, with `private_protocol = "connectors-private/2"`,
`profiles = ["runpod.api-key"]` and
`operations = ["pod.create", "pods.list", "pod.terminate"]`, here under the
adapter alias `gpu`. Connect with the credential document
`{"token": "<Runpod API key>"}`:

```sh
connectors --output json connections connect --adapter gpu --profile runpod.api-key --credential-file /owner-only/runpod.json
```

## Invoke

List the running pods with one name:

```sh
connectors operations invoke --adapter gpu --connection "$connection" \
  --operation pods.list --schema "$schema" --revision "$revision" \
  --input-json '{"name":"worker-a","desiredStatus":"RUNNING"}' --output json
```

Create a pod (`create.json`, approved as above):

```json
{"body": {"name": "worker-a", "imageName": "registry.example.com/team/worker:1",
          "gpuTypeIds": ["NVIDIA GeForce RTX 4090"], "gpuCount": 1,
          "containerDiskInGb": 20, "volumeInGb": 0, "ports": ["8000/http"],
          "env": {"MODE": "serve"}, "cloudType": "SECURE"}}
```

```sh
connectors operations invoke --adapter gpu --connection "$connection" \
  --operation pod.create --schema "$schema" --revision "$revision" \
  --input-file create.json --approval-file "$proof_file" \
  --idempotency-key "$business_key" --output json
```

Terminate it (`terminate.json` is `{"podId": "<id from the create result>"}`):

```sh
connectors operations invoke --adapter gpu --connection "$connection" \
  --operation pod.terminate --schema "$schema" --revision "$revision" \
  --input-file terminate.json --approval-file "$proof_file" \
  --idempotency-key "$business_key" --output json
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/runpod.rs`):
  the exact request of each operation with the bearer header, a create answered
  `201` and `200`, a create refused with `400`, a create whose connection was
  dropped after the request was read and one answered `500` (both `unknown`),
  a body key outside `body_keys` refused before any request, a terminate
  answered `204`, `200` and `404`, and the connection probe. No live Runpod
  account has been called and no pod has been created.
- Runpod's own answers to a create it rejects, and to a terminate of a pod that
  is already gone, have not been observed live; the pinned document lists only
  `400` for create and `204`, `400` and `401` for delete.
- No guard: neither write reads the pod first. A pod's state is not pinned
  before terminate.
- `pod.show` (`GetPod`), stop, start, reset, update and every endpoint,
  template, network volume, registry-auth and billing operation are not
  selected.
- The identity is the configured connection, not a Runpod account (see
  [Authentication](#authentication)).
