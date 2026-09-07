---
format: aep.planning-md/1
id: story:hosted-catalog-endpoint-bindings
kind: story
status: active
title: Hosted catalog connections retain deployment-declared destinations
relations:
- informed_by: story:the-hosted-posture-connects-a-catalogued-provider
- informed_by: story:deployment-declared-destination-aperture
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: catalog/grafana.catalog.json
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: contracts/connector-connection/v0alpha2/README.md
- confidence: cited
  path: contracts/connector-connection/v0alpha2/bundle.json
- confidence: cited
  path: contracts/connector-connection/v0alpha2/connector-connection.schema.json
- confidence: cited
  path: contracts/connector-connection/v0alpha2/vectors.json
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/catalog/src/lib.rs
- confidence: cited
  path: crates/catalog/src/table.rs
- confidence: cited
  path: crates/catalog/tests/main/pack_table.rs
- confidence: cited
  path: crates/connector-secrets/README.md
- confidence: cited
  path: crates/connector-secrets/src/file.rs
- confidence: cited
  path: crates/connector-secrets/src/file/prepared.rs
- confidence: cited
  path: crates/connector-secrets/src/file/transaction_crash_tests.rs
- confidence: cited
  path: crates/connector-secrets/src/memory.rs
- confidence: cited
  path: crates/connector-secrets/src/transaction.rs
- confidence: cited
  path: crates/connector-secrets/tests/main/prepared_transactions.rs
- confidence: cited
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-cli/src/error.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-cli/tests/hosted_connect.rs
- confidence: cited
  path: crates/connectors-client/Cargo.toml
- confidence: cited
  path: crates/connectors-client/src/hosted_connect.rs
- confidence: cited
  path: crates/connectors-client/src/hosted_connect_tests.rs
- confidence: cited
  path: crates/connectors-client/src/identity.rs
- confidence: cited
  path: crates/connectors-client/src/lib.rs
- confidence: cited
  path: crates/connectors-client/src/model.rs
- confidence: cited
  path: crates/connectors-config/src/hosted.rs
- confidence: cited
  path: crates/connectors-config/src/hosted_catalog.rs
- confidence: cited
  path: crates/connectors-config/src/lib.rs
- confidence: cited
  path: crates/connectors-console/Cargo.lock
- confidence: cited
  path: crates/connectors-console/src/connect.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: cited
  path: crates/connectors-runtime/tests/shared_catalog_custody.rs
- confidence: cited
  path: crates/hosted-vault/src/prepared.rs
- confidence: cited
  path: crates/hosted-vault/src/prepared_acknowledgement_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_custody.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_dispatch_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_endpoint_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_endpoints.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_form.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_verification.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend_custody.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend_tests.rs
- confidence: cited
  path: crates/integration-jira/src/backend.rs
- confidence: cited
  path: crates/integration-jira/src/backend/auth.rs
- confidence: cited
  path: crates/integration-slack/src/backend.rs
- confidence: cited
  path: crates/integration-slack/src/backend/connection_runtime.rs
- confidence: cited
  path: crates/protocol/src/connection.rs
- confidence: cited
  path: crates/protocol/src/connection_v2.rs
- confidence: cited
  path: crates/protocol/src/connection_v2_schema.rs
- confidence: cited
  path: crates/protocol/tests/bundles.rs
- confidence: cited
  path: crates/protocol/tests/connect_session_status.rs
- confidence: cited
  path: crates/server/src/hosted/connect.rs
- confidence: cited
  path: crates/service/src/credential_commit.rs
- confidence: cited
  path: crates/service/src/credential_commit_tests.rs
- confidence: cited
  path: crates/service/src/lib.rs
- confidence: cited
  path: crates/service/src/runtime.rs
- confidence: cited
  path: docs/design/04-the-callers-contract.md
- confidence: cited
  path: docs/guides/administer-hosted-integrations.md
- confidence: cited
  path: providers/grafana.toml
revision: 51
---
## Outcome

A deployment may bind the catalogue's declared endpoint variables for a hosted provider, allowing the existing Connect Session credential form and declared verification operation to work for configurable origins. The caller supplies only credential bytes through normal custody and cannot change the destination. Each durable connection pins its endpoint bindings; deployment changes make incompatible old connections unavailable until reconnected and never redirect an old credential.

