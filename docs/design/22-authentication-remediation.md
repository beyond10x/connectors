# 22 — Authentication remediation for an admitted Connection

Status: proposed implementation decision, 2026-09-06. No implementation, runtime completion, protocol acceptance or consumer delivery is claimed. Slot 22 was free in the inspected design directory.

This decision is grounded in integration `66dbede1876036260a0287871ef3ae8e1c0fb53d`, designs 01/02/13/21, the current grant/registry/session owners, and the rate prerequisite's committed `fe5da04ab9ea0a71fc1a75194a87cddd88f46cb2`. Rate stage2 has working edits; those are not a stable implementation baseline. OAuth runtime integration is also pending.

## Current acceptance proposal

Keep the original acceptance and context in `story:auth-as-tool-result` and `docs/stories/S-014-auth-as-tool-result.md` unchanged as history. Append the following scoped acceptance and update the current title to “An admitted Connection has a structured authentication next step” through the coordinator's planning transaction:

- An authenticated invocation which passes the exact Connection/operation grant and reaches credential preflight before any operation dispatch can return a typed `authentication_required` response under ConnectorOperation v0alpha3. It names the already admitted operation, Connection, derived integration and credential purpose, distinguishes initial authorization from reauthorization, and states `not_attempted` and the trusted next action. It contains no URL, session capability, provider code, registration or credential material.
- A configured stable Created binding with no callable description uses an explicit trusted ConnectorConnection v0alpha2 remediation start. The receiver derives admission metadata from that exact configured binding without advertising it as a callable member, evaluates the current operation grant, and creates a real single-purpose session only after management/self-service authority also permits it. Neither route creates a nonexistent placement or uses a label/profile to select another binding.
- Unknown and unadmitted identities receive the same ordinary opaque refusal. An absent grant store is an outage. Neither the existing hosted read-path fallback nor a generic `Unavailable`, socket failure, channel reconnect state, provider 403 or ambiguous execution becomes authentication authority.
- Trusted outer CLI/control plane can receive the existing protected human completion URL or instruction projection, poll the bound session, and acknowledge completion once. Every request revalidates the current principal, exact target and applicable grant; expired, failed, consumed and mismatched sessions cannot yield a resume acknowledgement. Model/MCP output remains an explicit safe allowlist with no URL or capability.
- Initial authorization and reauthorization retain the configured Connection reference. Completion for another integration/Connection is refused before credential publication where the owner can observe it, and is also rejected at acknowledgement. A session cannot add scopes, select credentials, change routes or replace a grant.
- After acknowledgement the caller reads a fresh operation description, validates its intended input against that description, and explicitly submits a new invocation. The server never saves or resends the invocation. `OutcomeUnknown`, timeout after dispatch, protocol mismatch and rate advice never trigger this loop automatically.
- A still-valid personal-local identity or Connectors-audience Identity token completes the loop without a new Identity login. Token rotation with the same admitted principal/realm is supported; expiry, revocation, scope reduction, owner changes and changed policy are re-evaluated, not bypassed by a session.
- Positive and adversarial fixtures cover both new protocol identities, exact downgrade loss, wrong versions, grant opacity, Created versus absent, credential versus transport failure, one-use completion, authority rotation/expiry, trusted/model output and zero dispatch before explicit resumption. Provider and affected in-repository client/server gates pass before a runtime completion claim.

These changes resolve two conflicts in the legacy acceptance: grants do not authorize creation of a missing Connection, and ConnectSession URLs are not model tool results (`docs/design/01-domain-model.md:88`, `:193`, `:289`; `crates/domain/src/grant.rs:131`).

## Decision and protocol identities

Use two explicit version changes, with different audiences:

