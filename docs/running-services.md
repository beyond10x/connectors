# Run connector services

These instructions describe the implemented first-slice runtime. Run commands from
the repository root after [building the workspace](development.md). For a complete
disposable environment, follow the [live acceptance recipe](live-e2e.md).

Connector, instance and connection are the terms of the [vocabulary](../README.md#vocabulary).

Copy the matching file from `examples/`, choose a unique instance/listener, configure
the permitted resources, and bind private credential files or environment references.
Credential files must be regular files owned by the running user with mode `0600`.
Credentials never belong in operation input. Config changes require a restart.
Credential file replacement is resolved on subsequent requests, including gateway
downstream credentials. Environment references reflect the process environment;
changing a parent shell's environment cannot update an already running service.

```sh
target/debug/connectors-kubernetes --config kubernetes.yaml
target/debug/connectors-sql --config sql.yaml
target/debug/connectors-catalog-provider --config catalog.json
target/debug/connectors serve --config federation.yaml
```

## Admission

A host admits its caller under one profile, chosen by the `service` member of its
configuration. The [service contract](../contracts/service/compatibility.md), § 2.1,
gives the exact rules.

- `static-bearer`, the default: the caller's bearer must equal the bytes of
  `service_credential`, which is read at every request.
- `service-account-token`: the caller presents its Kubernetes service account token, and the
  host asks the cluster's TokenReview API about it at every request. It needs the
  `urn:connectors:config:v3:service` members below and no `service_credential`.

```json
{
  "instance": "gitlab.example-org",
  "listen": "0.0.0.0:7104",
  "state": "/var/lib/connectors/state",
  "admission": "service-account-token",
  "audience": "connectors-gitlab",
  "principal": "system:serviceaccount:agents:coding-agent",
  "token_review": {
    "api": "https://kubernetes.default.svc",
    "ca_file": "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt",
    "credential_file": "/var/run/secrets/kubernetes.io/serviceaccount/token"
  }
}
```

The host admits a request only when the review authenticates the token for `audience` as
`principal`. Any other answer is `unauthorized`. A review that fails, takes longer than
5 seconds or cannot be reached is `unavailable`. In either case no operation runs. The host
sends one review per request and keeps none. At most 8 reviews run at once: a request that
arrives while 8 are under way is `unavailable` at once and sends no review, so a caller can
retry it. The host never logs the token. Its own token,
`credential_file`, is read at each review, so a projected token that the kubelet rotates is
used from the next review on. The host's service account needs the `system:auth-delegator`
cluster role, or another role that may create `tokenreviews`. Under this profile
`GET /v1/describe` is admitted the same way, and `POST /v1/invoke` answers `unsupported`.
Callers invoke through `POST /v1alpha2/invoke`, which records the service account as the
principal of every call.

The shared CLI discovers the current contract before invoking it:

```sh
target/debug/connectors describe \
  --endpoint http://127.0.0.1:7102/ --allow-plaintext --token-file service.secret
target/debug/connectors invoke \
  --endpoint http://127.0.0.1:7102/ --allow-plaintext --token-file service.secret \
  --operation endpoints.discover --input examples/requests/kubernetes-endpoints.json
```

Through the example federation host, the same operation is `kubernetes__endpoints.discover`.
Source prefixes distinguish instances; the gateway does not interpret provider data.
The first profile supports one federation hop. It never retries a provider request.

## Catalog instances

`connectors-catalog-provider --config <file>` serves one instance of the
[catalog provider](local-catalog-provider.md) on the host. GitLab, for example, runs this
way. The file has three members:

```json
{
  "service": {
    "instance": "gitlab.example-org",
    "listen": "0.0.0.0:7104",
    "service_credential": {"kind": "file", "path": "/var/run/connectors/service-token"},
    "state": "/var/lib/connectors/state"
  },
  "catalog": {
    "connector": "gitlab",
    "instance": "example-org",
    "endpoint": "https://gitlab.example.com"
  },
  "credential": {
    "binding": "mounted_secret",
    "directory": "/var/run/connectors/credential"
  }
}
```

- `service` is the host's configuration, as above. Its `instance` must be the catalog
  instance's id, `<connector>.<instance>`.
- `catalog` names exactly one source:
  - `connector`, a connector the binary ships, with `instance` and optionally `endpoint`
    and `profile`;
  - `local_config`, a private `connectors-catalog-local/2` file whose paths are absolute;
  - `definition`, a definition file, with `definitions_root`, the definitions checkout its
    source is under, `instance` and optionally `endpoint` and `profile`. The binary lowers
    the definition once at start, in a private temporary directory that it removes again.
- `credential` names where the instance's credential document comes from. The only binding
  is `mounted_secret`. The document is the item `<directory>/<instance id>/<profile id>`,
  for example `gitlab.example-org/gitlab.pat`. Its content is the document the profile's
  protected entry takes, such as `{"token": "..."}`.

The host reads the item at every request, under all seven
[mounted-secret checks](local-mounted-secret-custody.md#checks): the directory on a read-only
mount, the projected `..data` layout or the flat one, an item with one link, no write bit
and at most 64 KiB, on a read-only filesystem, read once through `/proc/self/fd`. A
Kubernetes Secret volume passes them, for example one mounted with
`items: [{key: gitlab-token, path: gitlab.example-org/gitlab.pat}]` and `defaultMode: 0400`.
No pin is kept: a served instance has no connection whose validated version a pin would
hold. A rotated Secret is therefore used from the next read on. An item that is absent or
fails a check answers `unavailable`, and nothing is sent to the provider. Start refuses an
item it cannot read. The binding exists on Linux only; elsewhere start answers
`unsupported`.

Without a grant file the served instance lists and runs its reads only. A catalog write is
not in its description, and invoking one answers `not_found` before anything is recorded or
sent. With `service.standing_grants` set, the description lists the instance's writes too,
and each runs only as [standing write grants](#standing-write-grants) describe. Start also
refuses:

- an instance that uses more than one profile;
- an instance whose profile's material is acquired by OAuth (the CLI writes that, nothing
  mounts it);
- an instance with alternative bindings;
- with a grant file, a write whose answer carries a credential it created: the served host
  has nowhere to keep it.

## Standing write grants

Standing grants are optional. A host without `standing_grants` in its service configuration
is unchanged: `POST /v1alpha2/invoke` dispatches a write with no key, `idempotency_key` is
refused as `unsupported`, and a served catalog instance lists its reads only.

With `standing_grants` set, the host runs a write (an operation with the `mutation` profile)
only under a standing grant: a file the operator mounts that names, per operation, the
targets a caller may write to and how often. There is no per-call approval. The rules are in
the [service contract](../contracts/service/compatibility.md#21-first-binding-post-v1alpha2invoke),
"Keyed writes under a standing grant".

Point the host at the file with `standing_grants` in its service configuration, here the
`service` member of the catalog example above. It requires `state`; without it the host does
not start.

```json
{
  "instance": "gitlab.example-org",
  "listen": "0.0.0.0:7104",
  "service_credential": {"kind": "file", "path": "/var/run/connectors/service-token"},
  "state": "/var/lib/connectors/state",
  "standing_grants": "/var/run/connectors/grants/grants.json"
}
```

```json
{
  "format": "connectors.standing-grants/1",
  "grants": [
    {
      "id": "issues-in-widgets",
      "instance": "gitlab.example-org",
      "operation": "issue.create",
      "target": [{"pointer": "/id", "value": "7"}],
      "not_after_unix_ms": 1798761600000,
      "max_per_hour": 20
    }
  ],
  "revoked": []
}
```

A grant covers a write when it names the host's instance and the operation, is not listed
in `revoked`, its `not_after_unix_ms` is still in the future, and every `target` member holds
in the input: the JSON pointer finds that exact value. `max_per_hour` (1 to 10,000) caps the
writes admitted under the grant in any 3,600 seconds. A file holds at most 256 grants, each
with 1 to 16 target members.

The host reads the file on every write, so an edit, a revocation or a removed grant applies
from the next call on, without a restart. With no file at the path, no write is admitted.
The file may be a plain file or a key of a Kubernetes Secret or ConfigMap volume: on Linux
the host reads the volume's `grants.json -> ..data/grants.json` link through `..data`, as
the [`mounted_secret` custody binding](local-mounted-secret-custody.md) reads its items, and
follows no other symlink. The host refuses the file, and answers `unavailable` without
sending anything, when any directory above it is owned by a user other than the running
user or root or is writable by group or others (its own directory may be, on a read-only
mount such as a volume's), when it is reached through any other symlink or is not a regular
file, is writable by group or others, is owned by a user other than the running user or
root, is larger than 64 KiB, or is malformed. Whoever can change the file can widen what
the instance writes, as whoever can change its mounted credential can: mount it read-only,
with mode `0400`, `0440` or `0644` on a volume and `0400` or `0600` otherwise.

The caller sends every write with `idempotency_key`, a string of at most 256 bytes it chooses
per intended effect. The host answers, before anything is recorded or sent:

| Request | Answer |
|---|---|
| A write without `idempotency_key` | `invalid_input` (400) |
| `idempotency_key` on a read | `unsupported` (400) |
| A key already used for a different operation or input | `idempotency_conflict` (409) |
| A write no grant covers, or any write with no file at the path | `approval_required` (403) |
| A covering grant has used its hour | `rate_limited` (429), with `retry_after_seconds` |
| The grant file or the host's state cannot be read | `unavailable` (503) |
| A write on `POST /v1/invoke`, which records no attempt and reads no grant | `unsupported` (501) |

A write that passes is recorded as one attempt and sent at most once. Sending the same key
and input again never sends again: it answers the original attempt's outcome, marked
`replayed: true`, and checks no grant, so a revoked grant does not hide what already
happened. If the original is still running, the repeat waits up to 25 seconds, without
taking one of the host's in-flight slots, and then answers `outcome_unknown`; at most 4
repeats wait on one write, and a fifth answers `outcome_unknown` at once. To answer a
replay, the host keeps an applied write's result, up to 64 KiB, in its `state` directory; a
replay of a larger one answers `capacity` and still reports the write as applied. The host
admits requests with one key one at a time, so concurrent requests with one key make one
provider request and take one use of the grant: the others are replays, or
`idempotency_conflict` when their input differs. Keys and the hourly counts are kept in the
host's `state` directory (`standing-grants.sqlite3`, mode `0600`) and survive a restart; a
key is never forgotten, so it is never admitted for another request. A write that was in
flight when the host stopped stays pending: its key answers `outcome_unknown` at once and is
not sent again.

`connectors invoke` sends `POST /v1/invoke` with no key, so on a host with standing grants it
runs reads only. A Rust caller sends a keyed write with `Client::invoke_v1alpha2_keyed`;
`Client::invoke_v1alpha2` sends no key, which such a host refuses for a write as
`invalid_input`.

## Supported surfaces

| Service | Operations | External binding |
|---|---|---|
| Kubernetes | `resources.list`, `endpoints.discover`, optionally `hosts.discover`, optionally `helm_releases.history`/`.status`/`.values`/`.manifest` | Kubernetes API; namespace/kind restrictions; bearer token and CA; Helm release reads need their own `helm_release_reads` selection |
| SQL | `schema.list`, `query.read`, `database.list`, `table.list`, `table.describe`, `index.list` | PostgreSQL or MySQL; explicit role or user and database; read-only transactions or sessions and deadlines |
| Catalog | The instance's reads; its writes under a standing grant file | The provider's HTTPS API; one profile; credential document from a mounted file |

SQL uses the `postgresql-native-text` profile: column names and native type names
accompany rows of strings or JSON null. Exact numeric, array, timestamp, and JSON
representations are preserved without numeric coercion. Parameters are text or null
and PostgreSQL resolves their types. Use explicit SQL casts where inference needs
help. The database role's grants govern accessible schemas and functions. Each request
opens a fresh connection and read-only transaction, initialized with `search_path = public, pg_catalog`;
qualify tables in other schemas (for example `reporting.sales`). There is no pool in
this bounded local profile. Fresh connections resolve the current credential each
time and discard session state; connection setup therefore contributes to latency.

Endpoint discovery reports observations with provenance, readiness and reachability.
It does not create a connection or dial the discovered address. Explicitly configure
an SQL binding and its credential after selecting an authorized reachable endpoint.

Service listeners speak HTTP; use loopback locally or a trusted TLS-terminating ingress
for remote deployments. The 32-operation limit and 20-second execution deadline
apply after request admission and reading; configure connection and request-read
limits at the ingress. Upstream TLS certificate validation is enabled by default.
For HTTP providers and SQL, `ca_file` replaces public roots with that PEM bundle.
Federation downstream clients currently use built-in public roots and have no
private-CA setting; local downstreams use explicitly permitted loopback HTTP.
`allow_plaintext` is an explicit local-test configuration, not a TLS verification bypass.

SQL keeps its execution deadline independent of caller-controlled database settings.
Timeouts and dropped invocations trigger a bounded cancellation/cleanup attempt;
the request budget includes cleanup. Remote termination cannot be guaranteed after
a network partition or process loss. See the [service contract](../contracts/service/v1alpha1/semantics.md)
for the exact budgets. Live timeout, caller-drop and TLS checks run in the SQL adapter's ignored tests.

No service in this slice advertises OAuth acquisition, writes outside a standing grant,
durable events, process execution, or media sessions. Those remain separate contracts in
the full design.
