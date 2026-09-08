---
format: aep.planning-md/1
id: review-result:wire-family-r1-20260908
kind: review-result
status: active
title: Independent family compatibility review, first pass
relations:
- reviews: story:contracts-wire-compatibility
revision: 1
---
# Independent compatibility review A — proposed contract families

Reviewed on 2026-09-08, read-only. Scope: E02 / `story:contracts-wire-compatibility`; every proposed `v1alpha1` semantic family, including catalog, against the implemented service/core wire. The parent reviewer owns the proposed governed service envelope and negotiation protocol. I did not inspect another review. No runtime or tracked files were changed. Exact source hashes and byte-for-byte snapshots accompany this report in `source-hashes.json` and `sources/`.

Verdict: **needs compatibility fixes**. The semantic proposals are useful, but neither optional fields nor arbitrary JSON payload slots establish compatibility. A family contract version, a selected profile, a service envelope version, an adapter specification version, and a stream binding version have different roles. Several documents currently blur these boundaries.

## Baseline actually accepted

- `crates/connectors-core/src/lib.rs:7` fixes `WIRE_VERSION = v1alpha1`. `ErrorCode` at line 13 is closed: `invalid_input`, `unsupported`, `unauthorized`, `forbidden`, `not_found`, `stale_description`, `stale_cursor`, `rate_limited`, `unavailable`, `capacity`, `timeout`, `upstream_protocol`, `internal`.
- `Error` at line 31 is closed to `code`, `message`, optional `retry_after_seconds`. Optional new members still fail old readers.
- `Operation` at line 61 is closed to `id`, `description`, `contract`, **singular** `profile`, `input_schema`, `output_schema`. `Descriptor` at line 72 is closed to `version`, `instance`, `adapter`, `revision`, `operations`, `configuration_schema`.
- `Invocation` at line 93 is closed to `version`, `request_id`, `operation`, `revision`, `input`. The custom response decoder at line 113 accepts only the original `success` or `error` fields: `version`, `request_id`, `status` plus `result` or `error`.
- `input`, `result`, and schemas are JSON `Value`: new **operation-local** shapes can be represented inside these slots. This proves framing representability only, not that a current client understands or is allowed to invoke the new contract/profile. The typed `Page`, `Provenance`, `EndpointObservation`, `Column`, and `QueryResult` in `crates/connectors-contracts/src/lib.rs:7` are themselves closed. For example a resource page's new root `generation` is not an additive member of the existing `Page` type.
- `contracts/service/v1alpha1/semantics.md:25` fixes the current unary paths, implicit configured connection, unknown-envelope-field refusal, 64 KiB request, 4 MiB response, 20 s service execution / 15 s provider budget. It provides no duplex transport or callback binding. `docs/design.md:296` requires explicit versions/profiles and declared information loss/refusals; line 313 forbids silent fallback, dropping guarantees or request resend.

## Family inventory

Classification used below: **envelope** means a new field/enum in a closed core object; **payload** means a selected operation's input/result under existing `Value` slots; **declaration** means safe descriptor/specuration metadata requiring an explicit location; **private** means an SDK/host value that must not be exported merely to satisfy a matrix. A proposed public payload is still unsupported until its schema, profile and binding are explicitly selected. Reserved vocabulary is not callable support.

### Operations mutation

Source: `contracts/operations/v1alpha1/semantics.md:30`, `:81`, `:91`, `:207`.

- Profile: `operations/v1alpha1` + `mutation`; admission modes `not_required`, `required`, `event_claim` (the hyphenated `event-claim` also appears in old-disposition prose).
- Envelope/declaration additions on `Operation`: `effects[]`, `semantic_effects[]`, `risk`, `idempotency.{kind,key,retention_seconds}`, `approval`.
- Invocation additions: `idempotency_key`, `approval.{reference,evidence}`. Connection selection is required by the surrounding auth family but has no chosen field here.
- Response additions currently sketched: `effect`, `original_request_id`; required semantics additionally distinguish effect classification (`not_attempted`, `refused`, `applied`, `unknown`), diagnostic cause, original attempt correlation and replay delivery metadata. `replayed` cannot replace classification; a replayed refusal/unknown is not a success.
- New public codes: `approval_required`, `approval_refused`, `approval_replayed`, `idempotency_conflict`, `outcome_unknown`. No new `not_attempted` error code. All these new codes fail the current `ErrorCode` reader.
- Private: attempt state, dispatch gate, namespace/fingerprint, reservation/refresh coordination and ESS `Observation` are not public envelopes. Safe wire identity/correlation must not export host-private approval/credential handles.
- Required disposition: explicit new closed envelope or another equally explicit tested binding, and projection refusal/hiding of all mutations from the old read-only profile. Omitting mutation metadata while retaining invocation access is unsafe. Current §7's optional-fields alternative does not prove compatibility with already-installed readers.

