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

The implemented public lookup searches the descriptor's operation vector after authentication and revision validation (`crates/connectors-core/src/lib.rs`, `connectors-host/src/server.rs`). The Kubernetes binding records the existing disabled-host-discovery behavior. These facts do not establish a universal rule that every future disabled implementation is indistinguishable from absence. The proposed [extended matrix](../v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12) specifies its admitted private lookup; [legacy projection](../compatibility.md#3-legacy-projection-and-federation-intersection) deliberately keeps descriptor-only lookup. No current reader or runtime behavior is changed by that proposal.

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
  (5 s connect). Native execution/query/row ceilings are owned by each adapter binding.
  Shared record pages request 1–100 records and cursors expire after 300 s. These bounds are fixed in this profile;
  configuration does not expose overrides. Requests choose smaller page/row limits.
- Every public failure is a structured error with a stable code and safe message.
  Provider response bodies and credentials are never copied into error messages.
  No automatic retry. Provider 401/403, 404, 410, 429, 5xx and malformed/oversized
  responses remain distinguishable. Unsupported versions and stale revisions fail
  before provider dispatch.
  This existing rule is unchanged by the proposed
  [read-refresh-once/v1alpha1](../../auth/capability/v1alpha1/read-refresh-once.md)
  binding: its distinctly advertised native profile requires v1alpha2 and is
  excluded from legacy projection. No current adapter selects it; a capability
  rename or refreshed credential never opts this profile into retry behavior.
- These concurrency/deadline bounds begin at adapter execution after admission,
  body reading and validation. They are not connection-count, header-read or
  body-read bounds. Public ingress needs separately configured transport limits.
- Provider deadlines bound this service's wait; a dropped connection alone does
  not prove the provider stopped work. Native cancellation and cleanup behavior are owned by the adapter binding;
  sending cancellation need not acknowledge remote termination.
  These read profiles do not promise zero query cost or certain remote termination.

## Auth and configuration

Service admission and external provider credentials are separate. Credentials are
referenced by configured environment/file-backed secret bindings, never serialized
in descriptions or supplied in invocation input. An injected SecretStore and
connection-bound authenticated HTTP capability allow fake/in-memory stores and
durable files without changing provider operation code. Files must be bounded,
regular, owner-readable only and not symlinks. Tokens can rotate by replacing the
bound file; service identity/config revision remain stable until config changes.

HTTP follows no redirects, uses verified TLS by default and optional configured CA.
A configured HTTP CA bundle replaces public trust roots; empty bundles are refused.
HTTP uses
no implicit system proxy, and admits plaintext only for explicitly enabled local
test endpoints. Auth is placed only on the configured origin/base path. Adapter
configuration is typed, rejects unknown fields, and is activated at startup; edits
require restart and get a new descriptor revision. No live-update API is advertised.

## Selected operations

The shared service defines transport, lookup and envelopes. Native implemented
profiles belong to [GitLab](../../../adapters/gitlab/contracts/reads/v1alpha1/semantics.md),
[Kubernetes](../../../adapters/kubernetes/contracts/reads/v1alpha1/semantics.md)
and [SQL](../../../adapters/sql/contracts/reads/v1alpha1/semantics.md).
These index links document current local implementations, not a closed provider
catalog or dependency of this wire contract on their code.

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