## Acceptance

- Unconfigured templated providers expose no acquisition profile. Invalid, unknown, non-HTTPS or undeclared endpoint configuration refuses startup.
- Configured provider acquisition shows its deployment-selected destination and verifies the submitted credential using the provider's declared operation through the ordinary catalog transport and custody paths.
- Stored endpoint bindings survive restart. Destination removal or change prevents spending an old credential, including when another provider admits the old host.
- Owner and tenant isolation, session capability validation, exact transport origin/path and denied caller endpoint overrides have executable refusal coverage.
- Public-network policy remains the default. Explicit operator network configuration admits only the resolved declared origin through the existing post-DNS transport policy. Fixed-origin providers and old stored connections remain compatible.

## Existing typed authority

Provider and Connection are already declared in ess/system/domains/catalog.yaml and ess/system/domains/connection.yaml. This adds deployment configuration and private connection metadata to those existing entities; no provider API or operation schema changes.

## Scope

- crates/connectors-config/src/hosted.rs, hosted_catalog.rs and lib.rs: deployment policy fields, structural validation and public configuration type.
- crates/integration-catalog/src/hosted.rs, hosted_endpoints.rs, hosted_endpoint_tests.rs and lib.rs: canonical endpoint validation, connection pinning, acquisition, transport selection and boundary tests.
- crates/connectors-runtime/src/composition.rs: exact origin and explicit network policy composition.
- docs/guides/administer-hosted-integrations.md: generic configuration and credential acquisition contract.

This is one bounded implementation alongside the consuming application's separate work. No decomposition or critic panel is needed for a single story.

## Validation so far

All twelve repository Cargo workspace gates pass on implementation commit 7e11cb919bd2746e8d3e04c1e74ede788409f147, including both default and no-default-features hosted-runtime configurations. The final gate passes: 65 providers and 70 artifacts verified, portable Markdown links, story index, ESS 0.18.0 specification validation and byte-identical committed clap projection. Focused final suites pass 136 tests; all-target clippy for connectors-config, integration-catalog and connectors-runtime passes with warnings denied. Formatting and diff whitespace checks pass.

The first broad run found the configuration module size fence; extracting hosted_catalog.rs restored the bound without a waiver. A later concurrent run hit an existing one-second OAuth fixture expiry during setup, which passed unchanged alone and with bounded test concurrency. A long temporary directory also exceeded the native completion transport's Unix socket path bound; the unchanged test passed with a short task-owned temporary root. The completed broad gate used RUST_TEST_THREADS=4, a short private TMPDIR and the repository-pinned ESS toolchain. Failed evidence is retained alongside successful runs.

New endpoint metadata is additive to the existing v1 state and omitted for default fixed-origin records. Backward loading and serialization are tested. A connection created for a newly configurable provider needs this updated runtime; an older runtime has no implementation for spending its endpoint-bound credential. No change to provider declarations, canonical request/response schemas, SQL storage or Secrets protocols is made.

Actual composed browser acceptance and the real provider credential remain the consuming deployment's next proof; this artifact remains active until that evidence arrives.

## Credential entry clarity

The actual hosted Grafana form still labels its single masked input only as Credential, which leads an SSO user to infer that the form asks for their browser password. The renderer must derive the label, help, authentication description and documentation link from the selected catalogue credential/config declaration. Grafana's declared service_account_token help must explicitly distinguish the API token from the SSO password and direct the owner to the Grafana service-account token page. Keep the generic renderer provider-neutral and preserve normal credential custody and endpoint pinning.

Official sources checked on 2026-09-07: https://grafana.com/docs/grafana/latest/administration/service-accounts/ and https://grafana.com/docs/grafana/latest/developer-resources/api-reference/http-api/authentication/. These describe service accounts separately from Grafana users and service account tokens as bearer credentials for the HTTP API. This is acquisition copy only; provider endpoint, operation and request/response schemas remain unchanged.

Acceptance: the real selected profile's form renders its matching credential label/help and official documentation; a second profile of the same provider renders its own copy; text is escaped and unsafe documentation links are omitted. Regenerate affected canonical artifacts and prove unchanged operation schemas, then run the required source gate. Actual deployment/credential changes remain with the consuming deployment.