| Boundary | Proposed change | Preserved boundary |
|---|---|---|
| ConnectorOperation | `b10x.connector-operation.v0alpha3` adds a typed pre-dispatch authentication error carrying only safe facts. | Rate v0alpha2 and the complete v0alpha1 bundle remain frozen. No auth field is reserved in rate work. |
| ConnectorConnection | `b10x.connector-connection.v0alpha2` adds bound remediation start/status/acknowledgement commands and a trusted response wrapping existing ConnectSessionStatus. | v0alpha1 request/status meanings and bundle bytes remain unchanged. |

The second change is necessary: current `ConnectSessionCreateRequest` has only `integration_ref`, `label`, and optional `auth_profile` (`crates/protocol/src/connection.rs:94`). It cannot express the particular Connection being repaired. Encoding that target in a label, profile, integration string or URL would create undocumented authority.

Operation v3 requests retain the v2 request vocabulary. Extend its closed `OperationErrorCode` with `AuthenticationRequired` and add optional typed `authentication` to `OperationError`. It is required exactly when that code is present and forbidden otherwise. The payload is:

```text
AuthenticationRequired {
  operation_ref, connection_ref, integration_ref, auth_profile,
  need: authorize_configured | reauthorize_existing,
  attempt: not_attempted,
  next_action: start_trusted_remediation
}
```

The envelope has `status:error`, no result, and `retriable:false`; no delay accompanies this error. It is a distinct protocol error, not a successful InvocationResult or an HTTP 401. The authenticated hosted route returns the typed refusal as HTTP 409; local framing uses the same envelope. Identity failure remains 401, unadmitted authority remains the existing 403-class refusal, and authority/transport outages keep their existing meanings. The CLI exits nonzero while preserving the structured error. Rate v2 fields retain their exact semantics in v3.

The model-safe error does not itself create a ConnectSession. Session creation is a separate, explicit trusted action. This avoids leaking a session capability, opening a callback from an ordinary model invocation, consuming approval, or starting a long-lived workflow inside one-shot execution.

Connection v2 includes the existing commands and adds:

```text
remediation_start { operation_ref, connection_ref, input }
remediation_status { connect_session_ref }
remediation_acknowledge { connect_session_ref, operation_ref, connection_ref }
```

Start accepts the intended operation input only as a bounded transient value, with the existing 64 KiB operation-input limit. Set the new Connection v2 frame ceiling to 128 KiB so the bounded input plus envelope fits; retain v1's 64 KiB frame ceiling. Existing response and reference bounds remain. No caller-supplied integration, scope set, credential, auth profile, label, route, authority or callback URL is accepted by these commands. The receiver derives integration/profile from the admitted target.

Start/status return a typed `BoundRemediationStatus` containing the derived operation/Connection/integration/profile, need and resume state, plus the existing ConnectSessionStatus projection. Only this trusted response may carry its pending `browser_completion_url` or owner-only endpoint. The existing URL validator still applies. Terminal projections contain no endpoint. Acknowledgement returns the session reference, operation, Connection and `fresh_description_then_explicit_invoke`; it carries no invocation input, description lease or execution authority.

`resume_state` is Pending, Ready, Consumed, Expired or Failed. It is a projection of the existing ConnectSession lifecycle, current expiry and an in-memory acknowledgement bit. It adds neither a durable session entity nor a ConnectSession lifecycle transition. Status polling is idempotent; successful acknowledgement is one-use. An authorized caller receives a typed Conflict for an unusable or mismatched acknowledgement and uses status for the state; it does not parse message text. Unknown/unadmitted session references remain neutral refusals.

## Grant and credential ordering

The present hosted route re-describes and then calls `HostedAuthority::admit_invoke`, which combines grant evaluation with approval redemption (`crates/server/src/hosted.rs:356`; `hosted/enforcement.rs:149`). The personal registry also spends an `event:` claim before entering the integration (`crates/connectors-runtime/src/registry.rs:278`). Inserting remediation after those calls would consume a one-use authorization without attempting the operation.

Introduce a non-consuming admission preflight, then keep execution admission separate:

