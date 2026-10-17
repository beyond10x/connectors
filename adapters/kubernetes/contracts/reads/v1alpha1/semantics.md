# kubernetes implemented read profiles/v1alpha1

This adapter owns the native behavior of its existing local implementation.
Shared wire envelopes and admission remain in
[service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md).
This relocation changes no runtime or selected codec.

Kubernetes: paginated allowed resource lists, EndpointSlice-derived endpoint
observations, and explicitly enabled node/host discovery. Namespace and kind
allowlists precede HTTP dispatch. ResourceVersion and continue tokens remain
observable; expired snapshots return a stale-cursor failure. Endpoint observations
carry source UID, namespace, address/port/protocol, service association, readiness
and observation revision. Classifications are candidates, not connection grants.
Discovery never dials, authenticates to, or materializes the observed endpoint.
Source facts: https://kubernetes.io/docs/reference/using-api/api-concepts/ and
https://kubernetes.io/docs/concepts/services-networking/endpoint-slices/.
Source access: 2026-09-08.

## Single objects, namespaces, events and rollout history

The typed selections are in the adapter's ESS model
([reads domain](../../../spec/ess/domains/reads.yaml)). Every read below keeps the
rules above: the configured namespaces bound it, the configured `resource_kinds`
admit each kind it reads, it writes nothing, and items are full provider objects.
Each scope refusal is `forbidden` and precedes every provider request.

- **Kinds.** `resource_kinds` also admits `replicasets` (apps/v1) and `events`
  (core/v1 Event, the collection `kubectl get events` reads). `resources.list`
  reads them like any other kind: one bounded page, no watch.
- **`resources.get`** (`kubernetes-get`) reads one object of an admitted kind by
  name: `GET <collection>/<name>`. The name must be a DNS-1123 subdomain and is
  refused as `invalid_input` otherwise, so it never adds a path segment. A 404
  answers `not_found`, distinct from an empty `resources.list` page. The result is a
  page of exactly one item with `complete: true`, no cursor and the object's own
  `resourceVersion` as its source revision; an object reporting another name or
  namespace is `upstream_protocol`. A separate operation, rather than a `name` input
  on `resources.list`, keeps a list's empty page and a missing object distinct.
- **`namespaces.list`** (`kubernetes-configured-namespaces`) reads each configured
  namespace by exact name (`GET /api/v1/namespaces/<name>`) in configuration order
  and lists the present ones. A 404 is an absent namespace and is omitted; any other
  refusal refuses the page, because a denied namespace is not evidence of absence.
  The cluster-scoped namespace collection is never listed, so a namespace outside
  the configured set can neither be read nor observed: namespace authority is the
  configured set it was before. `limit` bounds the configured names read per page;
  the continuation is an adapter-issued position bound to the configured list and
  page size, not a provider token. The page carries no collection revision.
- **`deployments.history`** (`kubernetes-rollout-history`) needs both `deployments`
  and `replicasets` admitted. It reads the Deployment (a 404 is `not_found`), lists
  ReplicaSets with the Deployment's own `spec.selector` translated to a label
  selector, and keeps a ReplicaSet only when an `ownerReferences` entry has
  `controller: true`, kind `Deployment` and the Deployment's uid; a label match
  alone is not ownership. A selector this binding cannot carry exactly (empty, or a
  key or value outside the label grammar) refuses before the list. Each item's
  revision is its `deployment.kubernetes.io/revision` annotation. Items keep
  provider order; the provider's continuation and collection revision are carried as
  for `resources.list`, and the continuation is bound to the Deployment's uid. The
  controller prunes beyond `spec.revisionHistoryLimit`, so a missing revision is
  pruned or never existed, and the page cannot say which.

