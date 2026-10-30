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

The served instance lists and runs its reads only. A catalog write is not in its description,
and invoking one answers `not_found` before anything is recorded or sent. Start also refuses:

- an instance that uses more than one profile;
- an instance whose profile's material is acquired by OAuth (the CLI writes that, nothing
  mounts it);
- an instance with alternative bindings.

### Git relay

A catalog instance can also relay git smart-HTTP, so that a caller with no forge credential
clones, fetches and pushes through it with a stock `git` client. The instance adds the token of
its own profile to each request it sends to the forge; the caller never sees it. The relay is on
when the configuration has a fourth member, `git`:

```json
"git": {
  "upstream": "https://github.com",
  "repositories": ["example-org/service"],
  "ref_prefixes": ["agent/"],
  "push_limit_bytes": 104857600,
  "commit_identity": {"name": "Example Automation", "email": "automation@example.com"},
  "pre_push_check": {"command": ["/usr/local/bin/secret-scan", "--policy", "/etc/scan/policy"], "timeout_seconds": 300},
  "work_directory": "/var/lib/connectors/git"
}
```

- `upstream` is the forge's git base over `https`. A repository is
  `<upstream>/<owner>/<repo>.git`. `ca_file`, when set, is the only root of its TLS.
- `repositories` lists the relayed repositories as `<owner>/<repo>`. Any other repository is
  `404`, and nothing is sent to the forge.
- `ref_prefixes` lists the branch prefixes a push may update: `agent/` admits
  `refs/heads/agent/<name>`.
- `push_limit_bytes` is the largest push the relay reads, 100 MiB by default and at most 1 GiB.
- `commit_identity`, optional: the author and the committer every new commit must carry.
- `pre_push_check`, optional: a command every push must pass, with its timeout (300 s by default).
- `work_directory` is a directory the instance owns; start makes it private (mode `0700`). It holds
  one mirror per repository and one quarantine repository per push, which is removed when the push
  is answered.

The caller reaches `<host>/git/<owner>/<repo>.git` and is admitted under the host's admission
profile, as for the operations: `Authorization: Bearer` carries its service account token or the
static bearer (with a stock `git`, `-c http.extraHeader="Authorization: Bearer <token>"`). A
refused caller gets `401` or `503`, before any forge request. The forge receives the instance's
token in HTTP basic authentication (`x-access-token:<token>`): the stored token of a `token`
profile without a prefix, or the installation token of a `github_app` profile. Start refuses any
other profile, and refuses when `git` is not on `PATH`.

Clone and fetch (upload-pack) and both advertisements are relayed as they are. A push
(receive-pack) is read whole, up to `push_limit_bytes`; a larger one is `413`. The relay then
checks, in this order, and answers a refusal for every command of the push, which `git` shows as
`! [remote rejected]`:

1. The ref policy: every command updates a branch under a prefix. A delete, a tag, any other ref,
   a shallow push, a signed push and a push with push options are refused. Nothing is sent to the
   forge.
2. Quarantine: the relay fetches the repository into its mirror, without following redirects,
   indexes the pack into a new repository whose objects borrow the mirror's, and requires every
   new object to be present. Every object in the pack must be reached from a pushed branch or
   already be in the mirror, so the checks below see everything the forge would store.
3. `commit_identity`: every new commit's author and committer.
4. `pre_push_check`: the command runs in the quarantine repository, with the pushed refs applied,
   `GIT_DIR` set and `CONNECTORS_GIT_REPOSITORY` naming `<owner>/<repo>`. Its standard input has
   one `<old> <new> <ref>` line per command. Its environment holds no credential. It runs in its
   own process group, which is killed when it ends or times out. Any exit status but 0 refuses the
   push; the reason the caller sees does not include the command's output, which is logged.

The check runs as the instance's user, so it can read what the instance reads, the mounted
credential included. Configure a check that inspects the pushed objects and never runs them; a
check that builds or tests the pushed tree needs its own sandbox around it.

Only then is the original request sent to the forge's receive-pack, once. The old object id of
each command is the forge's own compare-and-swap. The forge's report goes back to the caller
unchanged, and each command is logged as `applied`, `refused` or `unknown`. When the forge's answer
is lost, the outcome is `unknown`: the caller is told so, and the push is not sent again. A fetch
shows whether it was applied. A push whose caller hangs up after sending it still runs to its end
and is logged. At most 4 pushes are worked on at once; another one is `503`.

## Supported surfaces

| Service | Operations | External binding |
|---|---|---|
| Kubernetes | `resources.list`, `endpoints.discover`, optionally `hosts.discover`, optionally `helm_releases.history`/`.status`/`.values`/`.manifest` | Kubernetes API; namespace/kind restrictions; bearer token and CA; Helm release reads need their own `helm_release_reads` selection |
| SQL | `schema.list`, `query.read`, `database.list`, `table.list`, `table.describe`, `index.list` | PostgreSQL or MySQL; explicit role or user and database; read-only transactions or sessions and deadlines |
| Catalog | The instance's reads; optionally the git relay | The provider's HTTPS API; one profile; credential document from a mounted file; the forge's git smart-HTTP |

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

No service in this slice advertises OAuth acquisition, writes, durable events, process
execution, or media sessions. Those remain separate contracts in the full design.
