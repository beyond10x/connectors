# auth.evidence/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** auth. Sibling documents: [connection](../../connection/v1alpha1/semantics.md), [profile](../../profile/v1alpha1/semantics.md), [acquisition](../../acquisition/v1alpha1/semantics.md), [custody](../../custody/v1alpha1/semantics.md), [capability](../../capability/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.evidence/v1alpha1` |
| Checks | `custody_reachable`, `credential_present`, `credential_valid`, `identity_check`, `scope_check`, `permission_check`, `verify_operation` |
| Nature | value-free answers to "is this connection ready and is this invocation authorized on the provider side", collected without side effects and without widening credentials |

Discovery, readiness, authentication, authorization, and execution are separate facts (`docs/design.md:161`). Readiness has layers: process alive, infrastructure ready, provider access valid, selected resource callable, active session healthy (`docs/design.md:965`). Readiness checks must have no undeclared external effect and must not resolve credentials merely to list static metadata (`docs/design.md:965`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Every `CredentialSource` implements a value-free readiness check proving only its injected Secret Store dependency; it never resolves a credential or probes the provider before an admitted operation | `../connectors/crates/voice-runtime/README.md`, paragraph 2 | preserve as `custody_reachable` |
| Kubernetes activation: SelfSubjectReview; per list: SelfSubjectAccessReview for exact verb, group, version, resource, namespace; denied namespaces skipped; `get` on `services/proxy` required at proxy invocation | `../connectors/docs/design/10-local-kubernetes-context-and-resource-discovery.md:50-116` | preserve as `identity_check` and `permission_check` profiles |
| Provider `verify = "<operation>"`: a designated read used to prove a connection | `../connectors/providers/grafana.toml` (`grafana-datasources-list`), `loki.toml`, `prometheus.toml`, `alertmanager.toml` | preserve as `verify_operation`, explicitly admitted and effect-free |
| Capability evidence belongs to the Connection, not the catalog; one predicate at discovery and dispatch | `../connectors/docs/design/09-curation-and-credential-capability-admission.md:157-213` | preserve |
| Current: `/healthz` proves liveness only; a descriptor does not prove provider readiness; an invocation checks dependencies | `contracts/service/v1alpha1/semantics.md`, Wire boundary | preserve; this contract adds the connection layer between them |
| Safe distinctions: reauthorization required, insufficient scope, invalid or revoked, custody outage, uncertain refresh | `docs/design.md:600` | preserve as the vocabulary of results |

## 3. Types

```json
{
  "connection": "conn_…",
  "collected_unix_ms": 0,
  "checks": [
    { "check": "custody_reachable", "result": "ok" },
    { "check": "credential_present", "result": "ok", "version": "ver_…" },
    { "check": "credential_valid", "result": "ok", "expires_unix_ms": 0, "source": "cached_token_metadata" },
    { "check": "identity_check", "result": "ok", "external_identity": "opaque" },
    { "check": "scope_check", "result": "insufficient", "missing": ["write:jira-work"] },
    { "check": "permission_check", "result": "denied", "subject": "list pods in namespace x" },
    { "check": "verify_operation", "result": "not_run" }
  ],
  "state": "insufficient_scope"
}
```

Results: `ok`, `missing`, `invalid`, `insufficient`, `denied`, `unavailable`, `uncertain`, `not_run`. `state` is the derived connection state (`auth.connection` status vocabulary).

Operation-time predicate (host-internal):

```text
admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(reason)
```

## 4. Rules

- Value freedom: no check returns or logs credential bytes, secret references, or provider URLs of private deployments.
- Effect classes per check:

| Check | External effect | Credential resolved |
|---|---|---|
| `custody_reachable` | none | no (existence and reachability only) |
| `credential_present` | none | no (metadata) |
| `credential_valid` | none by default: uses stored expiry and last-known validity; a provider introspection call is a separately admitted profile | no unless introspection is enabled |
| `identity_check` | one provider identity read (Atlassian `me`, Kubernetes SelfSubjectReview) at acquisition and on explicit repair only | yes, at the execution boundary |
| `scope_check` | none: compares granted scopes recorded at acquisition with the operation's `requires_auth` | no |
| `permission_check` | one provider authorization query per invocation where the provider offers one (Kubernetes SelfSubjectAccessReview); otherwise `not_run` | yes |
| `verify_operation` | the designated read, only when explicitly invoked by an operator or acquisition completion | yes |

- Same predicate at description and dispatch: the state shown by `connections.describe` and the admission decision at invocation derive from the same checks; the invocation re-evaluates, the description may be cached with `collected_unix_ms`.
- Provider authorization and SaaS authorization are both required and separately reported: a provider `permission_check: ok` does not admit a caller the host's policy refuses, and vice versa (`docs/design.md:381`).
- Readiness is not authority: an `ok` evidence set does not by itself permit an operation; it is an input to admission.
- Stale evidence: an evidence snapshot older than the profile's freshness bound is `uncertain` and triggers re-collection before a mutation; reads may proceed on cached evidence within the bound.
- Kubernetes profile: `permission_check` is per verb, group, version, resource, namespace, before each list; a denied namespace is skipped and reported, not treated as empty (old rule preserved).

## 5. Limits

| Concern | Rule |
|---|---|
| Freshness bound | first-profile default 300 s for cached evidence; mutations require ≤ 60 s or re-collection |
| Provider calls per invocation | at most one `permission_check` call; no identity call |
| Deadline | evidence collection shares the provider deadline; timeout → `unavailable`, never `ok` |

## 6. Conformance scenarios

- Describe a connection with a fake custody outage → `custody_reachable: unavailable`, state `custody_unavailable`; fake provider sees zero requests.
- Descriptor fetch and `connections.list` never resolve a credential (fake custody records reads → zero).
- Granted scopes lack `write:jira-work` → mutation refused with `insufficient_scope`, no dispatch; a read requiring `read:jira-work` proceeds.
- Kubernetes fake denies SSAR for namespace `b` → list of `[a, b]` returns `a` items and reports `b` as denied.
- Expired stored token metadata → `credential_valid: invalid` → state `reauthorization_required`; the `verify_operation` is not run automatically.
- Provider `permission_check` ok but host policy refuses → `Forbidden`, no dispatch.

## 7. Compatibility

- Additive: the connection status in `auth.connection` is derived from this contract; no wire change beyond the optional `evidence` block in `connections.describe`.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Evidence collector in the host with per-check effect class enforcement | new host module |
| Provider hooks for `identity_check`, `permission_check`, `verify_operation` (optional trait methods) | `crates/connectors-sdk` (`Adapter` trait, `lib.rs:18-31`) |
| Cached evidence with `collected_unix_ms` in connection metadata | host metadata store (custody document) |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `EvidenceSnapshot` (value on `Connection`) | checks, results, collected time |
| `AuthProfile.evidence` | which checks a profile supports (profile document) |
| SaaS policy inputs | UNMAPPED |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether `credential_valid` may call provider introspection by default | no; metadata only |
| Freshness bounds | 300 s reads, 60 s mutations |
| Whether `verify_operation` runs automatically after acquisition | yes, once, as the completion step; result recorded, failure does not delete the credential |