1. Validate exact protocol, bounds and request structure before state/backend work. Authenticate the existing local owner or hosted Connectors-audience principal. Apply the current operation receiver policy and scopes.
2. Resolve the exact configured operation/Connection owner, using value-free metadata. Unknown, wrong-provider, revoked or unadmitted references receive the same opaque refusal. Resolve no secret and create no session at this stage.
3. Validate caller input and current non-consuming grant decision against the exact binding and catalog facts. Ordinary invoke must also pass its existing description-freshness check before an authentication response is emitted; the explicit management start uses its separately admitted internal metadata path. Hosted remediation requires a real grant decision; `InvokeAdmission::ReadPath` is not sufficient. A missing grant store stays unavailable. Personal remediation uses the configured binding's actual admitted policy and authority snapshot, never an invented hosted Grant revision.
4. Only now ask that exact owner's typed credential readiness. MissingCredential or CredentialDegraded may produce the safe authentication outcome if a bound acquisition implementation is available. DependencyUnavailable and Unsupported do not. Ready continues toward normal invocation.
5. For an actual invocation, recheck description/authority freshness and all normal admission/approval rules at their existing execution boundary, redeem required approval/event evidence, and dispatch once. If readiness changes after this point, preserve the normal refusal/ambiguity outcome; never retroactively claim `not_attempted` or launch remediation after an uncertain dispatch.

The safe outcome is reachable only while the service can prove the operation has not crossed its dispatch boundary. It must not spend approval or an event-reply claim. Hosted grant preflight and execution admission share evaluation facts; preflight is not a serialized or reusable `AdmittedOperation` proof. Decisions expire and are checked against real time again at use. Existing backend dispatch remains reachable only through its normal grant/approval seam.

Readiness is specific to the selected credential generation. A provider-confirmed expiry/revocation or missing required credential can be remediable. Failed DNS, TLS, socket, keyring, store, OAuth provider or egress availability are outages. Existing Slack ConnectionState::Degraded can mean a stopped/reconnecting channel (`crates/integration-slack/src/backend/connection_runtime.rs:35`), so generic ConnectionState alone cannot trigger reauthorization. A 403 is not evidence of token expiry. Missing scope evidence remains non-callability under design 09; the implementation does not turn it into permission to request broader scopes.

Defaults are fail closed: a backend without the new readiness/target-binding methods reports Unsupported and starts no session. The first operational adapter is design21's generic personal OAuth binding. Existing Slack/GitLab adapters may participate only after they implement and test the same exact-target semantics; they are not inferred capable from a provider name or a generic error string.

## Created bindings and the description boundary

Design 01 requires missing/stale capability evidence to hide callable members (`:273–283`). Keep that rule. A public description is not fabricated merely to get through grant evaluation, and a Created binding is not included as an invocable Connection on a callable operation description.

For ordinary invoke, a valid description obtained earlier can reach preflight after the credential becomes unavailable, if its normal freshness rules still admit it. If the description is stale, return StaleAuthority rather than bypassing that rule. For a configured Created binding with no usable description, the trusted `remediation_start` command provides the separate path. Its internal metadata lookup reads static catalog facts and the configured target, evaluates the current grant for the intended bounded input, and returns only the bound session. It does not return a callable description or perform an operation.

Reuse the resolved catalog operation document and actual `ConnectionAuthority` as internal metadata. For this management-only grant evaluation, bind `catalog_generation` to the actual catalog generation and `description_ref` to a receiver-computed, domain-separated `remediation-metadata:<sha256>` digest of that operation and configured binding. This is deliberately not a public description lease. The grant decision is checked for current expiry and retained only as bounded provenance; it is never converted into an `AdmittedOperation` or accepted at invocation dispatch. Status/acknowledgement resolve current metadata and re-evaluate using the saved input digest; the present GrantRequest already accepts that digest, so raw input need not survive start. A metadata/schema generation change invalidates the old remediation binding and requires a fresh explicit start. No persisted Grant shape or selector semantics change.