### Auth connection

Source: `contracts/auth/connection/v1alpha1/semantics.md:29`, `:74`, `:113`.

- Profiles: `configured`, `managed`. Payloads for `connections.list`, `.describe`, `.revoke`: `connection`, `instance`, `auth_profile`, `scope`, `actor`, `external_identity.{kind,display,id}`, `route.{kind,parent,profile}`, `status.{state,since_unix_ms,next_action}`, `revision`, `created_unix_ms`. Lifecycle vocabulary includes `ready`, `reauthorization_required`, `insufficient_scope`, `custody_unavailable`, `parent_degraded`, `disabled`, `revoked`; acquisition later creates the connection.
- Public invocation selects a connection ref in managed mode, but its envelope-versus-input location is not fixed. Descriptor adds a `connections` capability flag: this is a closed-object extension, not merely a new string.
- New public codes: `connection_not_ready`, `route_unavailable`; no credential-generation/snapshot/store references may be public.
- Valid no-change claim is restricted to the **unchanged** implicit configured profile. Explicit connection selection, safe management payloads, readiness refusals and capability advertising require selected contracts/bindings. The existing configured profile must not silently acquire mandatory generation-validation or repair semantics under a compatibility projection.

### Auth profile

Source: `contracts/auth/profile/v1alpha1/semantics.md:30`, `:58`, `:72`, `:98`, `:105`.

- Full provider-owned declaration: `id`, `purpose`, `subject`, `scheme`, `acquisition.{flow,authorize_url,token_url,pkce,refresh,grants,registration.{kind,redirect}}`, `scopes.{requestable,minimum}`, `identity.{kind,from}`, `revocation.{supported,url}`, `capabilities[]`, `evidence[]`, `sources[]`.
- Public safe projection specifically promises profile IDs, purposes, schemes and flows; safe selection/schema version must be declared. It does not authorize blindly exporting the entire authored object. Per-operation `requires_auth` is an alternatives array of `{profile,scopes?}`; spec/config introduces `auth_profiles`.
- Closed `purpose`: service_account / delegated_user / app_level / inbound_verification / transport_identity / trunk_registration. `subject`: app / user / none. `scheme`: http_bearer / http_basic / http_signing / mtls / socket_peer / exec_plugin / sip_digest / session_authority. Flow: static_entry / static_config / oauth2_authorization_code / oauth2_client_credentials / oauth2_password / workload_identity / exec_plugin / host_issued. Reserved/refused-by-default is distinct from implemented support.
- Scenario emits `insufficient_scope`, absent from current core codes. The `http_signing` reservation permits later declaration under this vocabulary only; it cannot silently authorize a previously unsupported handler/profile without negotiation.
- Private registration secrets remain configuration/custody data. Adapter-spec additions are a separate reader-compatibility question from wire versioning.

### Auth acquisition

Source: `contracts/auth/acquisition/v1alpha1/semantics.md:12`, `:28`, `:77`, `:147`.

- Profiles: static_entry / oauth2_authorization_code / oauth2_client_credentials; reserved workload_identity / exec_plugin / host_issued. These are not identical to auth.profile's complete flow vocabulary (which also contains static_config and compatibility-only oauth2_password).
- Management payloads: `auth.begin(profile,requested_scopes?,repair_of?)`, `.complete(acquisition_ref,evidence)`, `.status(acquisition_ref)`; results contain `acquisition`, `expires_unix_ms`, `action.{kind,url,fields?}`, or `state`, `connection`, `reason`. Action kinds browser/protected_entry; status pending/completed/failed (lifecycle also expired); failed reasons expired / refused_by_provider / identity_mismatch / scope_insufficient / custody_unavailable.
- The secret-bearing completion evidence belongs to the dedicated trusted callback/entry ingress, whose routes/codec are not provided by current unary service v1. The ordinary management operation's invocation cannot become an accidental generic credential-entry channel.
- `auth.refresh` and provider begin/exchange/refresh/revoke interfaces are explicitly private. Refresh outcome vocabulary refreshed / reauthorization_required / insufficient_scope / invalid_or_revoked / custody_unavailable / uncertain is **not** automatically a list of new public `Error.code` variants. Public safe status projections need declared mappings; private ledger/fence/candidate metadata stays private.
- Old connect-session name mapping is only a proposed facade, not a claim of byte/authority compatibility or an implemented endpoint.