Required RBAC, per configured namespace: `get` on `namespaces` for
`namespaces.list` (a namespace object's authorization namespace is its own name),
`get` on the admitted kind for `resources.get`, `list` on `replicasets` and
`events` for those kinds, and `get` on `deployments` with `list` on `replicasets`
for `deployments.history`.

## Statefulsets, nodes and custom resources

- **`statefulsets`** is an admissible `resource_kinds` entry (apps/v1,
  `/apis/apps/v1/namespaces/<ns>/statefulsets`). `resources.list` and
  `resources.get` read it like any other kind. Required RBAC: `list` (and `get`
  for `resources.get`) on `statefulsets` in each configured namespace.
- **`nodes.list`** (`kubernetes-nodes-list`) lists `GET /api/v1/nodes` one
  bounded page at a time (`limit` 1–100, provider continuation and collection
  revision as for `resources.list`), each item the full Node object. Nodes are
  cluster-scoped, so the read takes no namespace and needs cluster-wide `list` on
  `nodes`; it is a separate admission, `node_reads: true`, never implied by the
  configured namespaces. Absent, the operation is not advertised and an invocation
  is `forbidden` with zero requests. It is distinct from `hosts.discover`, which
  reads the same collection under `discover_hosts` and returns a fixed projection;
  enabling either does not enable the other, and a continuation issued by one is
  not readable by the other.
- **`custom_resources.list`** (`kubernetes-custom-resources`) lists one namespaced
  custom resource collection, `GET /apis/<group>/<version>/namespaces/<ns>/<plural>`,
  one bounded page at a time, items the full objects. Its input is `namespace`,
  `group`, `version`, `plural`, `limit` (1–100) and `cursor`. The configuration
  admits each collection explicitly in `custom_resources` (at most 32 distinct
  `{group, version, plural}` entries); the operation is advertised only when it
  admits at least one. A selection that is not a configured entry, or a namespace
  outside the configured set, is `forbidden` before any request. A configured
  `group` must be a DNS-1123 subdomain containing a dot and must not be `k8s.io`,
  `kubernetes.io` or a subdomain of either, which is Kubernetes' own rule for a
  custom resource's group and keeps every built-in collection, Secrets included,
  out of this allowlist; `version` and `plural` are DNS-1123 labels. A
  configuration breaking any of these is refused at load. A configured collection
  the cluster does not serve answers the provider's 404 as `not_found`, never as an
  empty page. The continuation is bound to the exact collection, so a cursor from
  one custom resource cannot page another. Cluster-scoped custom resources are not
  admitted. Required RBAC: `list` on `<plural>` in API group `<group>` in each
  configured namespace it is read in.

## Exec-plugin credentials

A kubeconfig user that authenticates through an `exec` credential plugin (for
example `aws eks get-token`) is supported in one pinned form, decided on
2026-10-15 (`decision-blocker:kubernetes-exec-credential-plugin`, option b) inside
the bounded pinned-binary process family of 2026-10-03. Running whatever the
kubeconfig names is refused. The typed binding, selection, lifetime and refusals
are the [`exec_credentials` ESS domain](../../../spec/ess/domains/exec_credentials.yaml).

- **Only by explicit configuration.** The local native configuration names an
  `exec_plugin` binding: `path`, the plugin's absolute path; `sha256`, the hex
  digest of its bytes; `user`, the kubeconfig `users` entry whose `exec` block
  supplies the arguments; `env`, the allow-list of environment variable names
  (each `ExecPluginEnvName`: never `HOME` or `KUBERNETES_EXEC_INFO`, and never
  one of these names, which load code or change how a loader or shell starts:
  the dynamic loader's `LD_*`; the interpreter and runtime start-up names
  `PYTHON*`, `PERL*`, `RUBY*`, `NODE_*`, `DOTNET_*` (`DOTNET_STARTUP_HOOKS`
  among them), `COREHOST_*`, `LUA_INIT`, `LUA_INIT_*`, `JAVA_TOOL_OPTIONS`,
  `_JAVA_OPTIONS`, `JDK_JAVA_OPTIONS`, `CLASSPATH` and `TCLLIBPATH`; the shell
  start-up and parsing names `BASH_ENV`, `ENV`, `SHELLOPTS`, `BASHOPTS`, `PS4`,
  `IFS`, `CDPATH`, `GLOBIGNORE`, `PROMPT_COMMAND`, `ZDOTDIR` and `FPATH`; the
  library module and configuration names `OPENSSL_CONF`, `OPENSSL_MODULES`,
  `OPENSSL_ENGINES`, `GIO_MODULE_DIR` and `GTK_MODULES`; and the names glibc
  drops for a set-ID program other than `LD_*`, `TMPDIR` and `TZDIR`:
  `GCONV_PATH`, `GETCONF_DIR`, `GLIBC_TUNABLES`, `HOSTALIASES`, `LOCALDOMAIN`,
  `LOCPATH`, `MALLOC_TRACE`, `NIS_PATH`, `NLSPATH`, `RESOLV_HOST_CONF` and
  `RES_OPTIONS`. `TMPDIR` and `TZDIR` load no code and are admitted. The
  contract refuses exactly the listed names and prefixes; it does not claim
  that no other variable can influence a plugin, and a name not listed that a
  particular plugin or library reads is the operator's to allow-list or not. A
  binding naming a listed one is refused when the configuration loads);
  optionally `home`, an absolute directory; and optionally `timeout_ms` (1 to
  60000, 10000 when omitted). It requires `kubeconfig`. No default configuration
  and no federated service configuration enables it. Absent, no plugin ever runs,
  the `kubernetes.exec_plugin` profile is not offered and validating it is
  `unsupported`. The binding enters the effective configuration as
  `exec_plugin_digest`, so any change to it changes the configuration revision.
- **The profile.** With a binding, the bootstrap offers `kubernetes.exec_plugin`
  instead of `kubernetes.token`, and every operation requires it. Its scheme is
  `http_bearer`: the plugin's token is sent as a bearer token. Its one protected
  entry field, `user`, is a confirmation, not a secret: it must equal the
  binding's `user`, so connecting is an explicit act that names the kubeconfig
  user the plugin runs for. The SelfSubjectReview identity probe and every rule of
  the token profile apply to the plugin's token unchanged.
- **The executable.** Before every run the adapter opens `path` once, without
  following a symbolic link as its final component, refuses a file that is not
  regular, is writable by group or others, carries the set-user-ID or
  set-group-ID bit or file capabilities (a `security.capability` extended
  attribute), or is owned by a user other than root and the adapter's own,
  and hashes its bytes through that open descriptor; a digest that is not
  `sha256` refuses before any process starts. The process executes that same
  descriptor (`fexecve`), never `path` again, so a file swapped in at `path`
  after the check never runs. `path` must therefore name the file itself, not a
  symbolic link to it. A script plugin (`#!`) keeps the descriptor open across
  the exec, because its interpreter reads the script through `/dev/fd`; a native
  executable inherits no descriptor. Besides that one descriptor and its
  standard input, output and error, the plugin inherits no descriptor of the
  adapter: every other one is closed on the exec, whether or not the adapter
  opened it close-on-exec. Limitation: on a kernel before Linux 5.11, which
  cannot mark a range close-on-exec in one call, the adapter marks descriptors
  one by one up to the open-files hard limit, capped at 1048576 (2^20, the
  default `fs.nr_open`); where `fs.nr_open` was raised and the adapter holds a
  descriptor numbered above that cap, that descriptor is not marked. Running a
  plugin needs Linux; elsewhere it
  is `plugin_unavailable`. The kubeconfig's `command` must equal `path` byte for
  byte (a trailing or repeated slash is another spelling and refuses) or be a
  bare file name equal byte for byte to the path's file name.
- **Arguments and environment.** The kubeconfig is read when the plugin runs,
  and only then: a use that a held token serves does not read it. Its
  `args` are the argument vector as data; no shell runs. The plugin starts from an
  empty environment and receives exactly: `HOME` when `home` is configured (it is
  then also the working directory, otherwise `/`), `KUBERNETES_EXEC_INFO`
  (`apiVersion`, `kind: ExecCredential`, `spec.interactive: false`), and each
  allow-listed name, valued from the kubeconfig's `exec.env` entry or, failing
  that, from the adapter's own environment (which the local owner clears). A
  kubeconfig `exec.env` name outside the allow-list refuses rather than being
  dropped. `interactiveMode: Always` refuses; `provideClusterInfo` is not
  honoured. stdin is closed and stderr is discarded. File reads beyond `HOME` are
  not sandboxed: the allow-list bounds what the plugin is told, not what the
  operating system lets it open.
- **When it runs.** Only inside an admitted credential validation or operation:
  never while loading the configuration or answering a description. One run is
  bounded by `timeout_ms`; at the deadline the plugin's whole process group and
  the plugin's own process are killed, and the caller is released then, whatever
  group the plugin's process moved itself to, and even when a descendant that
  left the group still holds stdout open (its read end is closed). stdout is
  bounded to 65536 bytes; more kills the process group and the plugin. The run
  ends when stdout is closed and the plugin's process has exited; whatever the
  outcome, the group and the plugin's own process are then each killed once,
  while that process is still unreaped so its id cannot name another process or
  group, and neither is signalled after it is reaped. Limitation: a descendant
  that left the plugin's process group is not followed; it is not killed and
  only loses its stdout.
- **The token.** The answer must be an `ExecCredential` of the requested
  `apiVersion` (`client.authentication.k8s.io/v1` or `v1beta1`) whose
  `status.token` is 1 to 8192 printable ASCII bytes; a client certificate answer
  is not admitted. The token is held in adapter memory, zeroized on drop, and
  reused until its `status.expirationTimestamp`; the next admitted use at or after
  it runs the plugin again. With no expirationTimestamp the token serves only the
  use that ran the plugin. Any valid RFC 3339 date-time is an expiry, one before
  1970 included (Go's zero time `0001-01-01T00:00:00Z` among them); one at or
  before now serves only the use that ran the plugin and is not
  `malformed_credential`. A numeric offset is `+HH:MM` or `-HH:MM` with `HH` 00
  to 23 and `MM` 00 to 59. The adapter's own buffers holding the token, the
  output it came in, the environment strings built for the exec and the
  kubeconfig bytes read for a run (read into one buffer allocated to the file
  bound, which never reallocates, so no earlier copy is released) are zeroized
  before their memory is released (the kubeconfig parser's internal copies of
  `exec.env` values are not). It is never written to disk, a configuration, a
  description, a log or an error.
- **Refusals.** `digest_mismatch`, `plugin_unavailable` and `kubeconfig_mismatch`
  are `invalid_configuration`; `timeout` is `timeout`; `output_too_large` and
  `malformed_credential` are `protocol`; a non-zero exit (`plugin_failed`) is
  `unavailable`.

The shared `exec_plugin` acquisition flow stays reserved
([acquisition](../../../../../contracts/auth/acquisition/v1alpha1/semantics.md)):
this binding is Kubernetes-native, and the host sees an ordinary `http_bearer`
protected-entry profile.
Source facts:
https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins
and https://kubernetes.io/docs/reference/config-api/client-authentication.v1/.
Source access: 2026-10-15.

## Descriptor visibility

The implemented adapter filters disabled hosts.discover from its public descriptor.
With a current descriptor revision, shared public lookup returns not_found before
the separate adapter-internal forbidden guard. An old revision fails stale_description
first. This is a native implementation fact, not a universal shared disabled-operation
policy.

## Kubeconfig contexts

`contexts.list` (`kubernetes-kubeconfig-contexts`) lists the contexts of the
kubeconfig a local connection was configured with. Its typed projection is the
[`contexts` ESS domain](../../../spec/ess/domains/contexts.yaml).

- **Which file.** The local native configuration names it as `kubeconfig`, an
  absolute path to an owner-only file. The composition reads and parses it once at
  load (a file that is not a readable kubeconfig refuses the configuration) and
  records the path's digest in the effective configuration, so the path is part
  of the configuration revision the host admitted. The operation's only input is
  `limit` (1–256): no input can name a file, and a connection reads the file it
  was configured with and no other. The service configuration cannot enable it.
- **Why an adapter read and not a setup listing.** Both would read the same file;
  only this one is invocable on a saved connection, and fixing the path in the
  admitted configuration is what keeps an operation from reaching any other
  kubeconfig. The composition, not the business library, reads the file, so the
  business adapter still holds no file authority.
- **When.** The file is read at each invocation, so a context added later or a
  changed `current-context` shows on the next read; there is no provider request.
- **What.** Each context in file order: `name`, `cluster` (the cluster entry's
  name), `namespace` (null when the context sets none) and `current` (true for the
  context `current-context` names; all false when it names none). Users, cluster
  servers, certificate authorities, client certificates and keys, tokens, exec
  plugins and auth providers are never read into the result.
- **Bounds and refusals.** At most 1 MiB and 256 contexts; a repeated context
  name, a name over 256 bytes or a namespace over 63 refuses the whole file
  rather than listing part of it. A smaller `limit` returns the first contexts
  with `complete: false`; there is no cursor. A file missing or no longer
  owner-only at invocation is `unavailable`.