## Credential form verification

The form now renders the exact selected credential's catalog label, help, canonical authentication description and safe HTTPS documentation link. Grafana asks for a Grafana service account token and explains that an SSO password is not an API token. The renderer contains no Grafana-specific selection rule. Tests also distinguish Anthropic's ordinary and administrative profiles and prove HTML escaping and unsafe-link refusal.

Validation passed on 2026-09-07: 110 integration-catalog tests, all-target clippy for integration-catalog and connectors-runtime, formatting, the complete scripts/gate.sh run across all twelve workspaces and both runtime configurations, and its final catalog/docs/ESS lane. Both catalog build/diff rounds reached the same fixed point; catalog check verified 65 providers and 70 artifacts. Comparing the Grafana canonical document with config removed proves all operations, auth, services and schemas unchanged. Only declared credential help/docs and the resulting pack/lock hashes changed.

The consuming deployment must still render the published form and complete its own real credential/read proof. This source change adds no admin credential requirement or programmatic provisioning endpoint to the generic catalog adapter.

## Owner-scoped programmatic acquisition

The owner-authorized hosted token flow can reuse Connection v1 CreateConnectSession and Status plus the existing one-use HTTP completion route. The CLI adds `connectors setup connect <provider> --target hosted --auth-profile <declared-profile> --credential-file <owner-only-file> --label <label>`, retaining local as the default. It uses the existing saved Identity session, requests only the ordinary self-connection and catalog scopes, and never requests administrative or cross-owner authority. Deployment automation with its own existing principal can use the reusable HostedClient completion method and unchanged APIs; this work does not invent workload login, create provider service accounts, or register shared administrative credentials.

Acceptance: owner-only regular non-symlink bounded credential input is read without output; returned session capabilities remain private and can be submitted only once to the selected Connector origin and exact session route with redirects refused; completion is reported only after owner-scoped Status and Describe agree on provider, profile and callable Connection. Refused/expired sessions, hostile destinations, redirects, provider verification failure, foreign Connection or profile, and unsafe files fail without secret/capability diagnostics. Multiple token providers exercise the same path. Existing personal setup, OAuth and operation routing retain their behavior. The selected provider's runtime Connect Session contract remains the authority for profile admission and custody.

Cited existing typed authority: ess/system/domains/connection.yaml ConnectSession and CreateConnectSession/FinishConnectSession; crates/protocol/src/connection.rs ConnectSessionStatus/Create request; crates/server/src/hosted/connect.rs one-use POST; crates/connectors-client/src/identity.rs scope selection; crates/connectors-cli/tests/cli_surface.rs treats setup connect as an existing multi-step Flow. No new wire entity, API schema or generated command path is introduced.

Scope adds the reusable client completion/workflow and tests, a thin CLI target branch and parsing tests, and generic setup documentation. This is one bounded story, so no parallel decomposition/critic panel is needed. The sub-agent run is non-interactive; no missing authority is treated as approval.

## Local TLS prerequisite

The consuming local deployment's public discovery route returns the valid contract with its retained CA, but the installed CLI fails Discovery because its reqwest build contains only bundled WebPKI roots. Enable native certificate roots on the client-owned reqwest dependency so the normal platform trust store and SSL_CERT_FILE can admit a private development CA while retaining full certificate and hostname verification. This is required to exercise the same hosted Identity/Connect Session path against the local TLS deployment. Preserve the user's saved login and use isolated state for verification; never add an insecure TLS flag.

## Programmatic acquisition boundary

The reusable protocol client remains independent of embedded provider declarations and accepts an already-authorized Identity bearer plus the ordinary OwnerContext. Its caller selects a declared single-secret token profile. Incompatible native flows retain their server refusal: Jira raw completion is Invalid; Slack OAuth raw completion is Refused; Slack companion input must parse exactly two tokens before any verification or custody commit. The CLI performs a conservative preflight in connectors-console, which already links the pinned catalogue: exact credential, ConnectSession acquisition, stated subject and exactly one matching secret config field. It refuses unknown, OAuth, Basic and native-only profiles before reading the credential file or acquiring Identity authority. Runtime deployment admission remains authoritative, and newer profiles require matching CLI declarations while protocol clients remain catalogue-version-independent.