This distinction avoids a circular requirement that the user possess a callable credential before being allowed to authorize that same configured Connection. It also avoids changing the public operation-discovery policy. Genuinely absent Connections continue to require the existing explicit deployment/candidate activation work, with its own authority, outside this story.

## Binding the session and finishing the loop

The owning acquisition backend captures a private `RemediationBinding` beside the existing ConnectSession record: exact operation/Connection/integration/purpose, need, canonical input digest, stable authenticated owner/realm digest, admitting grant reference and optional real revision, actual policy digest, and deadline. The policy digest includes the catalog/metadata generation and configured binding used at start. Store no raw input, token, provider code, browser URL capability or caller bearer in this binding value. Grant revision is absent for an unrevisioned personal policy; do not manufacture it from an Agent revision. Re-evaluate that policy on later requests.

Use the existing server-side canonical input digest implementation. Keep start inputs only while validating them and computing the digest; bound them by 64 KiB. The outer caller may retain its own bounded input in memory or reopen a user-selected input file. Connectors creates no durable invocation queue or automatic replay record.

Require the same currently authenticated principal/actor/tenant/optional realm on start, status and acknowledgement, plus current operation grant and existing acquisition management policy. Token IDs and request IDs are not stable principal identity; use the existing stable authority seed rules so normal Identity rotation works. A session is not an Identity credential, and possession of its reference is not a grant. A current policy change is re-evaluated rather than accepted because an old digest matched. Expired Identity authority must be renewed by the existing Identity mechanism; no second login is required when still-valid authority suffices.

Management admission is additional to operation admission: keep `connectors.connections.self` plus a supported self-service profile, or `connectors.connections.manage` plus operator policy, as appropriate. Existing hosted `connectors.catalog.read` remains required for status. A grant to invoke through a tenant-shared Connection does not grant permission to replace its credentials. Unsupported or unauthorized repair produces no session/URL; the trusted client reports the ordinary refusal.

Select one integration and credential purpose from the existing binding; do not accept an `allowed_integrations` set from the client. Capture the target before provider interaction and recheck it before credential publication. Design21's transaction and expiry linearization still owns publication. A changed owner, revoked authority, wrong integration, wrong Connection, or stale binding cannot retarget the session. A successful reauthorization preserves the Connection ID and route.

The start command creates one fresh session, bounded by design21's TTL and any applicable authority expiry. A repeated start for the same authenticated binding/operation/input digest may return its existing Pending session; a conflicting request for that active binding refuses rather than silently replacing it. Limit the remediation map to the existing pending-session capacity plus a bounded terminal/tombstone allowance of the same size. Prune expired records; never allow unlimited completed-session accumulation. All data is attached to the existing ephemeral session owner. Restart forgets these session capabilities; durable Connections remain ordinary grant-controlled state.

Status reports Ready only after the existing session is Completed with the captured Connection and that Connection has current callable credential evidence. Polling defaults to one second, stops at the returned deadline, and obeys caller cancellation; it does not poll provider OAuth endpoints itself. Acknowledgement verifies current owner/grant, matching requested operation/Connection, Ready and deadline, then atomically marks the acknowledgement consumed. Two simultaneous acknowledgements yield one success. The marker stays until expiry so a repeated acknowledgement cannot act as another resume token.

The session's deadline bounds acknowledgement too. If credential publication won before expiry but the client acknowledges after expiry, acknowledgement refuses without rolling back the now-authorized Connection. The user can still make a separate fresh grant-admitted invocation; that is not resumption by an expired session. A consumed/expired session never authorizes anything. Lost acknowledgement responses are resolved by status; they cause no automatic operation replay.

After a successful acknowledgement, the trusted client refreshes Connection and operation descriptions, confirms the same Connection and current schema, and waits for the explicit invocation command. The new invoke carries its current description lease and is admitted again. If the user changes the input, it is a new operation request with its own admission; no old digest or session substitutes for it.

