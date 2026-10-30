# Run connector services

These instructions describe the implemented first-slice runtime. Run commands from
the repository root after [building the workspace](development.md). For a complete
disposable environment, follow the [live acceptance recipe](live-e2e.md).

Connector, instance and connection are the terms of the [vocabulary](../README.md#vocabulary).

Copy the matching file from `examples/`, choose a unique instance/listener, configure
the permitted resources, and bind private credential files or environment references.
Credential files must be regular files owned by the running user with mode `0600`; the
TokenReview `credential_file` alone may also be `0640` or `0440` (Admission, below).
Credentials never belong in operation input. Config changes require a restart.
Credential file replacement is resolved on subsequent requests, including gateway
downstream credentials. Environment references reflect the process environment;
changing a parent shell's environment cannot update an already running service.

```sh
target/debug/connectors-kubernetes --config kubernetes.yaml
target/debug/connectors-sql --config sql.yaml
target/debug/connectors-catalog-provider --config catalog.json
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
    "credential_file": "/var/run/secrets/kubernetes.io/serviceaccount/..data/token"
  }
}
```

`credential_file` names `..data/token` because the conventional `serviceaccount/token` is a
symbolic link to it, and the host refuses a symbolic link as the last path component. The
token must be a regular file of at most 8 KiB, either `0600` and owned by the running user,
or exactly `0640` or `0440`, as the kubelet presents it to a pod with `fsGroup`: owned by
the running user or root, with a group the process holds as its effective or a
supplementary group. Any other mode with a bit for group or others is refused, among them a
group-writable file, one with any bit for others, a setuid, setgid or sticky bit and owner
execute beside group read, and so is a `0640` file whose group the process does not hold;
every other credential file keeps the `0600` rule.

The host admits a request only when the review authenticates the token for `audience` as
an admitted service account. Name the accounts in exactly one of three ways:

- `principal`: one account, `system:serviceaccount:<namespace>:<name>`, at most 128 bytes.
- `principals`: a list of 1 to 64 such accounts. Each is admitted, and no other.
- `service_accounts`: a namespace and a name pattern, such as
  `{"namespace": "agents", "name": "*-agent"}`. The namespace is a literal DNS label, with
  no wildcard. The name is 1 to
  128 bytes of `a-z`, `0-9`, `-`, `.` and `*`, and `*` is its only wildcard: it matches any
  run of characters, or none. The pattern must match the whole name, so `*-agent` admits
  `coding-agent` but not `coding-agent-2`. With each `*` read as one letter, the pattern
  must be a valid account name, and the whole account
  `system:serviceaccount:<namespace>:<name>` must fit in 128 bytes. A pattern such as `-*`,
  `*-` or `a..b`, which no account name can match, is refused.

None of the three, more than one of them, an empty list or one longer than 64, an invalid
entry and an empty or invalid pattern are refused at start, with a message that names the
key. A key written with no value (`principals:` in YAML, `null` in JSON) is refused, not
read as absent.

A review that names any other account, another audience or no authenticated user is
`unauthorized`. A review that fails, takes longer than
5 seconds or cannot be reached is `unavailable`. In either case no operation runs. The host
sends one review per request and keeps none. At most 8 reviews run at once: a request that
arrives while 8 are under way is `unavailable` at once and sends no review, so a caller can
retry it. The host never logs the token. Its own token,
`credential_file`, is read at each review, so a projected token that the kubelet rotates is
used from the next review on. The host's service account needs the `system:auth-delegator`
cluster role, or another role that may create `tokenreviews`. Under this profile
`GET /v1/describe` is admitted the same way, and `POST /v1/invoke` answers `unsupported`.
Callers invoke through `POST /v1alpha2/invoke`, which records the service account the
review named, its username verbatim, as the principal of every call.

Under `static-bearer`, a caller presents the service credential as a bearer token, reads the
current descriptor with `GET /v1/describe` and names its revision in `POST /v1/invoke`:

```sh
token="authorization: Bearer $(cat service.secret)"
revision=$(curl -s -H "$token" http://127.0.0.1:7102/v1/describe | jq -r .revision)
jq -n --arg revision "$revision" --slurpfile input examples/requests/kubernetes-endpoints.json \
  '{version: "v1alpha1", request_id: "discover-1", operation: "endpoints.discover",
    revision: $revision, input: $input[0]}' \
  | curl -s -H "$token" -H 'content-type: application/json' --data @- \
    http://127.0.0.1:7102/v1/invoke
```

The `connectors` command does not call a standalone service: its `describe`, `invoke --endpoint`
and `serve` commands were removed in 0.60.0. The local CLI reaches a provider through a configured
instance; `connections connect <connector>/<instance> --endpoint <url>` saves the provider
address there.

GitLab is served by the [catalog provider](local-catalog-provider.md): under the local CLI,
or on the host as a [catalog instance](#catalog-instances).

A federation host puts several services behind one endpoint. It is the library
`connectors_host::federation`, which a Rust program embeds and serves with
`connectors_host::server::serve`; `examples/federation.yaml` is its configuration, and
[Federate services](../website/docs/guides/federate-adapter-services.md) shows the program's
shape. Through it the operation above is `kubernetes__endpoints.discover`. Source prefixes
distinguish instances; the host does not interpret provider data. It supports one federation hop
and never retries a provider request.

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

  `catalog.repositories`, optional, lists the repositories the instance's operations reach,
  as `<owner>/<repo>`, for example `["example-org/service"]`. Entries are read as
  [`git.repositories`](#git-relay) is: owners and names compare without case and a trailing
  `.git`, in any case, is dropped. Start refuses an empty list, `null`, or an entry that is
  not `<owner>/<repo>`.
  With the list set:
  - an operation whose input has both an `owner` and a `repo` member, a write included,
    runs only for a listed repository. Any other repository is `not_found`, with the
    message a repository the provider does not have gets, and the instance neither reads
    its credential nor sends a request. A write is refused before the host records it, so
    its key is not used;
  - every other operation (a search, notifications, an organization listing, the user
    read) is left out of the description, so the host answers `not_found`. A page
    filtered to the listed repositories could be shorter than its page size, and a
    caller walking the pages would stop early.

  Without the list every described operation runs for any repository the credential
  reaches.
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

### The state directory

`service.state` is the host's metadata directory. Start creates it with mode `0700` when it is
absent and never changes an existing directory's mode. Start checks every directory from `/`
down to `state`: each must be owned by the host user or root and must not be writable by group
or others, and `state` itself must be owned by the host user with mode `0700`. The refusal names
the first directory that fails and why, for example:

```text
service state cannot be opened for this instance: "/var/lib/connectors", an ancestor of state, is writable by group or others
```

For `state` itself it states the whole target: owned by the host user with mode `0700`, no
group or other bits. A directory the host cannot open at all (one it has no search permission
on, or a missing parent it cannot create in) refuses start with the closed text
`service state cannot be opened for this instance`, without a name.

A persistent volume whose root is `0777` therefore refuses start when it is mounted at
`state` or above it, however private `state` is. Keep that root out of the path in one of
these ways:

- mount a subdirectory of the volume (`subPath`) as `state`: the host user's, mode `0700`;
- mount a subdirectory of the volume as the parent of `state`: owned by the host user or root,
  not group- or other-writable, and searchable by the host user;
- have an init container set the volume root to `0755`, searchable by the host user, owned by
  the host user or root, before the host starts.

An `fsGroup` makes the volume group-writable, and it is refused the same way.

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

The `state` directory holds at most 100,000 recorded attempts per instance, and the host never
removes one: keyed attempts do not expire on the HTTP host. Once an instance has 100,000,
every further write a grant covers answers `capacity`, records no attempt and sends nothing;
a replay of a recorded key still answers its outcome. Writes resume only when the operator
moves the host to a fresh `state` directory. The keys and hourly counts of the old one are not
carried over, so a key used before is admitted there as a new write.

`POST /v1/invoke`, as in the `curl` example above, carries no key, so on a host with standing
grants it runs reads only. A Rust caller sends a keyed write with `Client::invoke_v1alpha2_keyed`;
`Client::invoke_v1alpha2` sends no key, which such a host refuses for a write as
`invalid_input`.

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
  `404`, and nothing is sent to the forge. The relay reads this list only; operations read
  [`catalog.repositories`](#catalog-instances), and neither list is read for the other. To
  keep both to the same repositories, list them in both.
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
other profile, and refuses when `git` is not on `PATH`. The relay runs on Unix; elsewhere start
answers `unsupported` to a `git` member.

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
| Catalog | The instance's reads; its writes under a standing grant file; optionally the git relay | The provider's HTTPS API; one profile; credential document from a mounted file; the forge's git smart-HTTP |

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
for the exact budgets. Live timeout, caller-drop and TLS checks run in the ignored tests of `adapters/sql`.

No service in this slice advertises OAuth acquisition, writes outside a standing grant,
durable events, process execution, or media sessions. Those remain separate contracts in
the full design.
