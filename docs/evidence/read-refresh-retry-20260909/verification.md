# Read refresh retry: verification and dispositions

Story: contracts-read-refresh-retry. Base: 8e1836cad8ae1b2127ce9ae306c6d8131960db4c.
The [pre-correction cases](baseline-cases.md) were recorded before contract changes.
The normative decision is [read-refresh-once/v1alpha1](../../../contracts/auth/capability/v1alpha1/read-refresh-once.md).
The following is a manual textual scenario audit; it does not execute provider,
codec, coordinator, clock, permission, audit or dispatch behavior.

| Case | Selected observation/refusal after revision | Rule |
|---|---|---|
| RR01 | Legacy/unselected 401: one business call, zero refresh participation or redispatch caused by this invocation; native unauthorized error remains | Binding §1, capability §§2/6/7, configured service Wire boundary |
| RR02 | Combined profile with eligible direct bearer read: business call g0 returns complete 401; one admitted refresh publishes g1; new admission and current evidence permit one identical-target read using g1; its success is final | Binding §§2–4 |
| RR03 | The g1 read also returns 401: unauthorized, two business calls total, no second refresh or third business call | Binding §§3/5 |
| RR04 | First 403/429/5xx/timeout/redirect/malformed/oversized reply: native terminal error, one attempted business request, zero refresh participation | Binding §3 step 2; both encoded and decoded response bounds apply |
| RR05 | Mutation, anonymous, static_config, client-credentials reacquisition, exec plugin, generic realization, mediated/federated or streaming combination: cannot advertise/select this first retry binding; no implicit fallback | Binding §2; capability §4 |
| RR06 | Work already used 14 of the original 15 provider seconds: at most one second remains for refresh wait, checks and redispatch; no new window. A 4 MiB encoded/decoded response cap is not reset by credential rotation | Binding §4 |
| RR07 | X was checked before the first read: its call remains consumed; g0 evidence cannot authorize g1. Use independently fresh g1 evidence or unavailable with zero new check/redispatch. If X was initially cached and its call slot unused, one g1 check may proceed inside the original complete missing-budget reservation | Binding §4; unchanged evidence §4.4 |
| RR08 | Reauthorization/invalid/revoked/uncertain refresh or invalid lineage: connection_not_ready; narrower grants: insufficient_scope; custody/coordinator/publication uncertainty: unavailable. No private enum/credential/fence enters the response | Binding §5; unchanged evidence §4.3 and acquisition §4.1 |
| RR09 | Two invocations first dispatched with g0 before refresh cutoff: the coordinator authorizes at most one token exchange. Each still-live invocation may observe the same acknowledged g1 and use its own remaining business/permission budgets and fresh admission. Waiters send zero token requests | Binding §3; acquisition conformance; existing RefreshAttempt causation |
| RR10 | Publication invalidates original admission; revocation, replacement, host denial, stale identity/grants or permission evidence before second dispatch blocks it. g1 material is never installed in the old g0 admission | Binding §3 steps 4–5; current host refusal precedence and evidence §§4.1–4.3 |
| RR11 | Deadline/cancellation precedes publication: this invocation terminates and never resumes its business read. The coordinator may only finish/recover its already-authorized protocol under existing rules; it gains no second exchange | Binding §§3–5 |
| RR12 | Source attempt already published g1: observing it costs zero new token requests and may allow one fresh admission. If g1 is superseded again, refuse unavailable; no chain chase, new source or second selected attempt | Binding §3 steps 3–4 and §5 |
| RR13 | Redispatched native 404 stays not_found. A known read result followed by failed final-audit acknowledgement retains its result with incomplete audit. Neither circumstance retries or becomes mutation outcome_unknown | Binding §5; existing service audit semantics |
| RR14 | Exact new name P.read-refresh-once.v1alpha1 plus authored support and current revision are required. Base P, a guessed suffix, stale revision, unsupported reader or legacy projection cannot silently select retry | Binding §1; coordinator compatibility/v1alpha2 patch |

All fourteen baseline cases now have an explicit textual decision. The cases also
cover the error class beyond the original 401 example: no retry for ambiguous
transport/provider observations; no state/budget reset when credential identity
changes; no behavior change inferred from otherwise compatible wire shapes.
F15 is addressed by explicit old/new profile selection, counts, deadline and
fresh admission. E05 is addressed by the capability disposition row, preserved
configured-service rule and explicit compatibility integration. These are owner
implementation dispositions, subject to independent review and coordinator
integration; they are not immutable review outcomes or a story status change.

The implementation intentionally reuses the settled shared ESS entities:
CredentialGeneration, RefreshAttempt and DispatchAdmission. No owner/cardinality
or lifecycle is added. The first admission remains terminal; a successor requires
a new admission. The original refresh attempt retains consumed authorization and
publication/recovery fencing. The binding explicitly records the cross-model
budget/admission/lineage/cancellation predicates as UNMAPPED runtime obligations.

Executed model checks use the pinned ESS 0.20.0 at
`/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess`, with TMPDIR
inside the assigned worktree. Before and after the textual correction:

```text
ess specify validate --path ess
connectors v1 — 14 file(s), valid

ess specify compile --path ess --out <scratch>/ess-{before,after}.json
connectors v1 — 14 file(s), 218 declaration(s), compiled to <scratch>/ess-{before,after}.json
```

Each invocation exited 0. The two canonical IR files compare equal. These checks
establish unchanged model validity and declarations, not execution of the fourteen
manual scenarios. No runtime tests were added or run for this specification-only
unit; the coordinator runs the combined repository gate after integration.

The exact unapplied coordinator patch is retained in this worktree's scratch:
`.local/spec-completion-20260909/read-retry-coordinator.patch`. It adds explicit
selection/disposition/limit references to service compatibility and v1alpha2.
Those shared files, indexes, design and planning remain coordinator-owned. This
record requires that patch to be integrated before claiming cross-document closure.
