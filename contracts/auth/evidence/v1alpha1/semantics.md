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
| `permission_check` | at most one provider authorization query per uncached exact target, within the admitted shared budget in §4.4 (Kubernetes SelfSubjectAccessReview); otherwise `not_run` | yes for the independently admitted checking connection |
| `verify_operation` | the designated read, only when explicitly invoked by an operator or acquisition completion | yes |

- Description and dispatch use the same **two reductions** for the same question: connection-wide viability, then exact-operation eligibility. Description reports global viability under its observation/validity bounds; invocation freshly evaluates its own scope, permissions, verification and authority, then the F05 dispatch fence. An unrelated operation's failed or stale check never poisons global viability. `collected_unix_ms` is not a per-check expiry or authorization lease.
- Provider authorization and SaaS authorization are both required and separately reported: a provider `permission_check: ok` does not admit a caller the host's policy refuses, and vice versa (`docs/design.md:381`).
- Readiness is not authority: an ok baseline does not itself permit an operation. Stale/unknown/not_run baseline validation yields pending (positive invalidity or consumed refresh yields reauthorization_required); unavailable custody and parent facts use their own states. Stale operation-only scope/permission/verification refuses only affected requests under connection §4.1. Not-required checks may remain not_run and supply no authority.
- Stale evidence: each required check must meet its own freshness bound at admission and dispatch. Age-valid evidence for another generation or binding is unusable. Re-collection must itself be permitted by the check's declared effect and admission rules; otherwise refuse under the applicable reduction: stale baseline validation gives pending/connection_not_ready, stale required permission gives unavailable for that operation, and stale operation verification gives connection_not_ready without changing global state. Staleness alone is not positive credential invalidity and never authorizes an implicit probe.
- Kubernetes profile: `permission_check` is per exact target including verb, group, version, resource, namespace, name and subresource before each list/forward. A denied namespace is skipped and reported, not treated as empty, under §4.4.

### 4.1 Credential generation and identity