## Programmatic acquisition verification

The complete scripts/gate.sh run passed on 2026-09-07: all twelve Cargo workspaces, both hosted runtime feature configurations, catalog verification (65 providers, 70 artifacts), documentation/story checks, ESS 0.18.0 validation and exact committed clap projection. The client boundary suite uses real TLS with a temporary CA and verifies native SSL_CERT_FILE trust in isolated child processes, rejection without that CA, exact origin/API-prefix/session route, ownership/permissions/type/size file refusal, redirect refusal, dropped-response no replay, malformed acknowledgement refusal and matching completed principal Connection status. Three CLI process tests pass, including unknown/OAuth/native-multifield profile refusal before Identity and file access. Formatting, whitespace checks and client/CLI all-target clippy pass.

The consuming deployment independently reports a successful fresh normal Identity CLI login through a headless browser, upstream fixture, Identity callback and CLI loopback, followed by normal scope renewal and a hosted Connection list. It used task-isolated local selection and normal keyring custody without changing the operator's existing hosted session. Private evidence is retained as hosted-cli-login-672379418819327/result.json and hosted-cli-read-1788792695405657932/connections.json. The first browser helper attempt failed for missing TMPDIR and the corrected environment passed; failed evidence remains available.

The first focused source fixture used an HTTP hosted completion URL, which the unchanged Connection protocol correctly refused. Fixtures were corrected to use TLS and required Connection initiation/actor fields rather than weakening validation. Real provider token acquisition and an actual Grafana read remain pending owner setup; no service-account token was accessed or minted, and source/boundary tests are not claimed as real-provider success. This artifact remains active for that consuming-deployment evidence.

The final all-target clippy run for connectors-console also passes with warnings denied. Its existing catalogue dependency owns CLI preflight; connectors-client gains only native-root TLS support plus test-only TLS fixture dependencies and retains no catalogue/runtime dependency.

## Hosted completion failure diagnosis

Real owner submission exposed a diagnostic gap: the browser renders every non-503 refusal as the same sentence, and catalog verification discards safe upstream status and transport failure classes. Read-only diagnosis confirms that the provisional connection reference satisfies egress authority syntax and the declared Grafana verification request is GET /api/datasources with a service-account bearer token. Official documentation requires datasources:read (datasources:* where RBAC applies): https://grafana.com/docs/grafana/latest/developer-resources/api-reference/http-api/api-legacy/data_source/. Network policy is independently owned by the consuming deployment; no deployment identifiers or credentials enter this source change.

Acceptance: preserve closed diagnostic classes around the existing single verification exchange without recording credential bytes, capability, provider body, URL or underlying error Display. After valid capability admission, HTTP completion gives fixed actionable authentication, permission, unreachable and other provider-failure codes. Operator diagnostics retain fixed transport classifications and upstream status only. Unknown and invalid capabilities retain their indistinguishable refusal. The generic form displays the existing session deadline, refuses expired submission locally and never automatically retries or allows uncertain completion to replay. Provider help states the permission required by its unchanged verification operation. Focused tests prove classification, no custody on failure, invalid capability no dispatch, deadline rendering and sanitized HTTP responses. Full source gates remain required; real credential completion remains a separate deployment acceptance proof.

Scope adds service HostedCompletionError vocabulary and re-export, hosted server response handling/tests, catalog verification observer and form helpers/tests, and existing provider-help canonical artifacts. Existing ConnectSession authority and expiry semantics remain unchanged; no new ownership, persistence, operation schema or egress authority is introduced.

## Custody failure stages

After deployment reachability was corrected, the owner received the existing generic HTTP503 completion. Preserve separate closed diagnostics for verification setup, transaction reservation state, prepared custody (including its payload-free PreparedSecretError class), pending metadata persistence, commit and final connection persistence. Commit/finalization failures are explicitly unconfirmed; the form directs the owner to check Connections and ask the operator rather than replaying a credential. This is diagnostic work, not an unreviewed persistence migration or mutation of existing custody state.

