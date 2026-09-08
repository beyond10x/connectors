# Review dispositions — auth access and permission budgets

The original reports are retained unchanged in AEP review-result artifacts and copied beside their source-hash manifests. A fixed disposition refers to the changed normative contracts and the declared traces; it does not claim runtime behavior was implemented. Each review finding is attributed to one owning story even where a correction touches shared files.

## Original intake

| Finding | Owning story | Correction |
|---|---|---|
| F09 / E10 | contracts-anonymous-auth | Explicit anonymous and parent-mediated purpose/subject/scheme/flow/capability combinations, no child credential/account fabrication, current independent admission and no fallback. |
| F08 | contracts-permission-budgets | Exact target equality, 64-target/64-call/4-concurrent ceilings, preflight budget reservation, unknown refusal, bounded denied-target coverage with supporting reader. |
| E09 / E29 | contracts-acquisition-profiles | static_config activation versus managed entry, flow-specific required/forbidden fields, reserved support distinct from enablement and implementation. |

## AP-A-01

Fixed — anonymous-auth. Profile §4.2, capability and monitoring rows select .anonymous/http-anonymous and forbid contradictory material or implicit fallback. Traces A01–05.

## AP-A-02

Fixed — anonymous-auth. .via_parent selects separate parent authentication, fixed route and independent admission without child secrets/grants. Business semantics remain compatible while auth metadata differs. Traces A06–10.

## AP-A-03

Fixed — anonymous-auth. Material rules remain mandatory for real credentials; no-child-credential bindings have null identity and their own current binding predicates. Socket path/peer admission is explicitly separate and unmodeled, not a synthetic generation. Traces A11–13.

## AP-A-04

Fixed — permission-budgets. Evidence §4.4 defines exact coordinates and finite target/call/concurrency budgets, shared deadline, original expiry, current admission and no generation transfer. Traces P02–08/P11–14.

## AP-A-05

Fixed — permission-budgets. Pre-resource refusal for unknown/exhausted checks, explicit allowed/denied coverage under a new selected reader, no complete discovery publication or withdrawal from incomplete scope. Traces P01/P05–10/P15–20.

## AP-A-06

Fixed — acquisition-profiles. static_config activates deployment references or a nonsecret binding without Acquisition/UI/writable-secret-copy requirements; validation/publication remains admitted. Traces Q01–06.

## AP-A-07

Fixed — acquisition-profiles. Code/client-credentials/static field matrix separates endpoint, registration/client-auth, PKCE, callback and refresh requirements; source claims remain reviewed. Traces Q07–09.

## AP-A-08

Fixed — acquisition-profiles. Specified path, reserved vocabulary, instance enablement and runtime advertisement are distinct; index/adapter rows and management boundary follow the acquisition matrix. Trace Q10.

## AP-B-01

Fixed — anonymous-auth. Explicit direct and via-parent selections, placement and strict reader requirements; inherited_from_source remains candidate placement only. Traces A01–10.

## AP-B-02

Fixed — anonymous-auth. No fabricated material/identity or inherited authority; material F05 checks remain intact, socket transport remains separately gated, static/anonymous/child-route 401 does not grant refresh. Traces A05–13.

## AP-B-03

Fixed — permission-budgets. Exact normalized tuples and finite target/query/concurrency limits include cache hits and attempted calls. Impersonation is refused in this first profile; a future context must become a reviewed equality coordinate. Traces P02–08.

## AP-B-04

Fixed — permission-budgets. Exact checking authority/generation/configuration/target and original deadlines govern reuse; current host policy and final fence remain mandatory. Missing/malformed/contradictory provider status is unavailable, not a denial or success. Traces P03/P07/P11–14.

## AP-B-05

Fixed — permission-budgets. Declared bounded authorization coverage and explicit refusal distinguish known denial from unchecked/unknown targets; no additions to the existing Page or metadata hidden in items/errors. Traces P01/P05–10/P15–20.

## AP-B-06

Fixed — permission-budgets. Private exact auth-query POST is separately admitted and budgeted; the business method gate is unchanged and no hidden identity probe is allowed. Trace P19.

## AP-B-07

Fixed — acquisition-profiles. static_config activation does not allocate a flow or revive revoked/reassigned bindings. Grafana's configured-first profile explicitly uses read_only/static_config; later managed entry needs its distinct profile and versioned custody. Traces Q01–06/Q11.

## AP-B-08

Fixed — acquisition-profiles. Flow-specific field and support matrix prevents invented code endpoints/UI for client credentials and flag-only reserved handlers. Traces Q07–10.

## AP-A-R2-01

Fixed — anonymous-auth. Grafana's active §3 contract map now uses the same .anonymous/.via_parent names and capability set as §5; the retired ambiguous label is history only.

## AP-A-R2-02

Fixed — anonymous-auth. Both normative and conformance 401 rules are scoped to selected refresh support and separately permitted read retry; anonymous/static/child-route access cannot upgrade or repeat implicitly. Trace A05.

## AP-A-R2-03

Fixed — anonymous-auth. Fixed route/parent/target/mode/host ownership cannot be repurposed by revision. Changes require a new admitted connection; permitted same-binding policy/evidence updates only invalidate affected admissions and continuations. Trace A13.

## AP-B-R01

Fixed — anonymous-auth. The normative capability rule now requires a separately selected read-retry binding, fresh generation admission/permissions and original remaining budget. No automatic retry support is inferred.

## AP-B-R02

Fixed — acquisition-profiles. Authorization code explicitly declares sourced client-auth method/policy, not only registration kind. This matches trace Q07 without inventing a concrete provider mechanism. The review's line-reference correction is preserved separately.