### Auth custody

Source: `contracts/auth/custody/v1alpha1/semantics.md:12`, `:27`, `:89`.

- Profiles read_only / versioned. Private port write_new/read/delete; private SecretVersionRef, SensitiveMaterial, scope, active_credential, superseded references, revision-CAS publish; outcomes Missing / Unavailable / Denied / Published / Conflict.
- No public invocation or arbitrary secret retrieval is proposed. The safe connection status `custody_unavailable` belongs in that payload vocabulary, not a new core error by implication. Any management-only `requires` advertising custody must be classified separately from a public `provides` capability.
- No public wire change can be valid for retaining the original read_only use, but SDK/source changes and required versioned-binding guarantees remain separately negotiated capabilities. Do not make private storage versions wire fields for convenience.

### Auth capability

Source: `contracts/auth/capability/v1alpha1/semantics.md:12`, `:30`, `:101`.

- Named capabilities: http-bearer / http-basic / http-signing / mtls-client-identity / socket-peer / exec-credential-plugin / sip-credential-lease / session-authority / inbound-verifier / mediated-http.
- The capability objects, connection/profile/destination/limits binding examples, validated generation pin, request and secret placement ports are explicitly private SDK values. `AlreadyRedeemed` is an internal verifier outcome until a public authority-binding mapping is chosen.
- Public names reside inside **new profile declarations**, for which the current descriptor has no slot. §7's blanket 'No wire change' is false for that advertising surface; it is valid only for retaining the old http-bearer transport implementation without adding public fields or changing selected semantics.
- Catalog separately asks for http-header/http-query placement capabilities; neither is in this capability vocabulary. They need explicit reserved/new-profile disposition, not an inference that a string means support.

### Auth evidence

Source: `contracts/auth/evidence/v1alpha1/semantics.md:28`, `:46`, `:145`.

- `connections.describe` public `evidence` projection: `connection`, `collected_unix_ms`, `checks[]` containing `check`, `result`, optional `expires_unix_ms`, `source`, `external_identity`, `missing[]`, `subject`; derived `state`.
- Check names custody_reachable / credential_present / credential_valid / identity_check / scope_check / permission_check / verify_operation; result vocabulary ok / missing / invalid / insufficient / denied / unavailable / uncertain / not_run. These are payload enums, not `ErrorCode` variants.
- Public refusal scenarios require `connection_not_ready` and `insufficient_scope`, beyond the current closed core. Private generation IDs, snapshot handles, source generations, admission pins and secret version references must not enter public projection.
- Optional `evidence` can fit a newly selected management result schema under `result`; it does not imply old typed `connections.describe` readers accept an unversioned field. That operation does not exist in the implemented v1 slice, so 'no wire change beyond' must be recast as a payload/binding rule.

### Datasource records document

Source: `contracts/datasources/records/v1alpha1/semantics.md:11`, `:29`, `:87`.

- Profile family document, named confluence-page / jira-issue; existing family list. Representations confluence.storage / jira.fields.v2; confluence.view / jira.adf / gitlab-file adaptations are future/reserved.
- Input `id`, `representation`, `max_body_bytes`. Result `item` (provider identity/title/status/version.{number,when}), `body.{representation,bytes,content,truncated}`, `complete`, base `provenance.{instance,resource,observed_at_unix_ms,source_revision}`. Nullable version/provenance revision is explicit. No cursor.
- Only base codes. New shapes fit unary `input`/`result` slots with an explicitly selected document schema/profile; they are not a `Page` extension and must not be delivered to a list-profile reader by dropping/relabeling fields. The broad no-wire-change claim can mean only unchanged envelope and unchanged existing list operations, never semantic compatibility with all readers.

### Datasource logs

Source: `contracts/datasources/logs/v1alpha1/semantics.md:11`, `:27`, `:81`, `:112`.

