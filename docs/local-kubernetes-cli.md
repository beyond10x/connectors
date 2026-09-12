# Kubernetes through the local CLI

The current local binding reads the configured namespaces and resource kinds, and
optionally discovers hosts, using a saved Kubernetes bearer token. It requires
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
`deployments` and `endpointslices`. The optional `ca_file` names an owner-only PEM
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

Release reads do not touch `resource_kinds`: that enum stays `pods`, `services`,
`deployments` and `endpointslices`, and no setting adds a general Secret read.
Adding or changing this field changes the effective document, so the
`configuration_revision` changes with it and must be copied from
`--print-local-bootstrap` again.

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
| `endpoints.discover` | `{"namespace":"default","limit":50}` |
| `hosts.discover` | `{"limit":50}` |
| `helm_releases.history` | `{"namespace":"default","release":"api","limit":50}` |
| `helm_releases.status` | `{"namespace":"default","release":"api","limit":50}` |
| `helm_releases.values` | `{"namespace":"default","release":"api","revision":3,"limit":50}` |
| `helm_releases.manifest` | `{"namespace":"default","release":"api","revision":3,"limit":50}` |

For every operation above except the two release projections,
`limit` is between 1 and 100. For `helm_releases.values` and
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
every Secret the release applied. `values` returns one entry per recorded path
with its JSON shape and a SHA-256 over the canonical JSON of its value;
`manifest` returns one entry per rendered document with its position, byte
length and a SHA-256 over exactly the document text those bytes count, which
`sha256sum` on the document reproduces. No recorded scalar and no manifest byte
is returned, and there is no setting that returns one. Use the digests to tell
whether something changed between revisions; to read the value itself, use your
own cluster credentials directly.

Both digests are unsalted, which is what makes them comparable — and means
**a digest confirms a guess**. Anyone holding a `value_digest` can test a
candidate literal offline with one `sha256sum` and learn whether it is right.
The projection therefore bounds disclosure of a value nobody can enumerate; it
does not protect a short, guessable or already-suspected one. Treat the output
as you would treat the list of keys in a values file, not as a secret.

Both projections read a single object, so they never issue a cursor: a
projection larger than `limit` comes back with `complete: false` and
`next_cursor: null`.

A namespace outside the configured scope is refused with no request at all. A
namespace whose release Secrets the credential may not read is refused too,
after one request, by the cluster's own RBAC. Neither is ever reported as an
empty history — a release with no stored revisions returns an empty, complete
page.

## Limitations

This binding advertises up to seven read operations. Kubernetes events,
conditions, pod logs, exec, copy, port forwarding and every mutation are not
implemented.

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

No runtime evidence against a dedicated Kubernetes sandbox has been recorded yet.
The journeys above are verified against a local TLS fixture cluster and a
disposable Secret Service.