## Trusted human and model outputs

Extend `connectors setup connect` with an explicit bound mode selected by `--operation`, `--connection` and its input source. Require these flags together; omit provider/profile/label target overrides in this mode. It starts/polls/acknowledges and then reports readiness. It never re-runs `operations invoke`. Model-safe operation errors may name this trusted action, but must not synthesize an executable shell command containing unescaped user arguments.

The trusted client uses design21's controlling-terminal or explicitly selected owner-only instruction-file policy for the human URL/device instructions. JSON/stdout/stderr and MCP retain only safe typed facts. Do not send the trusted Connection response through the generic `reduce_envelope!` macro, which serializes an arbitrary successful payload (`crates/connectors-console/src/envelope.rs:25`). Add a dedicated typed projection which cannot contain endpoints. Also update the operation-error reducer to preserve v3 authentication facts and existing v2 rate facts without string matching.

Hosted MCP continues through `operation_decided`, with the same grant and preflight owners; its toolset exposes no remediation start/status/ack command and no Connection URL. Its operation error conversion uses an explicit AuthenticationRequired allowlist. A trusted product can use the management route separately with its own current authority. SDK transport types and model-output types are separate; serializing a trusted response directly into a model result is a test failure.

One-shot operations may return the safe structured need and shut down normally. They do not reserve a session, bind a listener or retain inputs. Bound setup requires a persistent runtime; without one the trusted client returns the established daemon-required refusal before session creation. Authentication repair does not weaken one-shot ownership or shutdown rules.

## OAuth v1 prerequisite limit

Story21 can retain Connection v1 only where authenticated owner plus existing integration/profile/default resolve exactly one configured stable binding. Zero or multiple matches refuse before session creation or provider interaction; defaults are deployment-owned and unambiguous. Label remains display-only. The session captures that target internally, and completion cannot choose a different binding. Thus v1 can safely authorize or repair that uniquely selected binding, but cannot promise arbitrary caller-selected Connection repair.

Connection v2's explicit bound commands are the later target-selection contract. They derive purpose from the receiver's resolved Connection instead of trusting a caller profile. Stage1 OAuth helpers are unaffected; stage2 must enforce the v1 ambiguity limit and hand the exact binding/readiness methods to this story. The held multi-credential branch is not imported. The accompanying `oauth-prerequisite-note.md` gives this narrow handoff separately.

## Version adapters and coordinated reliance

Keep complete Operation v1/v2 and Connection v1 bundles unchanged. Serve all selected identities through explicit version adapters and one underlying admission path. v3 authentication errors projected to Operation v2/v1 become ordinary `Unavailable`, `retriable:false`, with a neutral bounded message and no authentication payload or URL. This deliberate loss is not structured auth support. Other v3 rate fields project according to the established rate adapter. Never decode a new identity by first forcing it through an older reader that rejects its new fields.

Connection v1 has no bound-remediation commands: reject attempts to downgrade those commands before creating a session; never translate them into an unbound ConnectSessionCreate. Ordinary Connection v2 commands can call the existing v1 semantic handlers and project their responses losslessly. Unknown identities fail validation before authority lookup, session allocation or dispatch. The server replies in the requested supported identity. No automatic version fallback/resend occurs, including for invoke.

Upgrade the shipped provider daemon, in-repository client and CLI together; the operation client default then becomes v3 and trusted bound setup explicitly uses Connection v2. External callers may remain on existing versions. An explicit old-version selection loses remediation and does not auto-negotiate. Conformance documents must distinguish this deliberate downgrade from a fully supported new client.

Atlas ADR0040 is proposed and records only a proposed rate-v2 migration; its item 5 preserves Connection identity and item 6 excludes speculative repair fields. Before implementing these changed protocol bytes, the coordinator must append an auth-specific dated reliance/order section to `architecture/adr/0040-connector-operation-rate-limit-version.md`, naming Operation v3 and Connection v2 as the subsequent migration and explicitly limiting those two existing exclusions to the rate unit. Preserve the rate decision and its historical scope. This design supplies the concrete content for that extension, not evidence it exists or has been accepted.