- Profiles logql-range / kubernetes-pod-logs / docker-container-logs.
- Inputs: LogQL query/start_unix_ns/end_unix_ns/direction/limit/max_line_bytes; Kubernetes namespace/pod/container/since_seconds/tail_lines/max_bytes; Docker container/streams/since_unix_s/tail_lines/max_bytes (until reserved pending authoring).
- Result lines[].{timestamp_unix_ns,stream,line,line_truncated,source}, window.{start_unix_ns,end_unix_ns,direction}, complete, truncation.{by,streams_dropped}, next_cursor, base provenance. By vocabulary limit/bytes/time/provider; optional source stdout/stderr/null; timestamp string or null. Host opt-in redaction is separately said to be flagged per line but its exact public flag spelling is not fixed in the proposed output (old redacted is cited).
- Only base codes; `Timeout` before dispatch only is an underspecified post-dispatch read error rule, owned by other semantic work rather than solved by inventing a mutation classification.
- `http.extra_headers` tenant binding is receiver-owned configuration, not request input/envelope; current configuration does not implement it. New configuration schema/version and an explicit selected binding are required, with no silent ignore/alias.
- All read payloads can fit `Value` slots under their selected new profile. Old pod-log text blob is not preserved; line joining is a separately tested lossy facade, never an implicit downgrade. Cursor behavior must follow the settled continuation story, not field-dropping compatibility.

### Datasource series

Source: `contracts/datasources/series/v1alpha1/semantics.md:11`, `:27`, `:91`.

- Profile promql-range; promql-instant/promql-labels reserved. Input query/start_unix_s/end_unix_s/step_s/max_series/max_samples_per_series.
- Result result_type=matrix, series[].{labels,samples,samples_truncated}, series_truncated, complete, warnings[], window.{start_unix_s,end_unix_s,step_s}, base provenance. Samples retain numeric (possibly fractional) timestamp plus string value; vector/scalar/string belong to reserved instant forms.
- Only base codes. `min_step_s`, `max_window_s` and advertised minimum step are declaration/profile metadata requiring a chosen schema location. Tenant extra headers follow the logs configuration disposition.
- New profile payload fits unchanged envelope if explicitly selected. Dropping old status/instant value is not backward semantic compatibility; preserve existing profiles or refuse, with any adapter facade separately specified.

### Resource discovery

Source: `contracts/discovery/resources/v1alpha1/semantics.md:11`, `:29`, `:100`.

- Profiles grafana-datasources / kubernetes-service-targets. New resources.observe input profile/limit/cursor.
- Result items[].{id,source_connection,observed_type,title,locator.{kind,digest},candidate.{target_adapter,route_profile,confidence},auth_requirement.{kind},reachability,generation,observed_at_unix_ms}, next_cursor, complete, **root generation**, base provenance. locator opaque, address reserved; confidence declared/inferred; auth requirement inherited_from_source/separate/unknown; reachability via_source_only/direct_possible/unknown. Unknown candidate is null.
- Only base codes including stale_cursor. Denied namespaces are promised as reported, but no field encodes this coverage in the proposed output; that sibling coverage story must select its payload and it then belongs in the matrix.
- New operation/schema can fit `result` but is not existing typed `Page` because of root generation. endpoint_discovery/host_discovery stay unchanged; no automatic conversion. Provider UID/URL/proxy binding remains private, public observation generation is distinct from private credential generation.

### Mediated HTTP route

Source: `contracts/discovery/mediated_route/v1alpha1/semantics.md:29`, `:43`, `:55`, `:67`.

- Profiles grafana-datasource-proxy / kubernetes-service-proxy. Public/safe capability declaration example `provides[].{contract,profile,targets[]}` requires a descriptor/spec location; current core Descriptor has no provides field.
- Configuration binding child_connection/parent_connection/observation/profile/generation/target_adapter and child capability destination.kind/route/parent are host-private. `forward(binding,method,target_relative_segments,query,headers_allowlisted,body_bounded)` is explicitly host-internal and same-composition only.
- Child-facing `route_unavailable` and `route_refused` require an explicit public mapping if propagated as invocation failures. They are not accepted by current core. Keeping a private enum and mapping only to selected public codes is possible, but the present text does not choose it.
- Contradiction: §3 says 'the wire is the parent's operations surface' while §4 forbids remote exposure and makes forward host-internal. A new public arbitrary provider-traffic operation must not be inferred. Same result semantics for direct/mediated child calls does not imply identical failure code inventories absent route failures in the direct path.

### Sessions

Source: `contracts/sessions/v1alpha1/semantics.md:11`, `:28`, `:53`, `:71`, `:94`, `:154`.

