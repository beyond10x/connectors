# Declared discovery-profile and persistence traces

These 43 cases are textual conformance expectations checked against owner prose. They are **not executed recognizer, canonicalization, storage, crash, authority or session tests**. Schema acceptance is reported separately and cannot establish these results.

Owners: [discovery §4.5](../../../contracts/discovery/resources/v1alpha1/semantics.md#45-fixed-declarations-recognition-and-configured-source-identity) and [design §31](../../design.md#31-host-persistence-ownership-and-atomicity), with that inventory's links to each family authority.

| ID | Decisive input/history | Required result |
|---|---|---|
| PP-D01 | services.observe declaration selects kubernetes-service-targets and its configured source | Exactly that profile/source rule; no generic resources.observe dispatch |
| PP-D02 | datasources.observe request includes profile:kubernetes-service-targets, origin or namespace override | Closed input refuses invalid_input before provider work |
| PP-D03 | Selected source missing, ambiguous or incompatible; explicit connection disagrees | Refuse before provider work, no fallback to another source |
| PP-D04 | Gateway alias resolves to the fixed leaf declaration | Preserve leaf contract/profile/source meaning; alias cannot change it |
| PP-D05 | Receiver changes namespace mode, mapping or projection | New selected scope/revision and old cursor refusal; no silent reinterpretation |
| PP-D06 | Service name exactly argocd-server; stable label absent | Argo inferred marker, candidate:null, no callable route |
| PP-D07 | Helm-prefixed Service name, app.kubernetes.io/name exactly argocd-server | Same Argo observation-only marker; name AND label conjunction is not required |
| PP-D08 | Ordinary argocd-server-metrics, argocd-repo-server or argocd-redis names/labels | No Argo marker from substring or component label |
| PP-D09 | Only legacy app/k8s-app/name label equals argocd-server | Does not select Argo in this profile; explicit disposition differs from broader old aliases |
| PP-D10 | Stable label ARGOCD-SERVER versus padded " argocd-server " | Declared ASCII lowercase recognizes the first; no trimming means the second does not match |
| PP-D11 | Exact Argo stable label plus monitoring hint | Argo exact arm wins before monitoring; still no candidate/route |
| PP-D12 | Multiple monitoring substring hints | Deterministic old priority grafana, alertmanager, loki, prometheus; inference grants no authority |
| PP-D13 | Unrecognized well-formed Service inside admitted scan | Observation remains with null recognition/candidate and unknown auth/reachability; counts against scan limits |
| PP-D14 | Grafana reviewed type versus unknown plugin or allowlist suppression | Known type has declared recognition; unknown remains observable with null marker/candidate; allowlist affects candidacy, not provider absence |
| PP-D15 | Same origin/context label under two configured instances/connections | Distinct configured source identities, no automatic physical-cluster deduplication |
| PP-D16 | Equivalent accepted canonical HTTPS origin spelling; only alias/context display changes | No new identity solely from spelling/display; canonicalizer remains a required strict binding |
| PP-D17 | Configured origin or admitted trust/authority boundary changes | New independently admitted source connection; no old observation/route continuity assertion |
| PP-D18 | Physical cluster replaced behind an unchanged authority | No attestation guarantee is claimed; known replacement invalidates continuity and requires re-admission, no invented identity probe |
| PP-P01 | Custody write durable but active-reference publication absent | Orphan is not active/dispatch authority; cleanup does not establish refresh outcome |
| PP-P02 | Custody write acknowledgement unknown | No publication proof; resolve only under custody/coordinator contract, never assume material active |
| PP-P03 | Same rotating material offered through another generation/alias | One refresh-source authority; UUID or pointer difference cannot authorize another exchange |
| PP-P04 | Refresh authorize commits, process dies before/after send | Source remains consumed; no retry. Stored acknowledged response, if present, permits publication-only recovery |
| PP-P05 | Refresh publish races local revoke/replacement/final dispatch | Shared binding metadata authority orders them; private fence/current generation wins, no stale resurrection |
| PP-P06 | Same-identity refresh changes generation/private publication fence | Invalidate old dispatch/permission evidence; public semantic revision and F02 fingerprint stay unchanged solely for refresh |
| PP-P07 | Target/identity/operation meaning changes | Semantic revision/binding changes; old live key conflicts under F02, no silent fingerprint preservation |
| PP-P08 | Acquisition completion belongs to one coordinator and repair target | Recheck stored current scope; custody and metadata acknowledgements precede completed; gateway cannot recreate owner after unknown response |
| PP-P09 | Caller attempts to write readiness status directly | ConnectionAuthorityPort updates facts; readiness is reduced from current authority/evidence, no independent ready transition |
| PP-P10 | Material snapshot/pin or evidence continuity lost | Capture/validate anew or refuse; never reread mutable source under an old admission; refresh alias history remains required |
| PP-P11 | Optional permission cache lost or expired | Separately admitted check within original F08 budget or refusal; cache absence grants no positive/negative permission |
| PP-P12 | Concurrent namespace/key reservation and Prepared attempt creation | One atomic MutationKeyPort/MutationAttemptPort anchor; no separate orphan reservation authorizing send |
| PP-P13 | Terminal attempt settles while replay result/timestamps are written | Atomic settlement/replay state and original expiry; replay never slides retention |
| PP-P14 | Pending/Quarantined key at capacity or uncertain expiry/clock | Retain live deduplication fact; refuse new capacity/unsafe reuse, never infer missing/expired |
| PP-P15 | Approval spend acknowledged then abort wins before dispatch gate | Approval stays spent, no provider send; spending and gate are distinct decisions |
| PP-P16 | Approval spend or dispatch-gate acknowledgement ambiguous | No send permit; observe/recover the owning record conservatively, never replay success as a new permit |
| PP-P17 | Same approval reaches direct and gateway ingress/replicas | One executing-leaf spend owner; gateway does not redeem; no automatic tombstone expiry |
| PP-P18 | Delegation nonce consume succeeds but expiry/key/policy/deadline check fails after acknowledgement | No authenticated entry; nonce remains consumed until trustworthy retirement |
| PP-P19 | Admission audit unavailable versus final audit append failure | First prevents dispatch; second preserves known business outcome with incomplete audit and no resend |
| PP-P20 | Discovery view/private index publication loses predecessor/binding fence or acknowledgement | No stale/new claimed authority; same-attempt observation or refusal under F13, no rescan to manufacture certainty |
| PP-P21 | Route revalidation competes with parent/child revoke or observation update | Shared current-fact comparison and route CAS; unknown/stale result grants no forward or repointing |
| PP-P22 | Session establishment token versus live lease versus owner restart | Separate redemption/supervisor responsibilities; redeem before data, existing cutoff/teardown limits, no reattach or recreated live authority |
| PP-P23 | Restore stable host identity from a stale backup missing nonce/spend/key/refresh history | Refuse affected profile; no split active authority or reset of uniqueness. Lost private discovery continuity follows its new-epoch rule |
| PP-P24 | Ordinary custody error/log/metric rendered | No secret, scope/version/generation/private route reference; use separately admitted safe correlation |
| PP-P25 | Deferred event/checkpoint/assignment/execution store named; ESS value/entity exists | Inventory is not an implemented backend, event claim approval, durable session continuity or an invented entity ownership graph |

E07/E14/E32 and E28 are the source findings covered. Remaining mutation classification/restart/log/document/read-retry/media/index/evidence/vocabulary stories and broader catalog/open-decision review retain their own scope. Actual backend choice, entity graph gaps and unsupported authority bindings must be proved before the affected implementation/profile is advertised.
