# 21 — Personal OAuth callback and credential custody

Status: proposed implementation decision, 2026-09-06. This document and its ESS value patch are a handoff, not an implemented or accepted runtime claim. Source inspected at `84ccd0247438dee8d7e731db80fd903b74137ad7`, including the integrated one-shot operation work.

## Decision

Implement catalog-driven personal acquisition first for GitLab: explicitly selected public authorization-code with PKCE S256 on a development-only IPv4 loopback redirect, or explicitly selected public device authorization. Reuse the existing Connection identity and ConnectSession lifecycle. A provider admission states what is supported; deployment configuration selects exactly one admitted flow for the credential purpose. There is no automatic flow change, secret-paste fallback, or redirect substitution.

Use an explicitly selected, dedicated OAuth `FileStore` through the existing `PreparedSecretStore` transaction contract. Open `state_root/oauth/credentials.store` once and pass that same instance for prepared writes and OAuth credential reads. This first custody option is `DevelopmentFile`: durable and atomic, unsealed at rest, and refused for a production deployment selection. Ordinary catalog credentials retain their existing keyring selection. No OAuth token bundle may use `MemoryStore`, point-write emulation of prepare, or an ambient alternative store.

This is the smallest implementation supported by the current store interfaces. `KeyringStore` implements point `SecretStore` operations through Secret Service; it does not implement prepared batches or value-free enumeration/recovery. A prepared keyring implementation would require a separately designed crash-safe publication and recovery mechanism. Design 07's personal keyring and production custody expectations remain in force: this proposal does not establish production-ready OAuth custody. Device authorization can be provider-valid for production while this initial local storage option still prevents a production readiness claim.

## Provider evidence and admission

The following is a documentation measurement on 2026-09-06, not a live registration or provider interoperability test. Auth-flow endpoints remain platform acquisition declarations under the repository's source-fidelity exception; ordinary provider operations continue through the official API source/import pipeline.

| Provider | Measured registration/flow facts | Admission in this implementation |
|---|---|---|
| GitLab | Documents public PKCE, development HTTP redirects, production HTTPS redirects, and public device authorization. Detailed device history says generally available in 17.9; the introductory version summary differs. Its device success example does not guarantee a refresh token. | Public PKCE with `LoopbackIpv4Http`, `DevelopmentOnly`; public device authorization for a deployment whose GitLab supports it. Never downgrade to another flow after a refusal. |
| Slack | PKCE requires an explicitly public application. The documented desktop redirect exception uses HTTP `localhost` and excludes bot scopes. Ordinary installation uses HTTPS redirects. | No new admission. Numeric `127.0.0.1` compatibility and the repository's existing user-token endpoint combination have not been established by these pages. Existing Slack acquisition remains separate. |
| Jira Cloud | The 3LO guide requires client-secret exchange and an application-configured callback. These sources do not establish public PKCE, device authorization, or this numeric loopback exception. | No new admission. Unestablished support is recorded as unknown, not a claim that Atlassian rejects every such flow. |