- Profiles outbound / inbound_offer / duplex_transport. Establishment is an explicitly selected mutation operation, so its unary result requires the mutation envelope disposition.
- Public session record session/instance/connection/profile/state/participants[].{role,endpoint}/streams[].{id,contract,profile,direction}/lease.{expires_unix_ms,renewable}/admitted.{revision,authority}/terminal. Terminal contains state/reason/by/at_unix_ms; states offered/establishing/ready/closing/closed/lost. Shared reasons remote_hangup/local_close/cancelled/rejected/expired/revoked/lease_expired/media_overload/media_incompatible/transport_lost/error.
- Establishment result session/state/streams/transport.{binding,endpoint,authority}. The one-use endpoint/authority is dedicated session transport authority, not provider-secret/custody disclosure or a bearerless arbitrary route.
- Duplex messages: session and direction-qualified id plus request(method,body), response(outcome), event(seq,type,body), offer(deadline,streams,participant_context), accept(admitted), reject(reason), cancel(reason), close(reason), lease(expires_unix_ms), revoke(reason), ping/pong. §4 also requires verified ingress/interpreted destination and authenticated revision/endpoints/directions/sequence-bound live lease authority; exact binding fields beyond sketches must be specified by the stream binding, not assumed present because the semantic text says authenticated.
- New codes session_not_ready/session_lost/offer_expired/offer_rejected/lease_expired/revoked are distinct from terminal reasons. They cannot be returned in current core v1 errors. Public response `outcome` inside duplex control needs its own exact schema; service unary errors do not automatically define it.
- No current unary path can carry bidirectional sessions merely by adding a `profile` string. Explicit stream transport/version selection and unsupported-profile refusal are required before establishment. Old call/session/channel facade is prospective; deriving call/channel from streams cannot be treated as lossless without a selected mapping.

### Media

Source: `contracts/media/v1alpha1/semantics.md:11`, `:30`, `:51`, `:62`, `:100`.

- Track profile pcm-s16le-8k-mono-20ms; capabilities dtmf/interrupt, reserved hold/transfer. Track descriptor track/direction/profile/format.{encoding,sample_rate_hz,channels,frame_ms,frame_bytes,endianness}/bounds.{input_queue_frames,output_queue_frames}/capabilities[].
- Binary frame semantic view track/seq/timestamp_samples/bytes/payload, not a JSON wire codec. Exact binary framing/version belongs to the selected data binding.
- Controls track_ready; loss(direction,frames_dropped,seq_from,seq_to); overload(direction); signal(kind,digit,duration_ms); interrupt(track); stream_end(track,direction). Interrupt completion must report cleared-frame count but currently gives no exact response-field spelling; signal/interrupt details remain the sibling media-controls story.
- media_overload/media_incompatible are **terminal reasons already shared in sessions**, not automatically public ErrorCode variants. Unsupported is the current base code for unnegotiated DTMF; before-readiness refusal has no selected new code yet.
- Unary wire compatibility does not establish media compatibility. Select the exact session/control/data bindings and profile/capability set; unsupported/reserved formats must fail before ready without transcoding or silently discarding controls. Preserve established-session timing/cutoff guarantees through every relay.

### Catalog and generic HTTP companion profiles

Source: `contracts/catalog/v1alpha1/semantics.md:38`, `:92`, `:139`, `:165`.

- Catalog realizations index/service; companion profiles generic-http and datasource.records generic-http-page; mutation combination requires its own disposition. Catalog read operations are providers.list, provider.describe, operations.list, bundle.describe, sources.status.
- providers.list input limit/cursor; page items adapter/version/kind_version/contracts[]/operations.{exposed,unresolved,refused}/sources[].{origin,kind,url,revision,sha256,license}/bundle.{format,digest}/toolchain.{ess,rustfmt,generator}, base page/provenance.
- provider.describe input adapter; result bundle descriptor plus per-operation coverage generated/requires_implementation/refused and curation. operations.list input adapter/contract/profile/effects/limit/cursor; item adapter/id/contract/**profiles[]**/effects/risk/idempotency/requires_auth/realization. This catalog item is not automatically a runtime Operation despite similar names.
- bundle.describe input adapter/digest; manifest format/specification_sha256/upstream_sha256/ess/rustfmt/files plus locations[].{kind,reference}; kind path/https, oci reserved. sources.status input adapter; result pinned revision/sha256/fetched_at/refresh policy/drift (unknown/none/upstream_changed).
- Generic runtime descriptor example adds profiles[]/realization/effects/risk/idempotency/requires_auth, and omits the core required singular profile. `requires_auth` is a string here versus alternatives array in auth.profile. Multiple semantic profiles need one normative composition representation and digest rule; a wire-version label alone does not reconcile them.
- Generic result status(provider HTTP status)/body/provenance; paged result uses declared items/cursor/complete. Base errors plus inherited idempotency_conflict/outcome_unknown for mutations. Provider-to-error mapping must not erase mutation unknown classifications (catalog 5xx mapping currently conflicts with the strengthened mutation table; sibling mutation-classification scope).
- New declaration/artifact fields sources/curation/realization/license and changes to bundle/index formats are independent of service-wire version and adapter-kind version. Reserved asyncapi provenance is not ingestion support; reserved OCI locations are not fetch support. `http-header`/`http-query` placements also need auth vocabulary disposition.
- Generic 256 KiB request and configurable default 30 s provider timeout exceed the fixed v1 read binding's 64 KiB request and 20 s service deadline. Matrix must either choose a separately negotiated binding/limit profile or revise the proposal; silently clipping to current defaults weakens the stated profile.