Read-only source finding: runtime composition shares one PreparedVaultStore journal among adapters whose generation counters are independently persisted and initially1. Shared reclaim retires every generation through its threshold. A later adapter can therefore receive PreparedSecretError::Retired before any Secrets request. Deployment value-free metadata must establish whether this occurred; no secret values or staged credential bytes may be read. Any correction to this generation-domain mismatch needs its own reviewed scope and recovery proof.

## Shared custody retirement correction

Deployment metadata confirms a shared prepared-store retirement watermark above a new catalog coordinator's next generation. Preserve the existing shared journal and current pending transaction identifiers. Add an opaque retirement watermark and exact transaction acknowledgement to PreparedSecretStore, implemented by MemoryStore, FileStore and PreparedVaultStore. Existing decoded terminal records remain unacknowledged. Atomic prepared-store admission and Staging insertion must share one lock; terminal retirement must never cross an unacknowledged transaction, including a different nonce at the same generation.

Each hosted catalog/GitLab/Slack/Jira coordinator reserves its next persisted generation at or above watermark+1. Retry only an explicitly no-effect Retired prepare, bounded and without provider reverification or resubmission. Busy, backend and uncertain commit outcomes are not retried. Replace broad generation reclaim with exact acknowledgement after final Connection metadata is durable. Keep the existing pending record with an additive default-false published receipt until acknowledgement succeeds, including across restart. Recovery of published receipts performs acknowledgement/receipt cleanup without credential publication or provider access; legacy pending records retain their existing recovery path.

Acceptance includes cross-coordinator same/different generation acknowledgement, committed-but-not-published recovery while another owner completes, published-before-ack interruption, legacy journal decode, retired-generation advance, prepare admission races and bounded no-effect retry. No journal migration, no lower retirement fence and no private state patch is used. Shared store implementation is delegated to the read-only review agent after its findings; this session remains the sole AEP writer and owns all coordinator/form/diagnostic changes. The independent store implementation and coordinator implementation use disjoint files in the retained managed tree and separate leases.

## Write-ahead custody intent

Persist each owner pending intent before prepared custody staging, including the exact candidate Connection and transaction id. Add a default-false intent marker so legacy prepared receipts retain conservative recovery, while a new intent fenced by a Retired refusal can be discarded without claiming a committed credential. A no-effect retry durably replaces only its prior exact intent; interruption between counter reservation and replacement remains recoverable. Recovery fences unknown absent ids before durably recording discard and exact acknowledgement, preventing a delayed prepare from reappearing after cleanup. Hosted and memory journals allow this tombstone alongside a peer Prepared transaction; whole-image file storage defers until pending preparation is resolved, then retries in a second pass. Tests cover interruption before prepare, after prepare, staging abort, retired retry reservation and bounded exhaustion, as well as failed metadata publication and peer acknowledgement.

## Shared custody and completion verification

The corrected candidate passes every required scripts/gate.sh lane: all twelve Cargo workspaces, both hosted runtime feature configurations, catalog verification (65 providers and 70 artifacts), portable documentation links, story index, ESS 0.18.0 validation and exact committed clap projection. Formatting passes across every workspace; root and hosted-runtime all-target clippy pass with warnings denied. The final GitLab custody-module extraction also passes the full native package tests and a repeated hosted-runtime clippy check.

Five composed acquisition/recovery tests exercise the real PreparedVaultStore over controlled state/value adapters and provider HTTP. They cover the deployed retirement-floor shape, independent coordinator final-metadata and receipt-cleanup failures, failed intent persistence, cancellation before prepare, during staging and after prepare, retired write-ahead recovery, and fourth-attempt exhaustion without a fifth attempt or provider replay. Eight shared helper tests additionally cover racing watermark advance, failed reservation after a Retired attempt, no retries of Busy/backend/uncertain results, maximum generation, absent-id fencing and real FileStore deferred recovery. Store tests cover same-generation peers, exact acknowledgement across reopen, atomic concurrent admission, uncertain acknowledgement and the process-crash matrix. Foreign owner scope remains refused.

The exact generic form JavaScript passes 18 real Chromium cases against controlled responses, including expiry, fixed failure messages, credential clearing, malformed/lost acknowledgement, redirect refusal and one-submit behavior. This is source boundary evidence, not a claim of successful provider acquisition. The consuming deployment separately proved its corrected network policy and a task-owned sentinel roundtrip through the real Secrets HTTP boundary; no owner credential was accessed or replayed by this work.