The inspected ADR0040 relying-party inventory is a starting point, not a fresh pin audit: Service SDK implements backend types; Devcenter exhaustively maps operation errors; Workspace uses the hosted client; Agent Platform binds operation identity into toolset digests; Org Brain consumes CLI JSON; Zwirn/legacy Platform retain strict old clients; Website follows docs source locking. Preserve their old-version service. Service SDK alignment must precede embedded Devcenter adoption of new Rust types. Workspace and Agent Platform may adopt afterwards with their own projection/digest fixtures. Org Brain verifies that authentication-needed never triggers its retry path. Zwirn/legacy clients need no change to retain old behavior. Website has no runtime-wire role.

No consumer-repository edit is included in this wave. Actual external adoption requires exact pin/source inspection, recorded ownership and affected consumer gates; observed inventory alone does not authorize such edits. Do not retire old versions in this story. The coordinator owns Atlas recording and publication.

## Implementation stages and deciding tests

Keep the rate-v2 backend request/result/error vocabulary as the internal execution port. Put the new Operation wire error and strict schema in separate `operation/v3.rs` and `operation/schema_v3.rs` modules, with a new version dispatcher in `operation/versions.rs`. A typed service preflight returns the model's `AuthenticationRequired`; transports lift it into v3 or deliberately downgrade it for older callers. Ordinary backend results lift from v2 without adding an authentication field to every backend error constructor. Put Connection v2 DTOs and adapters in `connection_v2.rs` and its schema module; preserve the v1 module and bundles. New backend metadata/readiness/bound-remediation methods default to Unsupported and create no state. The registry owns exact routing; the session owner owns binding/acknowledgement; hosted enforcement owns real grant evaluation; transports own identity selection and output projection. `scope-paths.json` names each owner and its exact files.

1. Record this current acceptance, model, exact scope and Atlas migration proposal. Pure type/validator and bounded-session helper design can be reviewed now. No stage is a runtime completion claim.
2. Finish and integrate rate v2 so its exact types, reducers, transports and schema ownership are stable. Finish OAuth stage2's exact binding/readiness/custody/client handoff, including its v1 ambiguity refusal. These are prerequisites for shared-source implementation, not assumptions that working files have landed.
3. In a managed implementation tree, add new version modules, bundle generators/vectors and explicit adapters; then the private non-consuming admission and session-binding helpers. A disconnected helper file may be developed independently only with an exact test seam owned by that unit; wiring registry/service/protocol/client files waits for the prerequisite owners.
4. Wire generic personal OAuth, trusted CLI/client and local/hosted/MCP projections. Backends default Unsupported until exact-target readiness is implemented. Run protocol, runtime, client, console, CLI and hosted/local tests and the repository's required full gates; obtain the bounded adversarial passes before publication.

Deciding cases must prove: unknown/unadmitted byte-identical refusals with no session/provider work; Created binding through trusted metadata preflight while callable discovery stays hidden; missing/confirmed-degraded versus outage/channel-down/403; grant refusal and missing store; no approval/event claim consumption on auth need; expiry/revocation between preflight and execution; ambiguous dispatch never becomes NotAttempted; exact integration/profile/Connection binding; v1 ambiguity refusal; concurrent start/ack races, expired/consumed/wrong-target sessions; terminal endpoint removal; authority rotation and expiry; changed input/schema/description on explicit resume; one-shot no listener; zero operation dispatch before resume and exactly one after; model/CLI output without URL/code/capability; and old/new protocol conformance with no automatic resend.

