# Kubernetes through the local CLI

The current local binding reads the configured namespaces and resource kinds, and
optionally discovers hosts, reads pod logs, lists the contexts of one kubeconfig
and runs a command in a pod under approval, using a saved Kubernetes bearer token. It requires
Linux x86_64 and the [qualified Secret Service binding](local-secret-service.md).
The separately installed older CLI is not upgraded by building this checkout.

Use the optimized build for the local owner. Startup copies and verifies its
executable within a bounded deadline; large unoptimized debug binaries can exceed
that budget on a busy host. Such a refusal grants no provider dispatch.

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-kubernetes
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

An absolute `--config` and `--state-dir` can select a separate private placement.
Existing installations and credentials are not automatically imported.

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks.
Replace the example target and scope with your cluster coordinates:

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

`api_base` is the cluster API root and must be `https`. `namespaces` and
`resource_kinds` are the complete configured scope: a request outside either is
refused before any provider work, and no wildcard or all-namespaces mode is
available in this profile. `resource_kinds` accepts only `pods`, `services`,
`deployments`, `endpointslices`, `replicasets` and `events` (the core/v1 Event
collection that `kubectl get events` reads). The optional `ca_file` names an owner-only PEM
certificate file; supply the cluster CA there when it is not in the system store.

`discover_hosts` is optional and defaults to false. While it is false the adapter
does not advertise `hosts.discover` at all, so node reads cannot be requested.

`helm_release_reads` is optional and defaults to `"off"`. It selects how much of
a Helm release this binding may read:

| Value | Advertised release operations |
|---|---|
| `"off"` | none |
| `"metadata"` | `helm_releases.history`, `helm_releases.status` |
| `"redacted_content"` | those two plus `helm_releases.values`, `helm_releases.manifest` |

Release reads do not touch `resource_kinds`: that enum stays the six kinds above,
and no setting adds a general Secret read.
Adding or changing this field changes the effective document, so the
`configuration_revision` changes with it and must be copied from
`--print-local-bootstrap` again.

Three more fields are optional and each enables one operation. Absent, each is
off and leaves the effective document, and so the `configuration_revision`, as it
was before the field existed; set, it changes the revision.