## Consolidated defects to resolve in E02

1. **P1 — No single complete disposition for closed additions.** Core optional descriptor/envelope/error extensions require an explicit new selected binding. Current capability and connection no-change claims, auth evidence's additive claim, and records/resource additive shorthand omit the strict-reader distinction. Sources: capability :104; connection :115; evidence :147; records :89; resource discovery :102; core :31/:61/:72/:93/:113.
2. **P1 — Mutation response still lacks a representable full classification/replay contract.** `effect: replayed` is a delivery marker, cannot replace original applied/refused/not_attempted/unknown, and must not imply success. New public errors, cause versus classification and observation-subject identity must be preserved for duplicate/refused observation cases. Source: operations :73, :91, :129, :163. The parent owns final envelope encoding.
3. **P1 — Descriptor profile/auth composition is inconsistent.** Catalog `profiles[]` + generic-http/mutation composition conflicts with required singular core `profile`; its `requires_auth` string conflicts with auth.profile's array. Select a normative representation and update examples/declarations. Sources: catalog :73/:97/:121; auth.profile :72; core :61.
4. **P1 — Session/media binding version and extension refusal are missing from service-only compatibility.** Establishment payload, public session errors, duplex controls, authenticated live authority and binary frames must each have an explicit selected binding disposition. Old unary success with a session-looking result is insufficient. Sources: sessions :53/:67/:71/:105; media :45/:51.
5. **P2 — Mediated route public/private boundary is contradictory.** Resolve parent-operation-surface wording against same-composition-only forward; choose propagation mapping for route_refused/route_unavailable without exposing a generic forward route. Sources: mediated_route :43/:55/:67.
6. **P2 — Generic HTTP limits cannot fit unchanged v1 semantics.** 256 KiB and 30 s must not be advertised over a fixed 64 KiB/20 s binding absent explicit profile/binding limits and refusal. Sources: catalog :145/:147; service :36; operations :147.
7. **P2 — Artifact/schema compatibility needs its own accounting.** Adapter-kind additions, catalog manifest/index formats and receiver configuration (`http.extra_headers`) are not solved by service wire v1alpha2. Strict readers need named unsupported-format/configuration refusal; do not rename adapter/v1 or /v2 to match service wire. Sources: auth.profile :109; catalog :174/:176; logs :83.
8. **P2 — Existing profile preservation versus proposed private hardening must be explicit.** The old configured/read_only/http-bearer paths can retain wire bytes while proposed generation validation changes admission behavior. New readers must retain the selected old semantics, or refuse an unsupported projection, rather than silently impose new validation obligations while calling it unchanged. Sources: connection :82/:115; evidence :93; custody :91; service :63.

## Verification requirements for the final matrix

- Cover all fifteen proposed v1alpha1 documents above plus the separately reviewed governed-service proposal; distinguish contracts/profiles, envelope fields/errors, operation payloads, authenticated transport messages, binary data, configuration/spec/artifact changes and private values.
- Old reader + extended descriptor/error/response → explicit refusal before dispatch, not optional-field tolerance or coerced base error. New reader + old profile → original selected behavior/limits; unsupported profile → no invoke, no downgrade, no resend.
- Complete original mutation outcome × replay table including refused current disclosure of an original attempt; session readiness/revocation and media terminal reasons must keep their separate semantic domains.
- List reserved vocabularies as unavailable and link their owning semantic stories; unfinished provider semantics must not acquire implementation approval through a compatibility matrix.
- In future codec conformance, use original wire bytes for unknown/duplicate members and unsupported enum/version/profile cases. This review is textual/type evidence only; it did not run future transport/runtime scenarios.