The ESS proposal adds ten value types and changes no entity/lifecycle/command/event. UNMAPPED markers identify actual enforcement obligations: grant and management admission, absence of an enumeration oracle, input/time/capacity bounds, no dispatch or approval consumption, conditional error fields and exact version downgrade, secret/output separation, principal rotation, binding/expiry races and acknowledgement linearization. ESS validation does not prove these runtime properties. No unresolved routine choice prevents implementation of this bounded design after its named prerequisites and migration record.

## Published migration proposal and protocol handoff — 2026-09-06

Atlas draft PR23 publishes commit `bfb731377a448ab89443c44852d622601f6c1363`. Its
`architecture/adr/0041-connector-operation-rate-limit-version.md` contains the dated subsequent
authentication section, and `architecture/adr/0044-connector-authentication-remediation.md`
records Operation v3 and Connection v2 with relying parties and movement order. The old ADR0040
paragraphs above retain their historical snapshot; current Atlas main allocated0040 to the ESS
complete-selection decision. The rate exclusions apply only to the rate unit. The published
extension and separate auth decision provide the previously required migration record, while all
four Connector decisions remain proposed. Draft publication establishes no architecture acceptance.

The complete pin audit at that same Atlas commit inspected exact advertised consumer mains at
12:49:24 UTC. It distinguishes Devcenter's hosted and excluded embedded locks, Agent Platform's
protocol-bound toolset digest, Org Brain's unknown deployed executable and Zwirn's actual legacy
namespace/audience pair. Five inspected Connectors revisions use b10x v1 and one uses the legacy
namespace; none establishes rate-v2 or auth-v3 adoption. Preserve complete old-version service,
SDK-before-embedded-host ordering and separate consumer gates. No external consumer edit is assigned.

Reviewed rate source `0c026610e67f3ec6da0a8014813f3d88c4e72799` is retained in the complete schema3
publication candidate `0c69450921ab1794c81dadec915b717a61bf0983`: all twelve workspace gates and
strict checks pass. That supplies the exact v2 freeze for one independent additive protocol slice:
new strict DTOs, explicit pure adapters, schema generators and conformance bundles. It keeps the
internal `operation::wire` v2 API and every existing frozen bundle/reader/generator byte unchanged.

## Current migration proposal numbers — 2026-09-06

Atlas draft PR23 now publishes `b95b84c8fdae634447e57bd8eafea5871f8ce46f`, based on
accepted main `b89e5b835b384965818eccf0703c8a548ebdbe47`. Main allocated ADR0041 to
shared Eventlog persistence. The four Connector proposals consequently use these numbers:

- ADR0042: Operation v0alpha2 rate limits, including its dated subsequent-authentication section.
- ADR0043: source-fidelity catalog schema3.
- ADR0044: personal OAuth catalog schema4.
- ADR0045: Operation v0alpha3 and Connection v0alpha2 authentication remediation.

The current authentication migration is
`architecture/adr/0045-connector-authentication-remediation.md`; its prerequisite rate decision is
`architecture/adr/0042-connector-operation-rate-limit-version.md`. Earlier numbered references in
this design describe their explicitly pinned historical proposals. Contract versions, relying-party
order and proposed status are unchanged. Publishing the reconciled draft grants no architecture
acceptance, external consumer adoption or release, and schedules no Eventlog conversion here.
Original-byte decoding must retain duplicate-key refusal; bound Connection commands have a typed
refusal when projected to v1. Pure types and vectors cannot establish grant, custody, session or
acknowledgement behavior.

## Assembled implementation checkpoint — 2026-09-06

Source `df637dc00bd47ffc4f300c89d33b0853c913e257` assembles runtime/server
`1156626a57bc3149bfd40264965d784be0138499`, client/console/CLI
`d7e7a673600c868078659d6412a1f6ccb7aefca8`, and hosted documentation
`9bac5f2f1466bff45404b146965b215fc91acf98`, retaining the frozen protocol/service
and reviewed OAuth parents. Complete assembled gates and the whole-authentication review are
pending at this checkpoint; these source observations do not claim protocol acceptance or external
consumer adoption.

