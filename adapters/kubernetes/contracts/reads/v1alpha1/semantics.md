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
