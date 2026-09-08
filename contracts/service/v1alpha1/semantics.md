# Configured adapter services v1alpha1

This first implementation is authorized by the goal "3 adapters implemented e2e",
with the operator selecting Kubernetes including discovery, GitLab, and SQL.
The full direction remains in docs/design.md. This document settles the first
executable slice and defines the evidence required before claiming completion.

## Placement and contracts

Each adapter is an independent Rust library and executable. The SDK defines
adapter, credential, secret-store and authenticated-HTTP ports plus schema and
cursor helpers. The host implements bounded HTTP serving, token admission,
credential stores and scoped HTTP transport. Provider code
owns request and result semantics. The client and federation host have no provider
dependencies. No catalog, Atlas integration, deployment, or dynamic plugin ABI is
required. Ordinary source builds use Cargo alone.

The first provided contracts are operations/v1alpha1, datasource.records/v1alpha1,
datasource.relational/v1alpha1, endpoint_discovery/v1alpha1 and
host_discovery/v1alpha1. Only implemented contracts/operations are advertised.
Generic events, duplex/media sessions, OAuth acquisition, and process execution
remain future contract families; these three data/discovery adapters do not claim
to implement them.

The implemented public lookup searches the descriptor's operation vector after authentication and revision validation (`crates/connectors-core/src/lib.rs`, `connectors-host/src/server.rs`). Kubernetes filters disabled hosts.discover from that vector; with a current revision its public invocation therefore returns not_found before reaching the adapter's separate internal forbidden guard. An old revision returns stale_description first. These facts do not establish a universal rule that every future disabled implementation is indistinguishable from absence. The proposed [extended matrix](../v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12) specifies its admitted private lookup; [legacy projection](../compatibility.md#3-legacy-projection-and-federation-intersection) deliberately keeps descriptor-only lookup. No current reader or runtime behavior is changed by that proposal.

## Wire boundary

- GET /v1/describe returns the exact instance identity, specification digest,
  enabled operations, typed input/output JSON Schemas and configuration schema.
- POST /v1/invoke accepts version, request_id, operation, descriptor revision and
  input; returns the same request_id and either a typed result or safe error.
- GET /healthz proves liveness only. Descriptor discovery is authenticated; a
  descriptor does not prove provider readiness. An invocation checks dependencies.
- Receiver configuration binds a service token and provider connection. Runtime
  requests cannot select arbitrary credentials, remote hosts, database paths,
  issuer, tenant or realm. Requests with unknown envelope fields are refused.
- Limits: 64 KiB request body, 4 MiB upstream/result body, 32 concurrent adapter operations,
  20 s adapter-execution deadline at the service, 15 s provider request budget
  (5 s connect). SQL spends at most 10 s executing after connection and reserves
  up to 2 s for cancellation/cleanup within the 15 s total; a slow connection
  reduces the available execution time. SQL starts with 10 s statement and 2 s
  lock timeout settings. Other limits are 1–100 requested records, 1,000 SQL rows maximum,
  8 KiB query text and 300 s cursor TTL. These bounds are fixed in this profile;
  configuration does not expose overrides. Requests choose smaller page/row limits.
- Every public failure is a structured error with a stable code and safe message.
  Provider response bodies and credentials are never copied into error messages.
  No automatic retry. Provider 401/403, 404, 410, 429, 5xx and malformed/oversized
  responses remain distinguishable. Unsupported versions and stale revisions fail
  before provider dispatch.
- These concurrency/deadline bounds begin at adapter execution after admission,
  body reading and validation. They are not connection-count, header-read or
  body-read bounds. Public ingress needs separately configured transport limits.
- Provider deadlines bound this service's wait; a dropped connection alone does
  not prove the provider stopped work. SQL has an independent execution deadline
  and sends a backend-keyed cancellation request on failure, deadline or a dropped
  invocation, using the established endpoint and captured TLS trust. Its supervisor
  keeps driving cleanup for at most 2 s even if the caller disappears, then closes
  the connection. Sending cancellation has no acknowledgement; an unreachable
  database, process crash or runtime shutdown can prevent backend termination.
  These read profiles do not promise zero query cost or certain remote termination.

## Auth and configuration

Service admission and external provider credentials are separate. Credentials are
referenced by configured environment/file-backed secret bindings, never serialized
in descriptions or supplied in invocation input. An injected SecretStore and
connection-bound authenticated HTTP capability allow fake/in-memory stores and
durable files without changing provider operation code. Files must be bounded,
regular, owner-readable only and not symlinks. Tokens can rotate by replacing the
bound file; service identity/config revision remain stable until config changes.

HTTP follows no redirects, uses verified TLS by default, optional configured CA,
no implicit system proxy, and admits plaintext only for explicitly enabled local
test endpoints. Auth is placed only on the configured origin/base path. Adapter
configuration is typed, rejects unknown fields, and is activated at startup; edits
require restart and get a new descriptor revision. No live-update API is advertised.

## Selected operations

GitLab: project metadata, paginated project issues, repository file content/metadata.
Project allowlists apply before dispatch. Pagination preserves provider continuation
and has cursors bound to exact operation/input/instance/config identity and expiry.
GitLab source facts: https://docs.gitlab.com/api/projects/,
https://docs.gitlab.com/api/issues/, https://docs.gitlab.com/api/repository_files/,
https://docs.gitlab.com/api/rest/ and
https://docs.gitlab.com/api/rest/authentication/. Source access: 2026-09-08.

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

SQL: PostgreSQL schema discovery and parameterized, bounded single-statement reads
through a configured role/database. Database-enforced read-only transactions and
an adapter-owned execution deadline are mandatory. Each transaction installs
statement/lock timeouts as additional server-side guards. Caller-accessible
functions can change those session settings, including between portal executions;
they cannot extend the adapter's deadline. Schema scope is enforced by database grants,
not a string-prefix SQL filter. Preserve column type metadata, NULL, arrays/JSON
and numeric values without silently converting unsupported values into strings.
The first `postgresql-native-text` profile encodes non-NULL cells as PostgreSQL's
lossless text representation alongside each column's native type name; NULL is
JSON null. Numeric precision, arrays, timestamps and JSON are preserved as native
text rather than coerced to JavaScript numbers. Parameters are text or null with
server-side type resolution. This representation is explicit in the descriptor.
Report truncation explicitly; no invented resumable cursor for a fresh SQL query.
Each request uses a fresh connection and read-only transaction, initialized with
`search_path = public, pg_catalog`; qualify names in other schemas. Session settings
are not a substitute for the configured role's schema and function grants.
No connection pool is claimed in this local profile. A configured CA bundle
replaces public trust roots for SQL and HTTP providers; empty bundles are refused.
Do not advertise writes, transactions across requests, or dialect portability.
Source facts: https://www.postgresql.org/docs/current/sql-set-transaction.html and
https://www.postgresql.org/docs/current/runtime-config-client.html.
Source access: 2026-09-08.

## Federation

The generic host exposes each configured downstream operation under a stable source
prefix, preserves source provenance and response/error semantics, and forwards with
an independently configured downstream credential. It validates the client's
description revision, obtains the matching downstream revision, and never falls
back to another source or retries silently. Downstream failure must not appear as
an empty successful result. Configuration establishes trusted routes; discovery
cannot create a new route. Initial topology is one hop; cycles are refused.

On a leaf `stale_description`, the gateway fetches that configured leaf's descriptor
and atomically replaces its projected snapshot before returning the stale error.
It never replays the invocation. The caller fetches the gateway description,
inspects the changed surface, and deliberately resubmits. Refresh failure returns
the actual failure and preserves the previous complete snapshot. Credential
references are resolved on each invocation and refresh; routes retain no token.
The admitted revision also guards route selection if another call refreshes it.

## Specification and realization

Connectors owns adapter/v1's schema and validator. Adapter specifications declare
provided contract/profile, operation schemas, configuration schema, source URLs,
and implementation binding. A Connectors compiler validates and derives a canonical
descriptor and digest. Immutable declaration types are modeled and validated by
ESS in ess/. Automatic vendor OpenAPI import/lowering into executable ESS realizations
is a future authoring feature; this first slice implements explicit provider binding
obligations. Runtime consumes committed generated artifacts; source compilation
must not fetch vendor inputs or invoke ESS from build.rs. Generated declarations
must match registered handlers. A typed handwritten provider binding is an explicit
implementation obligation, not a generated claim that upstream effects work.

## End-to-end acceptance

All three binaries must start from documented config, be discovered by the generic
client, execute their supported operations directly and through the host, reject
invalid/unauthorized/stale requests before dispatch, and shut down cleanly.
Black-box upstream fixtures prove wire parsing, auth, pagination, limits and error
cases. Separate live evidence must prove Kubernetes discovery against an actual
API server, SQL against PostgreSQL, and GitLab against an actual GitLab endpoint.
Fixture success alone is not live-provider evidence.

The engineering chain discovers a PostgreSQL endpoint through Kubernetes, requires
an explicit configuration/credential binding, then queries SQL through the host.
The client/host build without provider libraries; each adapter builds without its
siblings. Record exact commands and evidence in docs/verification.md. The goal is
complete only after these scoped adapter capabilities and integration paths work.