The CLI selects Operation v3 by default and offers explicit operation-only
`--protocol-version v2|v3`. Each path sends the selected identity once through its actual socket,
hosted or one-shot API. The retained v2 rate tests explicitly select v2 and retain their original
response and assertion bytes. Connection remediation independently selects v2. Identity 401 renewal
retains its existing behavior; an admitted authentication-required 409 causes no renewal or resend.

Personal remediation uses the configured grant reference, an absent revision for the current
unrevisioned policy, and the actual policy/current-authority owner. The additive service admission
factory defaults to Unsupported. Its status-only authority check defaults to the existing strict
check; the personal owner can distinguish current policy admission from an expired session without
ignoring an opaque receiver rejection. Start and FULL custody publication remain strict. Authorized
expired status and unusable acknowledgement do not roll back already published credentials.

Production acquisition is personal-local. Hosted v2 admission requires the real operation grant and
management authority, then returns Unsupported where no acquisition adapter exists. The explicit
bearer Connection v2 transport is available, but no authenticated hosted bound-acquisition helper
invents a combined Identity scope. Bound CLI setup requires a persistent daemon, uses the protected
human instruction destination, acknowledges once, validates fresh matching descriptions and input,
and returns closed readiness facts for a separate explicit invocation. Public CLI and MCP
authentication output omits arbitrary daemon references and messages.

The deciding tests exercise actual Unix daemon/registry/OAuth composition, current-policy and
receiver revocation, expiry, concurrent acknowledgement, private presentation, strict version
selection, hosted grant/approval ordering and one-shot shutdown. The local completion case observes
zero operation dispatch through remediation and exactly one operation request after a separate
explicit v3 invocation. The served OpenAPI tests cover all retained versions and all 369 existing
Operation v2/v3 and Connection v2 schema vectors. Original product failures, missing-API diagnostics
and corrected new-fixture failures remain distinct evidence. ESS retains the same value types and
lifecycle structure; Rust admission, expiry and output safety remain executable obligations rather
than guarantees supplied by those field types.

OAuth's actual runtime and trusted-client handoff remains unfinished. Its shared owners, service
preflight, session binding, transports and default-client switch stay behind that completed source
handoff. The independent protocol slice acquires no runtime files or credential/session authority;
the coordinator records its exact paths, tests and build slot before implementation.

## Local execution correction checkpoint — 2026-09-06

The protocol-only handoff note preceding this checkpoint is historical. Reviewed OAuth source and the complete runtime, client, transport and protocol source are now assembled; executable validation and whole-authentication review determine the remaining delivery state.

Optional ordinary authentication preflight first requires an exact remediation route owner. Backends with no such route continue through their existing ordinary dispatch, grants and credential checks. A claimed ambiguous owner or actual metadata refusal remains an error. Bound Connection Start always performs its existing strict admission. The shared admission path and deliberate neutral v1/v2 downgrade remain unchanged.

The local one-shot owner now preserves a receiver-owned daemon requirement as an in-process outcome. Validated persistent session controls produce it before adapter composition; unsupported ephemeral invocation produces it after composition but before dispatch, then joins shutdown. Existing envelope-only APIs strip that local marker to the same correlated wire response. Neither received error text nor a provider outage can create the marker. The trusted CLI can display static persistent-daemon guidance while ordinary v3 errors retain their closed public projection.

Source 7e0611718556207a0a15d83be0b25fcd10846c4a combines runtime correction 4403f01796b50cfd16eb40431fcf01c79ec98407 and CLI correction 498d3618c542fd53b3a0c0de1ec6797160e6c04b. Corrected full runtime suites each pass 455 cases under default and no-default configurations, server passes 115, console 107, and CLI 140. Strict affected checks and the final repository catalog, Markdown, story and ESS checks pass. The initial full-root run's two failures are closed by complete affected server and catalog-cli rechecks; its original output remains evidence. These local results precede the first whole-authentication review and the complete sharded CI gate.
