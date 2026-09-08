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
    { "check": "credential_present", "result": "ok" },
    { "check": "credential_valid", "result": "ok", "expires_unix_ms": 0, "source": "cached_token_metadata" },
    { "check": "identity_check", "result": "ok", "external_identity": "opaque" },
    { "check": "scope_check", "result": "insufficient", "missing": ["write:jira-work"] },
    { "check": "permission_check", "result": "denied", "subject": "list pods in namespace x" },
    { "check": "verify_operation", "result": "not_run" }
  ],
  "state": "ready"
}
```

This JSON is an illustrative public projection: the missing write grant and denied namespace belong to particular operations, while the connection baseline is assumed valid. They do not reduce the whole connection to insufficient_scope. A concrete payload binds each operation check to its declared exact subject and reports only admitted safe details. This JSON is the public projection. It omits credential generation IDs, snapshot handles and secret-store versions; none is a public evidence identifier. Host-private evidence additionally binds the captured generation, instance, connection, auth profile and provider authority, and records each check's source, collection time, validity deadline and exact subject where applicable. `collected_unix_ms` never replaces those per-check deadlines.

Results: `ok`, `missing`, `invalid`, `insufficient`, `denied`, `unavailable`, `uncertain`, `not_run`. `state` is the global connection viability reduction in [connection §4.1](../../connection/v1alpha1/semantics.md#41-connection-viability-and-operation-eligibility), using only applicable baseline checks and administrative facts. Evidence alone does not override terminal revocation or disablement.

Operation-time predicate (host-internal):

```text
admit(connection, operation, input, captured_generation)
  -> Admitted(generation_bound_pin, evidence_snapshot) | Refused(reason)