The material-generation rules in §§4.1–4.3 apply to credential-bearing capabilities. Explicit anonymous and parent-route child profiles use [profile §4.2](../../profile/v1alpha1/semantics.md#42-explicit-access-bindings-without-child-credentials): no child credential/identity/custody checks, no invented generation or account. They require a current admitted configuration/route binding and applicable verification instead. The parent retains its own F05 credential-generation/evidence fence; a child's admitted route is not a substitute for it. This is an explicit applicability branch, never a response to missing required material. Host-internal admit/dispatch above selects the appropriate binding; CredentialGeneration and EvidenceSnapshot continue to mean material-bearing evidence only.

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

### 4.4 Exact authorization targets and fan-out budget (F08)

After host admission and configuration allowlist checks, build the complete bounded authorization target set from the operation and selected input before provider work. For Kubernetes a resource target is the exact `(verb, api_group, api_version, resource, namespace, name, subresource)` tuple. Empty group means core; empty subresource means none. Empty name is the selected collection-wide name attribute for list, not permission to substitute get/delete on arbitrary names. Namespace meaning depends on the declared resource and operation: a namespaced list carries its actual namespace; an explicitly selected all-namespaces list carries empty namespace; a cluster-scoped resource also carries empty namespace but is a different resource tuple. These distinctions follow Kubernetes [ResourceAttributes](https://kubernetes.io/docs/reference/kubernetes-api/definitions/resource-attributes-v1-authorization/) and cannot be inferred from a public null field or an empty result. A proxy forward has `get`, core/v1, `services`, the fixed namespace and Service name, and `proxy`. Exact configured kinds multiply finite per-namespace targets. Deduplicate identical tuples within the same checking binding and order them lexicographically by the seven fields in the order above, using their reviewed canonical UTF-8 representation. A required multi-target read with an empty normalized set is invalid_input; omitted selectors may expand only under an explicitly bounded configured selection. Neither grouping namespaces nor a list result grants permission to get another resource/name/subresource.

The first Service discovery profile preserves the explicitly configured all-namespaces mode: an admitted empty configured namespace list selects exactly `(list,"","v1","services","","","")`, one SSAR target and one cluster-wide collection partition. It is mutually exclusive with the finite per-namespace mode. A missing/invalid configuration, denied namespace, empty provider result or caller-supplied empty selector cannot activate this mode or fall back to it. An all-namespaces allow cannot be substituted for a per-namespace target in this exact-evidence profile, nor can several namespace allows synthesize the all-namespaces allow. Switching modes changes the selection/scope and requires fresh admission/evidence. All provider pages share the same scan object/call/byte/deadline ceilings; one target does not grant an unbounded cluster scan. The scope's private checking/profile/configuration coordinates preserve the selected resource semantics. This fixes the earlier overbroad claim that every empty namespace meant a cluster-scoped resource; it does not add wildcard matching or impersonation.

Target equality for evidence reuse additionally includes the checking instance, connection, profile, provider authority, immutable credential generation, current binding/configuration revision and host admission scope. A mediated check uses the parent checking binding and fixed child route/observation revision. This first Kubernetes profile forbids impersonation and caller-supplied Impersonate-* headers; any future supported impersonation context must become an explicit reviewed evidence/admission coordinate. A safe public target description is not that private cache key. Reuse requires unchanged bindings and unexpired evidence at both admission and dispatch: at most 300 s for reads, 60 s for mutation authorization, or the smaller provider/profile deadline. Known revocation, replacement, policy withdrawal or evidence invalidation wins over age. Permission evidence never crosses a generation change. Denied evidence may be reused only under the same bounds and never becomes success; caching cannot replace current host authorization.

The first profile admits **at most 64 distinct targets and 64 provider authorization calls per business invocation**, with at most one call per uncached target, at most four concurrent authorization queries, and no automatic retries, redirects or nested checks. A deployment/operation may lower target/call/concurrency ceilings (including a zero-call cache-only policy), never raise them without a revised profile. Cached targets still count toward the target ceiling. Reserve the whole missing-evidence call budget before starting any check; a target-set overflow is invalid_input and an insufficient remaining call budget is unavailable, both before any provider request. Every attempted call consumes one slot even on timeout or ambiguous response. Concurrency counts all in-flight authorization queries across nested work; zero remaining deadline grants no send. Stop scheduling on failure/deadline, and cancel or await already in-flight checks without issuing more. Provider pagination and parent forwarding share the original invocation budget and deadline; a subrequest does not reset it. Evidence checks and business reads share the provider deadline. An ordinary call cannot invent identity/verify probes to consume spare slots.

All required checks finish before the first business resource request. Only a well-formed definite allow authorizes; missing/malformed/contradictory status or a provider evaluation error is unavailable, never denial or success. Any unavailable, timed-out, unknown, not_run or stale required result refuses the invocation as unavailable, with zero business reads; completed checks may remain cached only under the original bounds. This conservative first profile does not return a successful checked prefix after budget exhaustion. A definitive authorization denial is separate: for an explicitly declared multi-target read supporting authorization coverage, skip denied targets and read only allowed targets. If all are denied, return forbidden, never a successful empty page. Single-target operations and mutations refuse denied permission with forbidden; a Kubernetes route maps exact parent permission denial to route_refused. A required result expiring or becoming invalid before dispatch cannot be reused; if earlier allowed resource reads already ran, discard their partial payload and return the applicable refusal (those read requests are not undone). no unchecked request or implicit extra check is permitted.

The multi-target read payload must declare `authorization: {complete: boolean, targets: [{target: {verb, api_group, api_version, resource, namespace, name, subresource}, result: "allowed" | "denied"}]}` as a closed safe projection. Every requested distinct target appears exactly once under disclosure admission, in canonical target order. The array has at most 64 entries and each coordinate at most 256 UTF-8 bytes (smaller provider-specific limits still apply). Coverage and provider items together must fit the selected result byte limit; if truthful coverage cannot fit, refuse unavailable without silently dropping targets. Here authorization.complete is true only if all requested targets are allowed; any denial forces both authorization.complete and the overall result's complete to false, even when all allowed-target pages have been read. A permitted target returning zero records is distinct from a denied target. Pagination/cursors bind the original target set, denial/coverage observation and all normal configuration/authority constraints; continuation re-evaluates current admission and exact evidence. A permission/coverage change invalidates the continuation (stale_cursor), requiring a fresh read, and never silently changes the represented target set. No cursor is fabricated solely to retry a denied target. A payload reader without this coverage field cannot advertise this partial-read profile; the existing strict Page is unchanged. A discovery consumer must not call an incomplete view complete: denied/incomplete partitions provide no absence evidence. An independently complete, comparable partition may establish absence only within its exact scope even when another partition makes the overall view partial, under [resource discovery §4.2](../../../discovery/resources/v1alpha1/semantics.md#42-complete-and-incomplete-collection). Authorization coverage alone never proves provider exhaustion. Prior terminal withdrawn rows remain withdrawn; other unobserved history may become stale. Coverage discloses only admitted target coordinates, never credential generations or hidden route locators. It is authorization coverage, not proof that a discovery observation was withdrawn or that a provider collection was exhaustive.

Kubernetes SelfSubjectAccessReview is an explicitly admitted auth-validation POST for this check, despite the business read's GET-only capability. A private check-specific capability fixes its endpoint and bounded ResourceAttributes body; it cannot be used by provider business code to POST arbitrary resources. Count its evaluation as provider work under this budget, without misclassifying it as an approved business mutation. Exact request/response interpretation and a supporting capability implementation are advertisement prerequisites; the generic read capability grants no such POST authority.

## 5. Limits

| Concern | Rule |
|---|---|
| Freshness bound | first-profile default 300 s for cached evidence; mutations require ≤ 60 s or re-collection |
| Authorization targets / provider calls per business invocation | ≤64 distinct targets / ≤64 calls, at most one per uncached target; lower configured bounds allowed, no retries; §4.4 defines preflight refusal and truthful denied-target coverage |
| Deadline | evidence collection shares the provider deadline; timeout → `unavailable`, never `ok` |

## 6. Conformance scenarios

- Describe a connection with a fake custody outage → `custody_reachable: unavailable`, state `custody_unavailable`; fake provider sees zero requests.
- Descriptor fetch and `connections.list` never resolve a credential (fake custody records reads → zero).
- Granted scopes lack `write:jira-work` → mutation refused with `insufficient_scope`, no dispatch; a read requiring `read:jira-work` proceeds.
- Kubernetes fake denies SSAR for namespace `b` → list of `[a, b]` returns `a` items and reports `b` as denied.
- Two namespaces × two resource kinds → four exact targets; fresh cached evidence saves calls, never target slots.
- 65 targets, or missing-evidence calls exceeding the admitted budget → preflight refusal; zero authorization and business requests, no successful truncated prefix.
- One SSAR times out after another succeeds → unavailable, zero business requests; no retry or success-shaped coverage.
- All target permissions denied → forbidden; an allowed namespace with no items remains a distinct complete empty result.
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
| Permission target/budget/coverage values | [auth_access.yaml](../../../../ess/domains/auth_access.yaml); exact binding equality, numeric ceilings, budget consumption, reduction, current authority/time and coverage completeness predicates are normative and not executed by generic ESS schemas. |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether `credential_valid` may call provider introspection by default | no; metadata only |
| Freshness bounds | 300 s reads, 60 s mutations |
| Completion-time verification | only the explicitly selected/admitted validation plan runs it. Mandatory baseline failure prevents publication/completed; optional failure preserves baseline publication but blocks any operation requiring that check. Supported is not mandatory; see [profile §4.1](../../profile/v1alpha1/semantics.md#41-baseline-and-operation-requirements). |