The first root gate caught existing source-size ceilings. Extracting FileStore crash tests and GitLab custody methods restored the bounds without raising a waiver. A following compile caught an incorrect extracted impl type name; it was corrected and the affected package/runtime checks passed. All failed logs remain retained. The complete gate was run in bounded workspace lanes, with exact completed test executables and inactive task-owned incremental cache reclaimed between lanes to preserve local cluster disk headroom; source, libraries, actual CLI and deployment state were retained.

The existing shared journal is loaded in place. Legacy outcomes remain unacknowledged; legacy pending receipts with lost Retired outcomes remain refused, never invented as successful commits. The additive write-ahead marker applies only to new intents. Mixed writers using the old broad-reclaim algorithm are unsupported; the consuming deployment has confirmed a single replica and Recreate upgrade strategy. Real owner Grafana completion and a governed read remain deployment acceptance, so this story remains active. No release, merge, deployment or private-state generation patch was performed here.

## Hosted operation lease continuity

The consuming deployment now reports successful owner Grafana credential acquisition, but separate describe and invoke requests immediately return stale_authority. The hosted adapter constructs a fresh CatalogBackend for every handle call, discarding that backend's description leases. The earlier endpoint test helper reused one private delegate and therefore did not exercise the actual public request boundary.

Retain a bounded delegate cache keyed by the existing stable principal authority seed and the current owner-filtered binding configuration, including grant, credential profile, endpoint and network policy. Recompute that configuration from current metadata and deployment policy on every request. Normal verified-token/request rotation preserves continuity; real principal/delegation/group or binding changes select a fresh lease map. No authority checks, credential custody or provider schemas change. Replace the endpoint test helper with separate HostedCatalogBackend handle calls and add focused renewal, wrong owner/tenant, changed authority/binding and cache-capacity regressions. Source gates remain required; do not access or replay the owner's credential or claim a live provider read from controlled tests.

## Pending status without capability replay

The consuming deployment also reproduces HTTP503 when polling a newly created pending Connect Session. The producer correctly omits its one-use completion capability after Create, but the validator applies the Create locator requirement to Status as well. The delegated protocol correction distinguishes acquisition responses from status-only responses, retaining the Create and pending remediation-start locator requirement while allowing pending status to omit it. Add an actual hosted producer-to-envelope regression. The existing v1 schema already admits this omission; update only v2 projections and generated schema/bundle/vector outputs that declare the stricter shared rule, using their established deterministic generation path. No caller capability is replayed and no completion authority expands.

The focused protocol gate exposed a predecessor source-byte pin for connection.rs. Apply one documented compatibility exception for correcting its existing status validator to the already-declared v1 response schema: update only that reader's reviewed hash in bundles.rs, retaining all other reader hashes and every v1 contract artifact byte. New tests keep Create locator requirements and malformed/terminal refusal explicit. This repairs the current hosted producer/reader mismatch without changing v1 wire shapes or authentication authority.

## Public request recovery verification

The corrected candidate passes the authoritative gate across all twelve Cargo workspaces, both hosted runtime feature configurations, catalog verification for 65 providers and 70 artifacts, portable links and story checks, ESS 0.18.0 validation and exact clap projection. Formatting passes across every workspace; root and hosted-runtime workspace all-target clippy pass with warnings denied. Final protocol tests include the reviewed predecessor source pin and the new create/status distinction. All 155 v2 vectors agree with their declared Rust-reader and Draft 2020-12 outcomes, and deterministic bundle generation matches the committed schema and manifest. Every v1 contract artifact remains byte-identical.

The full catalog suite passes 120 tests. Separate public HostedCatalogBackend describe and invoke requests now preserve their lease across verified scope-token, snapshot, request and trace rotation. Wrong owner, tenant, group or realm; changed delegated attempt, delegation, grant identity or grant revision; changed or removed binding; and delegate-cache eviction refuse the stale description before provider egress. Changed authority can obtain its own fresh description without using a foreign lease. A producer-to-envelope regression creates an actual hosted pending session and validates repeated status responses that omit completion capabilities; creation still supplies its route and malformed or terminal capabilities remain refused.