dispatch(admitted_pin, request) -> Dispatched | Refused(reason)
```

## 4. Rules

- Value freedom: no check returns or logs credential bytes, secret references, or provider URLs of private deployments.
- Effect classes per check:

| Check | External effect | Credential resolved |
|---|---|---|
| `custody_reachable` | none | no (existence and reachability only) |
| `credential_present` | none | no (metadata) |
| `credential_valid` | none by default: uses stored expiry and last-known validity; a provider introspection call is a separately admitted profile | no unless introspection is enabled |
| `identity_check` | a declared provider identity read during admitted acquisition, activation or explicit revalidation/repair; same-account refresh may use the narrowly defined lineage rule below | yes, for the exact captured generation |
| `scope_check` | none: compares granted scopes recorded at acquisition with the operation's `requires_auth` | no |
| `permission_check` | one provider authorization query per invocation where the provider offers one (Kubernetes SelfSubjectAccessReview); otherwise `not_run` | yes |
| `verify_operation` | the designated read, only when explicitly invoked by an operator or acquisition completion | yes |

- Description and dispatch use the same **two reductions** for the same question: connection-wide viability, then exact-operation eligibility. Description reports global viability under its observation/validity bounds; invocation freshly evaluates its own scope, permissions, verification and authority, then the F05 dispatch fence. An unrelated operation's failed or stale check never poisons global viability. `collected_unix_ms` is not a per-check expiry or authorization lease.
- Provider authorization and SaaS authorization are both required and separately reported: a provider `permission_check: ok` does not admit a caller the host's policy refuses, and vice versa (`docs/design.md:381`).
- Readiness is not authority: an ok baseline does not itself permit an operation. Stale/unknown/not_run baseline validation yields pending (positive invalidity or consumed refresh yields reauthorization_required); unavailable custody and parent facts use their own states. Stale operation-only scope/permission/verification refuses only affected requests under connection §4.1. Not-required checks may remain not_run and supply no authority.
- Stale evidence: each required check must meet its own freshness bound at admission and dispatch. Age-valid evidence for another generation or binding is unusable. Re-collection must itself be permitted by the check's declared effect and admission rules; otherwise refuse under the applicable reduction: stale baseline validation gives pending/connection_not_ready, stale required permission gives unavailable for that operation, and stale operation verification gives connection_not_ready without changing global state. Staleness alone is not positive credential invalidity and never authorizes an implicit probe.
- Kubernetes profile: `permission_check` is per verb, group, version, resource, namespace, before each list; a denied namespace is skipped and reported, not treated as empty (old rule preserved).

### 4.1 Credential generation and identity

A **credential generation** is a host-private, immutable capture of the material that a capability will actually use. Its host-issued opaque ID is distinct from a connection ref, configuration/descriptor revision, filename, exec command, or secret-store version. New material produces a new generation, even for the same account. An unchanged path or revision is never proof of unchanged material. A fresh process must either recover the same verifiable immutable snapshot or capture and validate a new generation; it cannot infer continuity from the path.

Evidence binds to that generation and to `(instance, connection, auth profile, provider authority)`. Expected identity at capture is intent, not validation. Successful identity evidence establishes the observed `(kind, stable subject)` within that provider authority and requires it to equal the intended connection identity. A display name, token prefix, certificate filename or config revision does not establish that equality. If the profile cannot establish identity by its declared validation mechanism, the candidate remains unvalidated.

The rule covers bearer/basic/signing material, coherent client certificate and key snapshots, and the material returned by one exec-plugin run. Validating an exec result and running the helper again at dispatch would create an unvalidated substitution. Private generation IDs and any internal equality information must stay out of public metadata, errors and logs.

Configured replacement and repair must preserve the established identity. An inactive independent repair candidate with a different subject, kind, provider authority or binding is rejected as identity_mismatch (or a safe binding refusal) without replacing or poisoning a still-valid active generation. Its prior state remains subject to independently current expiry, revocation and consumed refresh. Detected substitution of the currently used material blocks dispatch pending validation; a proved identity/binding mismatch there gives connection_not_ready and reauthorization_required, with private identity_changed/binding_changed reason. Neither case permits publishing the mismatched candidate. An explicitly admitted reassignment is a distinct management action that must invalidate old admissions, evidence, cursors and sessions before admitting the new binding. This profile provides no implicit reassignment through file overwrite or refresh; where no reassignment flow is declared, refuse it.

### 4.2 Admission, validation and dispatch

1. Host policy first admits access to the selected connection and the local capture/credential use required by the declared operation. `connections.list`, descriptor fetch and `connections.describe` do not capture or validate material.
2. Capture or acquire a handle to immutable material. Match evidence to that exact generation and binding. A detected replacement blocks pending and new dispatches until validation succeeds. Do not copy an old snapshot merely because its time bound has not expired.
3. Initial activation and replacement validation use the auth profile's explicitly declared and separately admitted validation step, with its own effect and deadline budget. An ordinary business invocation permits no implicit identity read. If the needed step is not available or not admitted, refuse the business invocation and require activation/revalidation; an `ok` cached identity for old material is no fallback. An exec helper and its identity validation are subject to the same rule.
4. Required checks, host authorization and provider permission checks must all apply to this request and generation. For example, an SSAR result binds the exact verb, group, version, resource, namespace and any name/subresource being authorized. Capture a transient `DispatchAdmission` and pin the validated material; the pin conveys no authority beyond this admitted use.
5. At the dispatch boundary, atomically order the final check with connection revocation, replacement detection and new-generation publication. The selected generation must still be current, the binding and host authorization valid, and every required check unexpired. Known credential revocation, expiry, loss of the snapshot, or an authorized rotating refresh that is still unresolved refuses even a pinned admission. The last case reports `credential_valid: uncertain` and the existing `connection_not_ready` refusal; it cannot assume the provider left old material usable. The transport must use the pinned bytes/key handle, never re-read the mutable path or re-run an exec helper.

Refresh exclusion also covers aliases or duplicate captures of the same rotating credential set; a fresh generation UUID is not independent authority to refresh or dispatch possibly consumed material.

For this first profile, successful refresh/replacement publication cuts off **all pending old-generation admissions**. A new request needs a new admission using the newly published generation and evidence; no old pin survives the publication boundary. Before that boundary a source file changing cannot silently change a pin's bytes: it may use its original validated material only while the host still considers that generation current and usable. Once replacement is detected, dispatch blocks as above. A request that already crossed the dispatch boundary may already have an external effect; publication does not undo that effect or authorize a retry.

A host unable to pin material and enforce these checks must refuse. Revalidation is a new admitted decision over a newly captured generation, never mutation of an old admission. Authority, snapshot liveness and the atomic dispatch/publication ordering are runtime obligations; the ESS model records the decision shape without executing them.

### 4.3 Which evidence can cross a generation change

Every change creates a new evidence value. A check may be transferred only as this table allows; copied checks retain their original collection time and validity deadline, and the new value records their source generation. Transfer cannot turn `not_run`, denied, uncertain or expired evidence into success.

| Check | Same-identity refresh with validated lineage | Configured replacement, repair or reassignment |
|---|---|---|
| `custody_reachable` | may retain within its original bound if the custody binding is unchanged; says nothing about the credential | same infrastructure-only rule |
| `credential_present` | establish for the new immutable snapshot | establish for the new snapshot |
| `credential_valid` | recompute from the new auth result and expiry; never inherit the old token's validity | establish for the new material by the declared validation mechanism |
| `identity_check` | may retain only if the provider auth profile explicitly proves the refresh lineage preserves the same stable identity and authority; preserve age/deadline and source generation; otherwise perform admitted identity validation or refuse | validate the observed identity of the new capture; old identity evidence is not reusable |
| `scope_check` | recompute from grants established for the new generation, including narrowing; omitted grants require a profile-defined proof of preservation or refusal | recompute against the newly established grants |
| `permission_check` | invalidate; collect anew for the exact operation/target and new generation where supported | invalidate; same rule |
| `verify_operation` | invalidate; keep `not_run` until explicitly admitted verification is performed, and refuse operations whose profile requires it | same rule |

A refresh response containing the expected identity label alone is not a lineage proof. The provider auth implementation must establish that the validated refresh exchange is for the previously bound account under its declared profile. Generic refresh coordination cannot assert that provider fact. Neither retained identity nor wider new grants enlarge the caller's host permissions. Refresh publication and its revocation fence are specified by [acquisition](../../acquisition/v1alpha1/semantics.md).

## 5. Limits

| Concern | Rule |
|---|---|
| Freshness bound | first-profile default 300 s for cached evidence; mutations require ≤ 60 s or re-collection |
| Provider calls per business invocation | at most one `permission_check` call under this first profile; no implicit identity call; explicit activation/revalidation has its own admitted budget (namespace fan-out budgeting is a separate story) |
| Deadline | evidence collection shares the provider deadline; timeout → `unavailable`, never `ok` |

## 6. Conformance scenarios

- Describe a connection with a fake custody outage → `custody_reachable: unavailable`, state `custody_unavailable`; fake provider sees zero requests.
- Descriptor fetch and `connections.list` never resolve a credential (fake custody records reads → zero).
- Granted scopes lack `write:jira-work` → mutation refused with `insufficient_scope`, no dispatch; a read requiring `read:jira-work` proceeds.
- Kubernetes fake denies SSAR for namespace `b` → list of `[a, b]` returns `a` items and reports `b` as denied.
- Expired stored token metadata → `credential_valid: invalid` → state `reauthorization_required`; the `verify_operation` is not run automatically.
- Provider `permission_check` ok but receiver grant policy refuses → `not_granted`, no dispatch; separate adapter/connection/resource restrictions use `forbidden`.

- Replace account A material with account B material at the same configured path and revision → no business dispatch; separately admitted validation discovers the identity mismatch and refuses ordinary replacement.
- Publish a same-account generation between admission and dispatch → the old admission refuses; a new admission validates/pins the new generation. A raw source change never substitutes new bytes into an existing pin.
- Refresh narrows grants → new scope evidence refuses a write requiring the removed grant; old `scope_check: ok` is not copied. Permission and verification results are invalidated even when identity lineage is retained.
- Revoke or expire a credential after admission → the pin refuses dispatch; `Admitted` is not enduring authority.
- Public describe is searched for secret bytes, private generation IDs, snapshot handles and secret-version references → no match.

[Verification evidence and ESS limits](verification.md) distinguish compiled authored expectations from required runtime tests.

## 7. Compatibility
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. The evidence block belongs to the selected managed connection payload schema, whose reader must support it. Managed metadata and new error codes require the extended codec; optionality does not make a field acceptable to an existing strict payload reader.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Evidence collector in the host with per-check effect class enforcement | new host module |
| Provider hooks for `identity_check`, `permission_check`, `verify_operation` (optional trait methods) | `crates/connectors-sdk` (`Adapter` trait, `lib.rs:18-31`) |
| Generation-bound evidence values, per-check provenance/deadlines and replacement invalidation | host connection metadata, separate from opaque custody |
| Immutable material pin plus dispatch/publication/revocation ordering | host admission and capability boundary; shared coordinator controls publication |
| Explicit validation admission and identity/grant interpretation | host policy and provider auth profile; ordinary business requests cannot invent probes |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `EvidenceSnapshot` (value on `Connection`) | modeled in [credential_evidence.yaml](../../../../ess/domains/credential_evidence.yaml): generation/binding, observed identity and per-check source/deadlines; not a separately persisted entity |
| `DispatchAdmission` | transient per-invocation decision, `Inspected → Admitted → Dispatched` or `Inspected/Admitted → Refused`; references exactly one immutable generation; no durable-store ownership implied |
| `CredentialGeneration` | [shared model](../../../../ess/domains/credentials.yaml); capture alone does not validate the expected identity |
| `AuthProfile.evidence` | which checks a profile supports (profile document) |
| SaaS policy inputs | UNMAPPED |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether `credential_valid` may call provider introspection by default | no; metadata only |
| Freshness bounds | 300 s reads, 60 s mutations |
| Completion-time verification | only the explicitly selected/admitted validation plan runs it. Mandatory baseline failure prevents publication/completed; optional failure preserves baseline publication but blocks any operation requiring that check. Supported is not mandatory; see [profile §4.1](../../profile/v1alpha1/semantics.md#41-baseline-and-operation-requirements). |
