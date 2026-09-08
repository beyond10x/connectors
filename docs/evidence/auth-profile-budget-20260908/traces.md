# Declared semantic traces — auth access, acquisition and permission budgets

These are textual expected outcomes, independently reviewed against the proposed contracts. They are not executed provider/cache/coordinator/clock tests. Baseline: 1d63519. All successful cases assume current admitted host policy, enabled operation, valid binding and the explicitly supporting reader; removing those prerequisites never grants fallback.

| ID | Given / action | Required observation |
|---|---|---|
| A01 | prometheus.anonymous selected with an admitted HTTPS origin and tenant header | Bounded direct read, no credential resolution, Authorization/cookie/client-certificate placement or fabricated provider account; host admission still required. |
| A02 | Bearer/basic profile selected; configured material is missing/null | Invalid configuration or connection_not_ready at business use; zero anonymous requests and zero credential-alternative fallback. |
| A03 | Anonymous mode includes credential, registration or supplied Authorization | Refuse contradictory configuration/input before provider use; null credential is not an accepted explicit selector. |
| A04 | Caller denied while anonymous provider binding is viable | not_granted before provider use and forbidden-state disclosure; provider anonymity never makes the service anonymous. |
| A05 | Anonymous endpoint returns 401 | Provider refusal; no refresh, browser, upgrade, other profile or destination retry. |
| A06 | Explicit prometheus.via_parent with separately admitted Grafana parent | Parent places its own pinned credential only at fixed hop; child has no generation/account/secret and inherits no grant. Same business result schema as direct. |
| A07 | Child grant denied, parent otherwise allowed | Refuse child before forward; parent permission cannot satisfy the child grant. |
| A08 | Parent revoked/unavailable or observation mapping changed | Local child revoked/disabled precedence still wins; otherwise parent_degraded/route_unavailable. No direct dial or secret acquisition fallback. |
| A09 | Parent publishes a new generation after child admission | Old child route evidence/admission invalidated; no forwarding with old proof or silent substitution. New validation/admission required. |
| A10 | Materialize via a parent that is itself via | Refuse second hop, no proxy request. |
| A11 | Ready anonymous/via_parent connection described | external_identity:null; not an invented account, credential_valid:ok or child CredentialGeneration. Current safe binding status only. |
| A12 | Owner-checked Docker socket configured | Validate admitted path/peer transport; no synthetic OAuth credential or anonymous HTTP fallback. Concrete race-safe transport admission remains a binding gate. |
| A13 | Change access mode, fixed target/parent or host owner; change allowed tenant-header policy | First set requires a new independently admitted connection. Permitted same-binding policy revisions invalidate old checks/admissions/cursors/sessions; no credential-repair or ref-repurposing shortcut. |
| P01 | Uncached list pods targets a,b; SSAR allows a, denies b | Two authorization calls before any resource read; read a only. authorization includes both exact targets, complete:false at coverage and root; b is denied, not empty. |
| P02 | Two namespaces × four configured resource kinds | Eight distinct tuples, up to eight calls; at most four concurrent, shared provider deadline. No namespace-wide shortcut. |
| P03 | Target a repeats and has exact fresh allowed evidence | Deduplicate exact tuple; target still counts once, zero new call for it. Re-evaluate current host policy and final dispatch fence. |
| P04 | All 64 targets have fresh exact cached evidence | Fits target ceiling and zero-call policy; still bounded target work and current checks. |
| P05 | 65 targets even though all cached | invalid_input before any permission/resource request; no successful prefix, cursor or complete discovery publication. |
| P06 | Two uncached targets, admitted remaining call budget one | unavailable before any provider request; reserve whole missing-evidence budget, no partly checked prefix. |
| P07 | One allow then timeout/missing status/malformed SSAR | unavailable, no resource reads. Attempts consume slots, no retry or interpretation as denial. |
| P08 | Zero deadline before checking; other checks already in flight at expiry | No new send; stop scheduling, cancel/await issued work. No resource request from unchecked evidence and no refunded issued slots. |
| P09 | All normalized targets denied | forbidden, never success with an empty collection. |
| P10 | One allowed namespace returns zero objects | Allowed complete empty result; distinct from denied or unavailable namespace. |
| P11 | Current cache is for list services; request is get services/proxy for a named Service | Cache mismatch; require exact new check under shared budget. List permission grants neither named get nor proxy. |
| P12 | Cache differs in generation, namespace, resource, name, subresource, provider authority or admitted context | Unusable; recollect only within admitted budget or refuse. Age alone cannot authorize a mismatch. |
| P13 | Evidence age 61 s for mutation; same identity after credential refresh | Mutation evidence too old; refresh invalidates permission evidence regardless of age. No transfer or business-write retry. |
| P14 | Required evidence invalidates before a later resource dispatch | Stop dispatch; discard partial payload and return applicable refusal. Earlier read requests may have occurred and are not undone. |
| P15 | Supported partial result; only denied targets remain | complete:false with explicit coverage, no fabricated continuation solely for denial. |
| P16 | Continuation sees changed permission/coverage/configuration or withdrawn host grant | Current host denial refuses; changed otherwise-admitted continuation binding gives stale_cursor and requires a fresh read. No permanent permission in cursor. |
| P17 | Receiver only supports the current strict Page | Cannot advertise selected authorization-coverage partial profile; no extra fields, forged provider items or error-message metadata. |
| P18 | Discovery receives denied/unknown/incomplete coverage | Cannot publish it as a complete replacing generation or withdraw unseen resources; broader coverage model remains separately owned. |
| P19 | Read needs SSAR POST; business code asks to POST a different resource | Private fixed auth-query port permits only separately admitted exact SSAR within its budget; business POST refuses. No identity probe added to ordinary list. |
| P20 | Required multi-target read normalizes to no targets | invalid_input, zero requests; no empty set manufactured as authorization success. |
| Q01 | Kubernetes bearer/mTLS static_config activates | No auth.begin/acquisition id/UI/registration or writable-custody copy. Separately admitted coherent capture, identity/baseline validation and acknowledged binding publication before use. |
| Q02 | Configured path overwritten with account B after account A binding | New material cannot reuse A evidence; admitted validation refuses identity mismatch. Still-valid A pin never reloads B; no reassignment by filename. |
| Q03 | Restart sees a locally revoked static binding | Revocation remains terminal; reloading identical configuration cannot republish it as ready. |
| Q04 | Static no-credential profile activates | Validate current destination/route and required verification without secret capture or fictitious completed acquisition. |
| Q05 | auth.begin called on static_config | unsupported before acquisition allocation/provider use; no implicit conversion to static_entry. |
| Q06 | static_entry selected | Protected UI/entry and writable custody/coordinator required; no OAuth URL/registration requirement. Success acknowledges validated durable publication. |
| Q07 | Authorization code missing authorize_url, token_url, sources or explicit sourced PKCE/client-auth choice | Future profile authoring/advertisement refuses; no inferred endpoint or provider behavior. |
| Q08 | Client credentials has token URL/source/registration and declared validation, no authorize URL/UI | Valid proposed noninteractive path; no browser/callback/PKCE requirement; actual implementation still prerequisite. |
| Q09 | Client credentials includes authorize URL, callback, PKCE or refresh_token grant | Contradictory first-profile declaration refused; no ignored browser fields or invented refresh flow. |
| Q10 | Reserved password/workload/exec/host-issued flow plus enable flag | No generic acquisition advertisement/handler from flag or enum; host-issued session authority keeps its separate owner. |
| Q11 | Grafana configured-first deployment versus later managed entry | First path selects configured service-account profile/static_config/read_only; later separately selected static_entry profile needs managed connection/versioned custody. |

The 50 [typed values](typed-values.json) check schema shapes, including seven expected shape rejections. Four shape-valid semantic contradictions are deliberately accepted to show that ESS generic projection does not run these traces or enforce their predicates.