Sources: [GitLab OAuth API](https://docs.gitlab.com/api/oauth2/), [Slack PKCE](https://docs.slack.dev/authentication/using-pkce/), [Slack installation](https://docs.slack.dev/authentication/installing-with-oauth/), [Jira Cloud 3LO](https://developer.atlassian.com/cloud/jira/platform/oauth-2-3lo-apps/). The general device polling and optional complete verification URI rules come from [RFC 8628](https://www.rfc-editor.org/rfc/rfc8628.html).

## Typed declaration and deployment boundary

The accompanying patch proposes value types in the existing catalog, deployment, and connection domains. It does not add a durable session entity or settle the held multiple-credentials-per-placement work.

Add optional `personal_flows` to the existing OAuth acquisition declaration. Each unique flow admission carries:

- `flow`: authorization-code PKCE or device authorization;
- `client_authentication`: public or client-secret-post, explicitly per flow;
- optional redirect shape and device authorization service/path, present only for the corresponding flow;
- permitted registration use, refresh policy, and an authenticated token-evidence service/path with declared JSON pointers and scope encoding.

The initial runtime admits only the public GitLab combinations. The vocabulary can describe other facts without enabling them. A provider's `ProductionAllowed` admits either deployment use; `DevelopmentOnly` admits only development use. Custody imposes a separate restriction, so this initial FileStore configuration always selects development use. Keep the existing global `public_client` meaning and legacy redirect defaults unchanged. New personal acquisition requires a matching per-flow declaration and never interprets absence through the legacy boolean. Add the device grant to the existing closed grant vocabulary, using the exact device-code grant identifier at token exchange.

GitLab declares authorization and token paths already owned by its OAuth block, device authorization at `/oauth/authorize_device`, and evidence at `/oauth/token/info`, all resolved against the declared GitLab service. Evidence uses GET with the newly acquired Bearer token, `/scope` as a string array, `/resource_owner_id` as the subject observation, and `/application/uid` as the deployment-client check. Observe this evidence for both flows and refresh, avoiding a policy that guesses scope from the authorization request or depends on a scope field missing from one documented token-response example. Record only the bounded, validated evidence, its origin and generation. A missing/malformed required observation or client mismatch refuses publication.

Deployment owns the actual client ID, optional client-secret reference, selected flow, exact redirect URI when applicable, known browser placement, registration use, bounded session TTL, allowed scopes, approved service origin and explicit custody option. The first public flows reject a client secret. Confidential remains a representable declaration requiring a matching implementation before readiness can advertise it. Client registration values never enter the compiled catalog. `ConnectSessionCreateRequest.auth_profile` continues to select the existing credential purpose; it is not repurposed as an OAuth flow name.

Choose a 300-second default session TTL with a deployment range of 30–600 seconds; these are local design bounds, not provider promises. Use the smaller of the session deadline and device authorization expiry. A PKCE session requires `SameMachine`, a fixed nonzero configured port and non-root configured callback path on `http://127.0.0.1`. Configuration rejects userinfo, query, fragment, hostname aliases and implicit/default ports. The development registration URI must exactly match the deployment value. `doctor` prints that derivable URI and the actual unsealed OAuth custody status before any session starts. Production selection with `DevelopmentFile` refuses readiness; it does not silently soften the posture.

`OtherMachine` permits the explicitly selected device flow. Loopback selection with that placement refuses before opening a listener. If no supported flow and human handoff channel are configured, setup exits with a bounded, named refusal. Registration itself remains operator work; the implementation neither creates an OAuth application nor requests operator secrets through a model.

## Acquisition and trusted instructions

The generic catalog acquisition owner resolves configuration, admission, owner, existing Connection grant, destination policy and custody before reserving a session. It uses `ConnectSessionLifecycle` and the shared `connector-oauth` state/PKCE/token primitives. The current raw-credential `BoundCompletionEndpoint` remains distinct; its `/complete` handler must never accept OAuth callbacks or token bundles.

Add a session-owned OAuth instruction/callback helper in `connect-session-transport`. Preserve the existing strict `browser_completion_url` shape: `http://127.0.0.1:<port>/#token=<capability>`. Its root is a protected instruction page, not the provider authorize URL. Capability material stays in the fragment, is sent only to the exact local helper by a protected request, and is never sent as a provider query parameter or Referer. Apply no-store, restrictive CSP and no-referrer policy. No local helper trusts forwarded headers, wildcard CORS, redirects to arbitrary origins, or a query-supplied target.

For PKCE, the fixed registered callback path and instruction page share the bound port. Creation binds before presenting any instruction; port conflict returns `port_in_use`, with no ephemeral-port fallback. Generate an independent one-use OAuth state and private zeroizing PKCE verifier. The trusted page directs the human to the declared provider authorization URL. The state and session capability are different values. The token exchange occurs only through the existing egress owner, with the exact client ID, redirect URI and verifier from that pending session.

For device authorization, no OAuth callback listener is created. A short-lived local instruction endpoint may still carry the existing completion-URL capability. The acquisition owner obtains a device code and polls the declared token endpoint through egress. The private device code never leaves that owner. Human instructions carry the verification URI and user code, or a validated optional complete verification URI. Accept only HTTPS on the admitted provider origin; do not invent a complete URI when the provider omits it. Enforce provider polling interval, the RFC default where omitted, `slow_down`, bounded backoff, denial/expiry termination, the session deadline and cancellation. Never reissue the device authorization request as a silent retry that creates a second authorization.

The trusted `LocalClient` can fetch the capability-protected instructions and poll the existing session status. Extend its internal pending representation to distinguish raw credential entry from OAuth instructions; preserve existing callers. The outer `connectors setup connect` writes human instructions to a controlling terminal. For a headless terminal owner, accept an explicitly selected owner-only instruction file and clear it at terminal completion/cancellation. Refuse before session creation if neither handoff is available. Do not print URLs, codes, verifier, token values or full callback errors to JSON/stdout/stderr that a model tool can capture. The model/inner-harness projection retains only its allowed opaque references, status, deadline and safe refusal classification. Trusted control-plane URL access does not authorize forwarding that URL to a model.

## Callback validation and terminal ownership

Bind numeric IPv4 loopback only, require a loopback peer, and compare the raw Host authority to the exact configured `127.0.0.1:<port>`. Require the exact callback path and GET. Reject absolute-form targets, duplicate or malformed parameters and `code`/`error` ambiguity. Use bounded parsing: at most 8 KiB headers, 4 KiB query, 2 KiB authorization code, and a 5-second per-request read deadline capped by session expiry. Enforce bounded accepted connections and request concurrency. These are chosen local limits; an oversized legitimate response produces a safe refusal, not unbounded allocation.

Top-level browser navigation may omit Origin. If present, accept only the exact configured provider authorization origin; reject `null` and mismatches. Local instruction requests use their own exact local-origin/capability policy. Do not describe Origin as a substitute for OAuth state, PKCE or Host validation.

Invalid state never consumes another pending state's capability. A matching live state is taken once before any awaited exchange, and the callback listener closes immediately after that claim. Repeated callbacks cannot trigger another exchange. A verifier/challenge inconsistency detected locally is `pkce_binding_mismatch`; a provider `invalid_grant` is only `code_exchange_refused`, because it does not prove which remote check failed. Host, Origin, state and malformed-request refusals have distinct internal safe classifications, with no reflected query/body data. The public/model projection preserves the existing authorization-opacity rules.

Session expiry, explicit failure and shutdown cancel device work, pending network work and listener tasks; clear the private pending table and all instruction capabilities. Terminal `ConnectSessionLifecycle` status clears both completion endpoints. No callback port survives the session's authorization deadline. All token/device response fields are size-bounded before use; any present refresh token is bounded even under an `IfIssued` policy. Accept the standardized Bearer token type without relying on the case of a documentation example. Require finite validated access-token expiry. GitLab PKCE may require refresh issuance; device flow uses `IfIssued`, and an access-only result remains valid until expiry, then degrades and requires explicit reauthorization.

## Atomic custody, expiry and recovery

Extract a small value-free custody transaction owner from the existing catalog hosted transaction machinery and use the existing `PreparedSecretStore` prepare/commit/abort/status contract. Share transaction primitives; do not blindly copy the hosted session's current commit-before-final-pending-check ordering. Session admission and credential durability need one explicit decision point.

Persist an owner-only, value-free transaction record before prepare: transaction ID and digest, existing binding/Connection identity, authority and credential generation, observation evidence, and phase. This record is an internal journal entry for the existing Connection; it is not a durable ConnectSession. Session state, code, verifier, device code, access/refresh tokens and provider URLs with capabilities are excluded. Preparing first without recording a recoverable transaction ID would leave an undiscoverable prepared batch after a crash.

Prepare one atomic secret batch containing access and, when issued, refresh credentials at reserved addresses derived from the same existing binding. Absent refresh explicitly removes any superseded refresh value; no stale value can survive reauthorization by omission. Keep the store choice fixed in this binding's metadata. Persist generation and observed scopes alongside the value-free transaction receipt; no operation may see a new credential with old evidence.

Serialize each binding's transaction and session completion claim. After prepare, recheck the same pending session, grant/authority, generation and trusted current time under the completion owner. Claim completion once with an internally captured `authorized_at` strictly before the deadline; expiry or revocation before this claim wins, aborting the batch without credential/Connection completion. The claim moves the private owner to Committing. Persist a durable commit decision carrying that captured instant and deadline before committing credentials. I/O may finish after expiry: the deadline is the authorization cutoff at the serialized claim, not a promise that filesystem synchronization finishes by then. The expiry worker cannot mark a claimed completion Expired. No caller supplies or backdates the instant. An uncertain decision write remains recovery-required; never report a false Completed or Expired merely to end the wait.

The value-free journal must use a durability mode that preserves its committed decision at least as strongly as the prepared credential store preserves its secret commit. The existing SQLite WAL `synchronous=NORMAL` default is insufficient for this ordering. Add an explicit FULL-durability open mode for this custody journal while retaining existing callers' behavior. Recovery without a durable commit decision aborts the prepared batch; recovery with a valid decision and `authorized_at` before its recorded deadline finishes idempotently. Missing or invalid decision timing refuses publication. Test a held decision write across expiry, uncertain writes and restart recovery, as well as expiry before the claim.

Connection v1 requires Pending to carry a live completion endpoint. After instructions retire, internal Committing or RecoveryRequired therefore projects the existing safe Unavailable refusal while reconciliation continues; it does not invent an endpoint or an additional wire state. Trusted polling may re-read bounded session status without repeating provider acquisition or an operation. Completed appears only after coherent publication.

Recovery aborts a preparing transaction with no commit decision, completes a decided transaction, and reclaims its receipt only after coherent publication. Unknown or inconsistent store status fails closed. Crash recovery does not revive the old session capability; a coherent recovered Connection is visible through its normal identity. Test every crash boundary, including prepare before decision, decision before secret commit, secret commit before metadata publication, and publication before receipt reclamation. Terminal state/instruction removal and interrupted client polling must not cause duplicate commit.

Only after token and evidence publication does the OAuth binding delegate operation resolution/execution to `CatalogBackend::bind_stored`, using the dedicated OAuth store. Keep those bindings out of the ordinary backend's credential-import set; registry claims must reject duplicate owners. Each OAuth binding carries one selected credential purpose. Operation admission checks that same generation's proven scopes and existing grant; it must never union evidence from other credentials. Reauthorization and on-demand refresh preserve Connection identity and custody selection. Refresh uses a per-binding double-checked lock and the same transaction/evidence path. Remote token rotation can invalidate the old pair before a local failure: mark such uncertainty degraded/recovery-required and require explicit repair when necessary; a local rollback is not proof the old remote token still works.

Refresh is bounded and on demand before an admitted operation; it adds no mandatory background service to one-shot execution. Session creation still requires the established persistent runtime. A one-shot invocation can use a coherent stored credential or return a safe refusal; it does not start a long-lived authorization session implicitly.

## Stable Connection and future remediation seam

A configured, admitted stable Connection with missing credentials can remain Created and require authorization. A genuinely nonexistent or unadmitted Connection retains the ordinary opaque refusal. Missing IDs never imply authority to create a placement or expand a grant.

The implementation can expose an internal readiness classification on an already resolved catalog Binding: usable, authorization required, reauthorization required, or unavailable. Its input must already carry the resolved owner/Connection grant; its result contains no URL, session capability or registration material. Trusted outer control can use that classification to start and poll the existing ConnectSession against the same binding. A later auth-result story may define its public projection. This proposal reserves no wire field, error variant, endpoint or automatic retry behavior for that later story.

## Compatibility, ownership and verification

Keep `connector-connection/v0alpha1` schemas, vectors, `auth_profile` meaning and the existing completion-URL validator. The local protected instruction bridge avoids widening the validator to arbitrary provider URLs. Regression tests must prove trusted URL delivery and model redaction independently.

The canonical OAuth acquisition meaning does change. Allocate canonical schema 4 after the source-fidelity work's planned schema 3; do not reuse 3 for two different meanings or claim the proposed source-fidelity implementation has already landed. Update the canonical writer and reader version fence together, and prove the old reader refuses the new pack before serving a record. Regenerate every canonical document and the deterministic pack/index outputs through the existing compiler; no handwritten generated JSON repair. Provider loader/schema and typed catalog accessor changes must preserve strict unknown-field and per-flow validation. No ordinary operation request/response schema is rewritten by this OAuth change.

The scope report names exact existing and proposed files. Responsibilities are: connector-spec/catalog-build/catalog-reader own declarations and canonical interpretation; connectors-config owns deployment selection; connector-oauth owns pure state/PKCE/token/device mechanics; connect-session-transport owns local HTTP lifetime and parsing; integration-catalog owns acquisition, custody coordination and exact binding; runtime owns store composition/readiness and registration; client/console own trusted human handoff. No new provider adapter, keyring transaction backend, public callback server or held multi-credential implementation is included.

Required later verification includes deterministic schema/pack regeneration and old-reader refusal; invalid declaration/config matrices; public/no-secret exchanges through mock egress; callback Host/Origin/path/state/replay/bounds/expiry races; device pending/slowdown/denial/expiry/cancellation; no-refresh expiry; token-info scope evidence and generation admission; crash/restart transaction outcomes; fixed store routing and unchanged ordinary keyring behavior; headless handoff and model redaction; existing raw ConnectSessions; and one-shot use of acquired/expired credentials. Use synthetic values and local mocks, not live operator accounts. Run the repository's required gates in the implementation tree. This read-only design assignment ran only ESS validation/compilation over complete scratch copies.

UNMAPPED in the ESS patch means an explicit enforcement obligation: flow-dependent fields and measured vendor compatibility; URI/Host/Origin and parsing bounds; private secret representation; atomic expiry/commit/recovery; generation-scoped admission; terminal cancellation; trusted-output separation; production custody refusal; and canonical-version migration. The value model proves none of those runtime properties. No external product choice blocks drafting this development-scoped implementation. Production-capable personal OAuth custody remains a separate decision and implementation prerequisite for production admission.

## Integration decision — 2026-09-06

The coordinator adopted this explicit development-scoped implementation shape for the approved story after the complete ESS model validated. GitLab source-fidelity schema3 and reviewed one-shot source are now integrated ate0974691. Rate metadata completes that same unpublished schema3 migration first; OAuth acquisition uses a subsequent schema4 and never edits frozen schema2 or an already published schema3. Atlas migration authority must record the schema4 relying parties and rollout before its canonical writer changes.

Stage1 owns only the pure OAuth/device/token helpers and the isolated local instruction/callback transport. Catalog declaration, runtime/acquisition/custody/client integration and generated schema/pack output remain held until the rate unit hands those shared owners back. Stage1 is not an operational OAuth completion claim.

## Existing Connection v1 target limit

The unchanged Connection v1 create request names an integration and optional auth profile, but no
target Connection. Before creating a session, callback listener or provider request, the authenticated
owner, integration and admitted profile must identify exactly one configured stable Binding. An
omitted profile uses a deployment-owned default only where that selection is unambiguous. Zero or
multiple matching bindings refuse. Labels remain display text; profiles cannot select another
instance or broaden authority.

Capture that exact Binding/Connection identity immutably when the session is created and recheck it
at completion. Reauthorization under this contract repairs only that uniquely selected binding;
completion-time lookup must never substitute a different target. The subsequent auth-result story
requires a separate Connection version for caller-selected, bound remediation. This story neither
reserves those fields nor imports the held multiple-credential implementation.

## On-demand refresh authorization window — 2026-09-06

Use one receiver-owned 30-second authorization window for an admitted refresh. Capture its start
and deadline before any refresh request, after taking the per-binding gate and rechecking the
current operation grant and credential generation. The existing invocation and backend context
carry no admitted outer deadline (`crates/protocol/src/operation/legacy.rs`,
`crates/service/src/runtime.rs`, and `crates/connectors-runtime/src/registry.rs`); the egress
timeout applies separately to each exchange. This fixed window is a local implementation choice.
Do not add a caller timestamp or infer a deadline from a description reference.

The receiver installs the trusted clock. Capture Unix milliseconds with checked addition of
30,000 and a matching monotonic deadline. Token exchange and token-info share the remaining
monotonic budget; neither they nor waiting for prepare may reset it. Refuse invalid clocks,
overflow, wall-clock rollback before the captured start, and elapsed monotonic expiry. Do not
cap this window by the old access-token expiry: an expired access token can require refresh.

After prepare, the same serialized authority claim rechecks the immutable binding, current
grant, client, origin and generation and requires newly observed evidence valid at the claim.
The internally sampled authorization instant must satisfy start <= authorized_at < deadline,
with monotonic time still remaining. A one-use private refresh handle shares the existing FULL
decision, secret commit, publication and recovery machinery. It creates no ConnectSession or
instruction endpoint. Preserve the original session completion API and its deciding tests.

Once the timely claim wins, filesystem I/O may finish after the deadline. Do not cancel the
entire commit future at the earlier egress cutoff or invent a terminal session result. An
uncertain write remains unavailable until recovery; a durable valid decision finishes, while
Preparing without that decision aborts. Remote token rotation followed by local uncertainty
does not prove the old remote credential remains usable. A per-binding generation recheck
makes concurrent operations share one completed refresh and never resends an operation.

Use a separate custody refresh test module to preserve the existing custody cases. Required
cases cover the original deadline across exchange/token-info/prepare, expiry and revocation
before the claim, a timely decision held past expiry, uncertain writes and real store reopening,
clock failures, cancellation, and concurrent operation callers. These are implementation
obligations; this decision records no executed refresh or operational OAuth result.

## Durable marker before refresh egress — 2026-09-06

A provider can rotate a refresh credential before token-info succeeds or local credential
prepare begins. The old custody publication alone cannot distinguish that uncertainty after
restart. Before any refresh egress, under the same binding gate and after opening the real
refresh authorization window, confirm a private marker through the existing FULL SQLite store.
An uncertain marker write permits no provider request.

Use the key `oauth.refresh.v1.<digest>`, where `digest` is the 64-character lowercase hexadecimal
SHA256 of the canonical custody Identity. The closed JSON value contains only `version: 1`,
`connection_ref`, `binding_sha256` as 32 octets, and `previous_generation`. The digest binds the
existing owner, integration, Connection, purpose, fixed store and credential-address digest.
No token, code, provider response, grant, caller time, endpoint URL or attestation belongs in it.
This value has a typed home as `connectors.deployment.OAuthRefreshAttempt`; bounded decoding,
digest construction, exact version and generation checks remain runtime obligations.

Recover custody first. Clear the marker only after confirming a publication with a generation
greater than `previous_generation` for the exact same binding, with evidence that passes the
current authority, client, origin and scope ceiling. A durably decided custody transaction can
recover into that publication. Preparing without a decision may abort local work but cannot
clear the marker or claim that remote rotation rolled back. Until that confirmation, refuse reuse
of the old credential across store reopening. Explicit reauthorization of the same binding may
establish the newer publication; it must satisfy the same checks.

Unknown marker reads or deletion remain unavailable. Keep one shared FileStore instance and the
existing FULL SQLite path; this is neither another credential store nor a custody journal image
version change. Actual pre-egress failure, post-rotation failure, cancellation and reopen tests
must establish these rules before operational OAuth completion is claimed.

## Trusted setup entry — 2026-09-06

Use `connectors setup connect <provider> --auth-profile <declared purpose>` to request the
configured personal OAuth flow. Optional `--instruction-file <owner-only path>` selects private
instruction delivery to a file. Without that file, acquire a controlling terminal before creating
a session, listener or provider request. Machine-readable output never carries the private human
instruction. The frontend must select the already configured unique OAuth binding and refuse
enrollment flags that would change it; a label remains display text. The existing raw `--as`
enrollment flow remains available where no personal OAuth flow is selected.

The current ESS component records `setup connect` as an unspecified guided flow
(`ess/system/components.yaml:170`). Its command projection has no representation for these options
on a forwarded multi-step flow. Preserve that explicit limitation; the real Rust Clap parser and
console tests own these arguments rather than a fabricated domain command or generated handler.