| Field | Enables | Value |
|---|---|---|
| `pod_logs` | `pods.logs` ([Read pod logs](#read-pod-logs)) | `true`; absent means false |
| `kubeconfig` | `contexts.list` ([List kubeconfig contexts](#list-kubeconfig-contexts)) | the absolute path of an owner-only kubeconfig file |
| `pod_exec` | `pods.exec`, a write ([Run a command in a pod](#run-a-command-in-a-pod)) | `true`; absent means false |

```json
{
  "format": "connectors-kubernetes-local/1",
  "instance": "kubernetes-local",
  "api_base": "https://cluster.example:6443/",
  "namespaces": ["default"],
  "resource_kinds": ["pods", "services", "deployments", "endpointslices"],
  "discover_hosts": false,
  "helm_release_reads": "off",
  "pod_logs": true,
  "kubeconfig": "/absolute/private/kubeconfig",
  "pod_exec": true
}
```

Pod logs are admitted apart from `resource_kinds`, because log text routinely
carries values a pod object does not. The kubeconfig is read and parsed once when
the configuration loads, and a path that is not an owner-only, readable kubeconfig
refuses the configuration; the revision records a digest of the path, not of the
file. The federated service configuration can enable `pod_logs`, but not
`kubeconfig` or `pod_exec`.

The block above is the file format. The adapter's published
`configuration_schema` has two branches, and the local one describes the
**effective** document the composition builds at bootstrap, not this file: there
the CA is reduced to a `ca_digest` and `discover_hosts` is always present. Nothing
validates this file against that schema, so do not write the file to match it.

Inspect the native bootstrap and hash the built executable:

```sh
target/release/connectors-kubernetes --local-config /absolute/path/kubernetes.json --print-local-bootstrap
sha256sum target/release/connectors-kubernetes
```

Copy `configuration_revision` from that output verbatim. It is a digest of the
effective document, which includes a `ca_digest` computed over the CA bytes in a
form of the adapter's own choosing — it is **not** `sha256sum` of the PEM file, and
no command reproduces it independently. Read it from
`--print-local-bootstrap`, never compute it.

Add an adapter entry to the configuration created by setup. Use the bootstrap's
exact `configuration_revision`, the executable's SHA-256 and absolute paths:

```toml
[adapters.cluster]
instance_id = "kubernetes-local"
adapter_id = "kubernetes"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[adapters.cluster.permissions]
profiles = ["kubernetes.token"]
operations = ["resources.list", "endpoints.discover"]

[adapters.cluster.executable]
path = "/absolute/path/connectors-kubernetes"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/kubernetes.json"]
```

Omitted permission sets deny access, and an operation left out of `operations` is
refused at `operations describe` as well as at invocation. Credentials never belong
in TOML, native configuration, executable arguments or environment variables. An
optional top-level `secret_service_socket` selects an explicit private local bus;
it does not waive custody qualification. Replacing a running
artifact/configuration requires exact stop before a subsequent admitted launch.

## Connect and read

Use the hidden native token prompt on your foreground controlling terminal:

```sh
target/release/connectors --output json connections connect --adapter cluster --profile kubernetes.token --credential-prompt
target/release/connectors --output json operations describe --adapter cluster --operation resources.list
```

For automation, `--credential-file /absolute/private/file.json` or
`--credential-stdin` carries the complete native `{"token":"..."}` document.
Files must be singly linked, regular, owner-only (0400 or 0600), with admitted
ancestors. Stdin accepts a deliberate same-session anonymous pipe or admitted
private regular file. Sources are mutually exclusive. Protected values are not
accepted as argv or printed in results.

Retain the connection reference returned by connect, then the descriptor revision
and schema identity returned by operation describe:

```sh
target/release/connectors --output json operations invoke --adapter cluster --connection CONNECTION --operation resources.list --schema SCHEMA --revision REVISION --input-json '{"namespace":"default","kind":"pods","limit":50}'
```

Business JSON also supports `--input-file` and `--input-stdin`. The owner validates
the exact original document and current authority before provider dispatch.

Each CLI invocation may exit independently; the owner and its exact adapter child
remain available. If the owner exits, a later admitted invocation can start it and
reuse the saved exact credential version. Lists, descriptions and status start
nothing. The cache always reports `stale: true` and does not grant permission.

`adapters status --adapter cluster` returns incarnation coordinates. Pass those
exact values to `adapters stop` with `--expected-revision`, `--host-incarnation`
and `--child-incarnation`.

## What the connection is bound to

Connect and repair validate the credential with one Kubernetes
[SelfSubjectReview](https://kubernetes.io/docs/reference/kubernetes-api/authentication-resources/self-subject-review-v1/)
request, issued through a check-specific capability whose endpoint the adapter
composition fixes. Business reads cannot issue it, and it is the only POST this
binding performs.

The credential must be allowed to create that review. A cluster whose RBAC
forbids it answers 403, which is reported as a `forbidden` refusal at connect and
no connection is saved — distinct from a `failure` with code `invalid_credential`
for a token the cluster does not accept, and from `unavailable` for 429 or a 5xx.
A read-only credential that cannot create a SelfSubjectReview therefore cannot
connect at all.

The saved identity is the authenticated **username** the cluster reports, for
example `system:serviceaccount:observability:reader`. RBAC binds to that name, so
it is the authorization-relevant identity; the optional `uid` is
authenticator-dependent and is not used, because a cluster that omits it would
otherwise report an unchanged principal as a switch. A `connections repair`
carrying a different username is refused as an identity mismatch and the existing
valid credential is preserved.

An unauthenticated principal is never saved as an identity. Both the well-known
`system:anonymous` username and the `system:unauthenticated` group refuse on their
own, so a cluster that configures a different anonymous username is still
refused.

Kubernetes issues no scope grant with a token, so the profile declares no required
scopes and the saved connection records none. A successful identity validation is
therefore **not** evidence that any particular read is permitted: an RBAC denial
appears when the read is dispatched, as a forbidden refusal rather than an empty
result.

This binding does not observe a credential expiry. A bound service-account token
carries its own lifetime inside the credential, which this adapter does not decode,
so `credential_expires_at_ms` is absent — meaning not observed, not unlimited. An
expired token is reported when it is next used.

## Read resources, endpoints and hosts

Use `operations describe` for each operation's current schema and descriptor
revision, then invoke with the saved connection as above. The request shapes are:

| Operation | Example input |
|---|---|
| `resources.list` | `{"namespace":"default","kind":"pods","limit":50}` |
| `resources.get` | `{"namespace":"default","kind":"deployments","name":"api"}` |
| `namespaces.list` | `{"limit":50}` |
| `deployments.history` | `{"namespace":"default","name":"api","limit":50}` |
| `endpoints.discover` | `{"namespace":"default","limit":50}` |
| `hosts.discover` | `{"limit":50}` |
| `helm_releases.history` | `{"namespace":"default","release":"api","limit":50}` |
| `helm_releases.status` | `{"namespace":"default","release":"api","limit":50}` |
| `helm_releases.values` | `{"namespace":"default","release":"api","revision":3,"limit":50}` |
| `helm_releases.manifest` | `{"namespace":"default","release":"api","revision":3,"limit":50}` |
| `pods.logs` | `{"namespace":"default","pod":"api-0","container":"api","tail_lines":200}` |
| `contexts.list` | `{"limit":50}` |
| `pods.exec` | `{"namespace":"default","pod":"api-0","container":"api","command":["cat","/etc/hostname"]}` |

For every operation above except `resources.get`, which takes no `limit`, the two
release projections, `pods.logs` and `pods.exec`, which take none, and
`contexts.list`, whose `limit` is between 1 and 256, `limit` is between 1 and 100. For `helm_releases.values` and
`helm_releases.manifest` it is between 1 and 500, because those two page over
one stored object rather than over a provider collection. List results contain
`items`, `next_cursor` and `complete`. Pass a returned cursor alongside unchanged selectors, limit and
connection; a cursor issued under one connection is not readable under another.
Pages are observations of a mutable collection.

`endpoints.discover` reads EndpointSlices in the selected namespace and expands
them into address/port observations carrying the source slice identity, the
service name where the cluster labels one, and the reported readiness. An
expansion over 4096 observations is refused rather than truncated; request fewer
slices per page.

`hosts.discover` reads nodes and is available only while `discover_hosts` is true.

## Read single objects, namespaces and rollout history

The [reads contract](../adapters/kubernetes/contracts/reads/v1alpha1/semantics.md)
states the behaviour of these three reads. Each one is bounded by the configured
`namespaces` and `resource_kinds`. A namespace or kind outside that scope is
refused as `forbidden` before any request.

`resources.get` reads one object of an admitted kind by name and returns a page of
exactly that object, `complete: true` and no cursor. The name must be a DNS-1123
subdomain. An absent object is a `service_failure` refusal with `service_code:
not_found`, not an empty page; use `resources.list` to ask what exists.

`namespaces.list` reads each configured namespace by its exact name, in
configuration order, and returns the ones the cluster has. A configured namespace
the cluster does not have is omitted. Any other refusal, such as a 403, refuses the
whole page, because a denied namespace is not evidence that it is absent. The
cluster's namespace collection is never listed, so this read cannot reveal a
namespace outside the configured set. `limit` bounds how many configured names one
page reads, and the cursor is a position in the configured list.

`deployments.history` needs both `deployments` and `replicasets` in
`resource_kinds`. It reads the Deployment, which answers `not_found` when absent.
It then lists ReplicaSets matching the Deployment's own selector and keeps only
those whose controlling owner is that Deployment by uid; a label match alone is
not ownership. Each item is the full ReplicaSet, and its revision is the
`deployment.kubernetes.io/revision` annotation. A selector this binding cannot
carry exactly is refused before the list. The controller prunes ReplicaSets beyond
`spec.revisionHistoryLimit`, so a missing revision may be pruned or may never have
existed, and the page cannot say which.

Events are read with `resources.list` and `"kind":"events"`, and ReplicaSets with
`"kind":"replicasets"`, once those kinds are configured.

Add each operation you use to `operations` in the adapter's permissions, as in the
configuration above. The credential needs this RBAC in every configured namespace
it reads:

| Operation | RBAC rule (namespaced Role) |
|---|---|
| `resources.get` | `get` on the requested kind (`pods`, `services`, `events`, `endpointslices` in `discovery.k8s.io`, or `deployments`, `replicasets` in `apps`) |
| `namespaces.list` | `get` on `namespaces` in each configured namespace |
| `deployments.history` | `get` on `deployments` and `list` on `replicasets`, both in `apps` |
| `resources.list` of `events` | `list` on `events` (core group) |
| `resources.list` of `replicasets` | `list` on `replicasets` in `apps` |

The authorization namespace of a namespace object is its own name, so a Role in
namespace `default` granting `get` on `namespaces` is enough to read the
`default` namespace. No ClusterRole is needed, and the binding never asks for
`list` on namespaces.

## Read pod logs

`pods.logs` reads a bounded tail of one pod's log in a configured namespace, as
the [logs contract](../adapters/kubernetes/contracts/logs/v1alpha1/semantics.md)
states. It is advertised only while `pod_logs` is true. The input is closed:
`namespace` and `pod` are required; `container` is optional, and without it the
cluster chooses the pod's default container, so name it for a pod with several.
`since_seconds` (1–86,400, default 86,400), `tail_lines` (1–1,000, default 200)
and `max_bytes` (1–131,072, default 131,072) bound the read. There is no cursor,
no follow and no previous container.

The adapter makes one log GET with timestamps and returns the lines in the order
the cluster sent them, each with `timestamp_unix_ns` (null when a line carries no
timestamp), its `stream` (namespace, pod, container), the line text and
`line_truncated`. A line is clipped at 8 KiB. `complete` is true only when the
read ended below both the line and byte bounds; reaching either is reported in
`truncation.causes`. A cut never splits a UTF-8 character; other invalid UTF-8
refuses the read as `unavailable`. A namespace outside the configuration is
refused with no request; the cluster's 403 is `forbidden` and its 404
`not_found`, never empty logs. The credential needs `get` on `pods/log` in the
namespace. The per-read permission pre-check the contract describes is not
performed, as for every read of this binding.

## List kubeconfig contexts

`contexts.list` lists the contexts of the kubeconfig named by the configuration's
`kubeconfig`, and is advertised only when it names one. Its only input is `limit`
(1–256); no input can name a file, so a connection reads the file it was
configured with and no other. The file is read again at each call, so a context
added later or a changed `current-context` shows on the next read; no cluster
request is made and the credential is not used.

Each item is `name`, `cluster` (the cluster entry's name), `namespace` (null when
the context sets none) and `current` (true for the context `current-context`
names). Users, servers, certificate authorities, client certificates and keys,
tokens, exec plugins and auth providers are never read into the result. A file
over 1 MiB or with more than 256 contexts, a repeated context name, a name over
256 bytes or a namespace over 63 refuses the whole file. A smaller `limit` returns
the first contexts with `complete: false` and no cursor. A file missing, or no
longer owner-only, at the call is `unavailable`. The list does not create a
connection from a context: each connection is still configured and connected as
above.

## Run a command in a pod

`pods.exec` runs one command in one named container of a pod in a configured
namespace, as the [mutation contract, §4.2](../adapters/kubernetes/contracts/mutations/v1alpha1/semantics.md#42-pod-exec-podsexec)
states. It is a write: it is advertised only while `pod_exec` is true, only on the
`connectors-private/2` write exchange, and the read exchange and the federated
service never list it. The adapter entry needs `private_protocol =
"connectors-private/2"` and `pods.exec` in its `operations`, and the operation
needs an approval policy and an issued approval like every write
([local approvals](local-approvals.md)). The host spends the approval and records
the attempt before the adapter opens the stream.

The input is closed: `namespace`, `pod`, `container` (required, no default
container) and `command`, an argument vector of 1–64 elements, each 1–4,096
bytes and at most 16,384 in all, whose first element is the executable; no shell
is added. `timeout_seconds` (1–60, default 30) bounds the run and
`max_output_bytes` (1–1,048,576, default 65,536) bounds stdout and stderr each.
There is no stdin and no terminal.

The adapter sends one GET to the pod's `exec` subresource, upgraded to a WebSocket
through the host's admitted connection to the configured API server (no redirect
followed), and reads the cluster's status message. The result is `namespace`,
`pod`, `container`, `exit_code`, `stdout`, `stderr`, `stdout_truncated` and
`stderr_truncated`. A non-zero exit is an applied attempt with that `exit_code`,
not a refusal. An upgrade the cluster refuses with 400, 401, 403, 404 or 422 is a
refusal and no process ran. A lost stream, an answer without a status, a 5xx or
the deadline is `unknown`: the command may have run, and it is never sent again;
running it again takes a new approval. The credential needs the cluster's
permission on the `pods/exec` subresource in the namespace.

## Read Helm releases

The four `helm_releases.*` operations read the Secrets a Helm release is stored
in, one per revision, named `sh.helm.release.v1.<release>.v<revision>`. The
[native contract](../adapters/kubernetes/contracts/helm/v1alpha1/semantics.md)
states the behaviour and the
[pinned Helm sources](../adapters/kubernetes/contracts/helm/v1alpha1/evidence/20260912/provider-sources.md)
state where every field comes from.

`helm_releases.history` returns the release's revisions, each carrying the exact
Secret it was read from, its revision number, its status and the timestamps Helm
recorded. It pages with `limit` and a cursor like the other list reads.
`helm_releases.status` is the same read restricted to the revisions the store
marks `deployed`; it reports all of them rather than choosing one, because a
concurrently written store can hold more than one.

`helm_releases.values` and `helm_releases.manifest` read one revision and return
a **redacted projection, never the stored content**. A release's recorded values
routinely contain credentials, and its rendered manifest contains the body of
every Secret the release applied. `values` returns one entry per recorded path it
can represent, with its JSON shape and a SHA-256 over the canonical JSON of its
value; `manifest` returns one entry per rendered document with its position,
its byte length as stored and a SHA-256 over exactly those stored bytes, which
`sha256sum` on the extracted document reproduces — nothing is stripped, so the
newline Helm writes at the end of every document is counted and digested. No recorded scalar and no manifest byte
is returned, and there is no setting that returns one. Use the digests to tell
whether something changed between revisions; to read the value itself, use your
own cluster credentials directly.

Both digests are unsalted, which is what makes one comparable and the other
reproducible — and means **either digest confirms a guess**. Anyone holding a
`value_digest` can test a candidate literal offline with one `sha256sum` and
learn whether it is right. The same is true of `content_digest`: a chart's
templates are usually public, so the unknown part of a rendered document may be
just the value injected into it, and the `bytes` field publishes that
document's exact length, which narrows the search further. Both projections
therefore bound disclosure of something nobody can enumerate; neither protects
a short, guessable or already-suspected value. Treat the output as you would
treat the list of keys in a values file, not as a secret.

Both projections read a single object, so they never issue a cursor: a
projection larger than `limit` comes back with `complete: false` and
`next_cursor: null`.

**Raising `limit` is not the only reason a projection is incomplete.**
`helm_releases.values` also reports `complete: false` when a recorded path
cannot be represented inside the 1024-byte bound the operation publishes for
`path` — an empty recorded key, or keys that concatenate past that length. Such
a node is dropped together with everything under it, because returning it would
produce a result the adapter's own published schema rejects. Raising `limit`
will not bring it back. A `complete: false` page is evidence about what you
did receive and none at all about what you did not: a path that is missing from
an incomplete page is not an unset value.

A namespace outside the configured scope is refused with no request at all. A
namespace whose release Secrets the credential may not read is refused too,
after one request, by the cluster's own RBAC. Neither is ever reported as an
empty history — a release with no stored revisions returns an empty, complete
page.

## Limitations

This binding advertises up to twelve read operations and one write, `pods.exec`.
Events are read as a list of core/v1 Event objects in one namespace; there is no
watch, and no read filters events by the object they involve. Conditions, log
following, interactive exec (stdin or a terminal), copy, port forwarding and
every other mutation are not implemented. `deployments.history` reports the
stored ReplicaSets; it does not diff revisions or roll one back. `pods.logs`,
`contexts.list` and `pods.exec` are verified against recorded API answers and
streams, not a live cluster.

Of the Helm surface, only release-state reads exist. Installing, upgrading,
uninstalling and rolling back a release are not implemented; neither are chart
rendering, linting, packaging and registry access, which are local tool work
that never reaches a cluster. `helm list` — every release in a scope, rather
than one release's revisions — is not implemented either and would need its own
selection and admission. A release's stored chart and hooks are not read, and a
manifest document's own resource identity is not reported, because this binding
has no YAML reader and will not guess one from lines.

Per-operation permission pre-checks are not performed. The native
[authorization contract](../adapters/kubernetes/contracts/auth/v1alpha1/semantics.md)
specifies a SelfSubjectAccessReview target set, fan-out budget and an
`authorization` coverage payload for multi-target reads; that profile is not
implemented and no result claims coverage. A read is dispatched and the cluster's
own RBAC decision is reported.

The [recorded provider restart checks](evidence/provider-restarts-20261002/README.md)
include a task-owned Kubernetes sandbox. Real-provider acceptance is separate from
the deterministic TLS fixture checks and requires explicit disposable resources.

## Run the bounded real-provider acceptance

The four `cli_journey::kubernetes_cli_*` acceptance cases in the `local_runtime`
test target exercise read reuse after owner/keyring restart, authenticated RBAC
denial, real continuation binding, and repair/revocation/stop behavior. They are
ignored by default and fail when explicitly selected without their prerequisites.

Build the optimized production CLI and adapter first. Set `CONNECTORS_TEST_CLI`
to the absolute CLI binary path, `CONNECTORS_K8S_SANDBOX` to the disposable HTTPS
API root, and `CONNECTORS_K8S_CA`, `CONNECTORS_K8S_TOKEN`, and
`CONNECTORS_K8S_KUBECONFIG` to protected files. The token file contains the raw
bearer token; the kubeconfig is used only for fixture provisioning. Never select
a default kubeconfig or place a token in arguments. Use an owner-private, short
physical `TMPDIR` and a separate Cargo target for the checkout.

The fixture requires `/usr/bin/kubectl`, `/usr/bin/dbus-daemon`, and
`/usr/bin/gnome-keyring-daemon`. It owns a private bus/keyring, uniquely named API
objects and namespaced Roles/RoleBindings in `fixture` and
`fixture-cb26d-empty`. The latter namespace's Service collection must be empty.
The `fixture/connector-reader` service account needs SelfSubjectReview and the
explicit node get/list grant for the host-discovery case; the test does not create
cluster-scoped permissions. Provision these disposable namespaces and cluster
permissions deliberately before running. A different pods-only service account
provides the authenticated Services-denial control.

The test-only loopback HTTPS observer verifies the real upstream CA, forwards
only the selected reads and SelfSubjectReview, disables redirects/retries, and
records method/path/status without credentials or query values. It counts before
forwarding, so a zero-request assertion includes unfinished requests. A bounded
barrier can hold one completed real response for a lifecycle race; it does not
simulate a provider failure. The fixture records exact object identities and
revisions and deletes only its own objects. Its deliberately unschedulable Pod
and zero-replica Deployment establish API-object existence, not workload readiness.

Select each of these exact names separately, using `--exact --ignored --nocapture
--test-threads=1` with `cargo test --locked --release -p connectors-kubernetes
--test local_runtime`:

- `cli_journey::kubernetes_cli_reuses_each_admitted_read_after_restart`
- `cli_journey::kubernetes_cli_provider_rbac_denial_is_not_empty_success`
- `cli_journey::kubernetes_cli_continuation_preserves_scope_and_revision`
- `cli_journey::kubernetes_cli_repair_revoke_and_stop_preserve_authority`

Retain each runner result and fixture cleanup outcome. A successful deterministic
fixture run or a missing sandbox does not count as these real-provider cases.