These use controlled value/state adapters and provider transport, not the operator's real provider account. The consuming deployment has reported durable credential acquisition, but the new pinned runtime and BFF still require local composition and a real governed provider read before acceptance is complete. No credential was read, replayed or requested by this upstream subtask, and no deployment or release was performed. The story remains active for that acceptance.

After verification, 102 exact completed test executables were reclaimed from the retained task target, excluding all top-level executable inodes and all libraries, source and evidence. This restored about 7.38 GiB for the consuming composition. The successful gate logs, earlier refusal evidence and exact cleanup inventory are retained privately with the task handoff.

## Declared legacy response schemas reach capability consumers

The consuming deployment now succeeds at a real governed provider read, but capability creation refuses the resulting description because output_schema is null. Source diagnosis confirms that the canonical legacy operation retains its effective response_schema while contract.output_schema is intentionally absent for legacy request semantics. The typed catalog reader reads only the latter. The generic description therefore drops an existing declared schema before the consuming platform sees it.

Read the existing canonical effective response_schema for legacy operations when no translated contract output is present. OpenAPI 3 operations continue to use only the compiler-translated contract output; absence remains absence where no supported output was declared. Preserve every declared constraint, including an array-valued response represented by a JSON object schema. Do not substitute an empty schema or JSON object response type, invent provider fields, change raw provider schemas, or reinterpret credential-producing wire payloads: canonical response_schema already represents the safe effective output.

Add catalog-wide byte/value-fidelity coverage between canonical legacy response declarations and typed operations, explicit precedence/absence coverage, and an actual public hosted Grafana description regression for the declared operations. The protocol DTO shape and catalog artifact bytes stay unchanged; legacy public description content changes from null to its existing schema. No provider request, credential, grant or invocation authority changes. Full source gates and parent-owned real profile/agent/provider acceptance remain required. No real provider credentials are accessed or replayed by the upstream implementation.

## Declared response projection verification

The typed reader now preserves the existing canonical effective response schema for legacy operations while giving translated contract output precedence. Catalog-wide fidelity coverage compares every legacy typed output with its canonical declaration. This restores 853 already-declared legacy response schemas; 154 legacy operations with no output declaration remain unspecified. The four OpenAPI 3 operations retain their existing three translated outputs and one unspecified output. Explicit fixture coverage preserves boolean schemas and absence, and prevents a literal OpenAPI source response from substituting for a missing translated contract. Canonical catalog documents, pack bytes, generated schemas and wire contract fields are unchanged.

A public hosted regression connects a sentinel through controlled adapters and describes all four declared Grafana operations. Their input and output schemas are JSON objects with exact canonical output equality. The datasource-list output keeps type array and its declared item properties and required fields; it is not coerced into an object response or an unconstrained schema. Description requests perform no additional provider exchange. The catalog package passes 27 tests/doctests, and the complete integration-catalog suite passes 121 tests.

All twelve authoritative workspace gate lanes pass, including both hosted runtime feature configurations, final catalog verification for 65 providers and 70 artifacts, documentation links, story checks, ESS 0.18.0 validation and exact clap generation. All-workspace formatting and root/runtime all-target clippy pass with warnings denied. The root gate ran before a clippy-only equivalent Option fallback correction; the affected catalog tests and root clippy were repeated afterward, and all subsequent lanes ran on the final implementation.

Failed evidence remains retained: clippy first requested an eager Option fallback, which was corrected without changing behavior. The first parallel runtime lane also hit four existing one-second OAuth fixture authorization/expiry failures. Those four tests passed unchanged in isolation, then both complete runtime configurations passed with serial test execution. Production deadlines and test assertions were not changed. Future constrained local runs should keep this timing evidence visible and use the bounded serial gate setting rather than treating a retry as unexplained success.

This is source-boundary evidence. The consuming deployment separately reported a real governed provider read before this change, but the corrected description still requires actual capability-profile compilation and an agent tool invocation after composition. No real credential was read, replayed or requested here. The generic catalog connector_audit_ref is a deterministic operation/connection digest, not a durable per-execution audit receipt, and is not used as proof of a provider call. The story remains active for parent-owned real acceptance.
