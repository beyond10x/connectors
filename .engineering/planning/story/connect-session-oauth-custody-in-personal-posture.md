---
format: aep.planning-md/1
id: story:connect-session-oauth-custody-in-personal-posture
kind: story
status: active
title: Decide the connect-session ↔ OAuth-callback custody chain in personal posture
tags:
- ready
- wave-cli
refs:
- provider: legacy
  reference: S-013
relations:
- derived_from: epic:carried-constraints
scope:
- confidence: cited
  path: .gitleaksignore
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: catalog/airtable.catalog.json
- confidence: inferred
  path: catalog/alertmanager.catalog.json
- confidence: inferred
  path: catalog/algolia.catalog.json
- confidence: inferred
  path: catalog/anthropic.catalog.json
- confidence: inferred
  path: catalog/argocd.catalog.json
- confidence: inferred
  path: catalog/asana.catalog.json
- confidence: inferred
  path: catalog/asterisk.catalog.json
- confidence: inferred
  path: catalog/b10x.catalog.json
- confidence: inferred
  path: catalog/babelforce.catalog.json
- confidence: inferred
  path: catalog/bitbucket.catalog.json
- confidence: inferred
  path: catalog/box.catalog.json
- confidence: inferred
  path: catalog/calendly.catalog.json
- confidence: inferred
  path: catalog/claude-code.catalog.json
- confidence: inferred
  path: catalog/clickup.catalog.json
- confidence: inferred
  path: catalog/cloudflare.catalog.json
- confidence: inferred
  path: catalog/confluence.catalog.json
- confidence: inferred
  path: catalog/connector-document-v4.schema.json
- confidence: inferred
  path: catalog/contentful.catalog.json
- confidence: inferred
  path: catalog/datadog.catalog.json
- confidence: inferred
  path: catalog/discord.catalog.json
- confidence: inferred
  path: catalog/docusign.catalog.json
- confidence: inferred
  path: catalog/dropbox.catalog.json
- confidence: inferred
  path: catalog/figma.catalog.json
- confidence: inferred
  path: catalog/fly.catalog.json
- confidence: inferred
  path: catalog/freshdesk.catalog.json
- confidence: inferred
  path: catalog/front.catalog.json
- confidence: inferred
  path: catalog/github.catalog.json
- confidence: inferred
  path: catalog/gitlab.catalog.json
- confidence: inferred
  path: catalog/google.catalog.json
- confidence: inferred
  path: catalog/grafana.catalog.json
- confidence: inferred
  path: catalog/hubspot.catalog.json
- confidence: inferred
  path: catalog/intercom.catalog.json
- confidence: inferred
  path: catalog/jira.catalog.json
- confidence: inferred
  path: catalog/klaviyo.catalog.json
- confidence: inferred
  path: catalog/launchdarkly.catalog.json
- confidence: inferred
  path: catalog/loki.catalog.json
- confidence: inferred
  path: catalog/mailchimp.catalog.json
- confidence: inferred
  path: catalog/microsoft_graph.catalog.json
- confidence: inferred
  path: catalog/miro.catalog.json
- confidence: inferred
  path: catalog/mysql.catalog.json
- confidence: inferred
  path: catalog/newrelic.catalog.json
- confidence: inferred
  path: catalog/notion.catalog.json
- confidence: inferred
  path: catalog/okta.catalog.json
- confidence: inferred
  path: catalog/openai.catalog.json
- confidence: inferred
  path: catalog/openrouter.catalog.json
- confidence: inferred
  path: catalog/pagerduty.catalog.json
- confidence: inferred
  path: catalog/postgresql.catalog.json
- confidence: inferred
  path: catalog/postmark.catalog.json
- confidence: inferred
  path: catalog/prometheus.catalog.json
- confidence: inferred
  path: catalog/resend.catalog.json
- confidence: inferred
  path: catalog/runpod.catalog.json
- confidence: inferred
  path: catalog/salesforce.catalog.json
- confidence: inferred
  path: catalog/sendgrid.catalog.json
- confidence: inferred
  path: catalog/sentry.catalog.json
- confidence: inferred
  path: catalog/shopify.catalog.json
- confidence: inferred
  path: catalog/slack.catalog.json
- confidence: inferred
  path: catalog/statuspage.catalog.json
- confidence: inferred
  path: catalog/stripe.catalog.json
- confidence: inferred
  path: catalog/supabase.catalog.json
- confidence: inferred
  path: catalog/trello.catalog.json
- confidence: inferred
  path: catalog/twilio.catalog.json
- confidence: inferred
  path: catalog/typeform.catalog.json
- confidence: inferred
  path: catalog/vercel.catalog.json
- confidence: inferred
  path: catalog/webflow.catalog.json
- confidence: inferred
  path: catalog/zendesk.catalog.json
- confidence: inferred
  path: catalog/zoom.catalog.json
- confidence: inferred
  path: connectors.lock
- confidence: cited
  path: crates/catalog-build/src/check.rs
- confidence: inferred
  path: crates/catalog-build/src/document.rs
- confidence: inferred
  path: crates/catalog-build/src/document_schema.rs
- confidence: inferred
  path: crates/catalog-build/src/document_tests.rs
- confidence: cited
  path: crates/catalog-build/src/pipeline.rs
- confidence: inferred
  path: crates/catalog-build/src/workspace.rs
- confidence: inferred
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: cited
  path: crates/catalog-build/tests/main/ess_claim_fence.rs
- confidence: cited
  path: crates/catalog-build/tests/main/no_network.rs
- confidence: inferred
  path: crates/catalog-reader/catalog.pack
- confidence: inferred
  path: crates/catalog-reader/src/lib.rs
- confidence: inferred
  path: crates/catalog-reader/tests/main/pack.rs
- confidence: inferred
  path: crates/catalog/src/lib.rs
- confidence: inferred
  path: crates/catalog/src/table.rs
- confidence: inferred
  path: crates/catalog/tests/main/consumer_api.rs
- confidence: inferred
  path: crates/connect-session-transport/Cargo.toml
- confidence: inferred
  path: crates/connect-session-transport/src/lib.rs
- confidence: inferred
  path: crates/connect-session-transport/src/oauth.rs
- confidence: inferred
  path: crates/connect-session-transport/src/oauth_tests.rs
- confidence: inferred
  path: crates/connector-oauth/src/device.rs
- confidence: inferred
  path: crates/connector-oauth/src/lib.rs
- confidence: inferred
  path: crates/connector-oauth/src/token.rs
- confidence: cited
  path: crates/connector-resolve/src/document.rs
- confidence: cited
  path: crates/connector-resolve/src/resolve.rs
- confidence: cited
  path: crates/connector-resolve/tests/adversary_gitlab_pass1.rs
- confidence: inferred
  path: crates/connector-spec/schema/provider-toml.schema.json
- confidence: inferred
  path: crates/connector-spec/src/auth.rs
- confidence: cited
  path: crates/connector-spec/src/lib.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/auth_validation.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/schema_sync.rs
- confidence: inferred
  path: crates/connector-spec/tests/main.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/ir_roundtrip.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/oauth2_acquisition.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/oauth_token_endpoint.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/personal_oauth.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/provider_schema.rs
- confidence: inferred
  path: crates/connectors-cli/Cargo.lock
- confidence: inferred
  path: crates/connectors-cli/src/lib.rs
- confidence: inferred
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-cli/tests/one_shot_operations.rs
- confidence: inferred
  path: crates/connectors-client/src/completion.rs
- confidence: inferred
  path: crates/connectors-client/src/lib.rs
- confidence: inferred
  path: crates/connectors-client/src/model.rs
- confidence: inferred
  path: crates/connectors-client/src/personal_oauth.rs
- confidence: inferred
  path: crates/connectors-client/tests/personal_oauth_adversary.rs
- confidence: inferred
  path: crates/connectors-config/examples/gitlab-personal-oauth.example.toml
- confidence: inferred
  path: crates/connectors-config/src/lib.rs
- confidence: inferred
  path: crates/connectors-config/src/personal.rs
- confidence: inferred
  path: crates/connectors-config/src/personal_oauth.rs
- confidence: inferred
  path: crates/connectors-config/src/personal_oauth_tests.rs
- confidence: inferred
  path: crates/connectors-console/Cargo.lock
- confidence: inferred
  path: crates/connectors-console/src/connect.rs
- confidence: inferred
  path: crates/connectors-console/src/doctor.rs
- confidence: inferred
  path: crates/connectors-console/src/enrol.rs
- confidence: inferred
  path: crates/connectors-console/src/envelope.rs
- confidence: inferred
  path: crates/connectors-console/tests/personal_oauth.rs
- confidence: inferred
  path: crates/connectors-runtime/Cargo.lock
- confidence: inferred
  path: crates/connectors-runtime/src/composition.rs
- confidence: inferred
  path: crates/connectors-runtime/src/registry.rs
- confidence: inferred
  path: crates/connectors-runtime/src/registry_claims_tests.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/local_catalog_writes.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/local_gitlab_schedules.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/personal_oauth.rs
- confidence: inferred
  path: crates/driver-cdp/Cargo.lock
- confidence: inferred
  path: crates/driver-sip/Cargo.lock
- confidence: inferred
  path: crates/driver-speech/Cargo.lock
- confidence: inferred
  path: crates/integration-catalog/Cargo.toml
- confidence: inferred
  path: crates/integration-catalog/src/custody.rs
- confidence: inferred
  path: crates/integration-catalog/src/custody_refresh_tests.rs
- confidence: inferred
  path: crates/integration-catalog/src/custody_tests.rs
- confidence: inferred
  path: crates/integration-catalog/src/hosted.rs
- confidence: inferred
  path: crates/integration-catalog/src/lib.rs
- confidence: inferred
  path: crates/integration-catalog/src/oauth.rs
- confidence: inferred
  path: crates/integration-catalog/src/oauth_acquisition.rs
- confidence: inferred
  path: crates/integration-catalog/src/oauth_adversary_tests.rs
- confidence: inferred
  path: crates/integration-catalog/src/oauth_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/tests.rs
- confidence: inferred
  path: crates/protocol/src/connection.rs
- confidence: inferred
  path: crates/rtvbp-voice-endpoint/Cargo.lock
- confidence: inferred
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: inferred
  path: crates/service/src/connect_session.rs
- confidence: cited
  path: crates/service/src/planning.rs
- confidence: inferred
  path: crates/state-sqlite/src/lib.rs
- confidence: inferred
  path: crates/voice-runtime/Cargo.lock
- confidence: cited
  path: docs/design/07-credential-custody-topologies.md
- confidence: inferred
  path: docs/design/21-personal-oauth-callback-custody.md
- confidence: cited
  path: docs/security/secret-scan-baseline.md
- confidence: inferred
  path: ess/generated/clap
- confidence: inferred
  path: ess/system/domains/catalog.yaml
- confidence: inferred
  path: ess/system/domains/connection.yaml
- confidence: inferred
  path: ess/system/domains/deployment.yaml
- confidence: inferred
  path: json-schemas.toml
- confidence: inferred
  path: providers/gitlab.toml
- confidence: inferred
  path: providers/jira.toml
- confidence: inferred
  path: providers/slack.toml
revision: 74
---
## Acceptance

Verbatim from `docs/stories/S-013-connect-session-oauth-custody-in-personal-posture.md:22`. **read**

- [ ] A numbered design document (`docs/design/NN-*.md`, per AGENTS.md's series rule) names the
      custody chain for each flow it admits: **loopback redirect** (`http://127.0.0.1:<port>/…`
      registered as the deployment's redirect URI) and **device-code-style** flows, with the rule for
      choosing between them. It states, measured against real vendor registration rules rather than
      assumed, which providers refuse loopback, which refuse plain `http` on loopback, and which
      offer a device flow — naming the vendors checked and the date.
- [ ] The choice is **declared in the catalog** (the redirect-URI shapes and grant flows a provider
      admits) rather than decided at runtime by attempting one and falling back. A fallback would let
      the same connection succeed by different means on two machines, and the pair would be
      unreviewable.
- [ ] The connect-session invariants hold unchanged in personal posture: single-purpose, short-lived,
      never returns credential material to its creator, terminal event names the connection id and
      nothing else. The loopback listener is bound to **one** session, admits exactly one callback,
      binds the port for no longer than the session lives, validates `state`, PKCE, exact Host and
      expected Origin where present, and refuses CSRF, DNS-rebinding, state, or PKCE mismatch by
      name.
- [ ] The headless case is covered explicitly: an agent obtains the URL and a human completes it
      ([S-014](../../../docs/stories/S-014-auth-as-tool-result.md)'s consumer), **including** the case where the human's
      browser is not on the machine running the listener. That case is either supported by a named
      mechanism (device code, or a paste-back of the authorization response) or refused with a message
      that says what to do instead — never a silent hang until the session expires.
- [ ] BYO registration stays deployment configuration: no `client_id` and no secret appears in the
      catalog, the redirect URI a user must register is derivable from their configuration alone, and
      `connectors doctor` can print it.
- [ ] Whatever is decided is portable: a connection authorized in personal posture and one authorized
      in org posture are the same connection shape, because the catalog and the auth templates are
      the same text everywhere (vision, three postures).

## Context

Answer the domain model's open question 3 — how a connect session takes custody of an OAuth callback
on a machine with no public callback origin — and make the answer **declared per provider** rather
than discovered at runtime, so a personal-posture user can connect a real vendor through the same
connect-session contract every posture uses.

Source frontmatter: pillar Platform · areas [domain, service, catalog]. **read**

Source `note:` field, quoted: “domain model open question 3: personal posture has no public callback origin, so the custody chain is loopback redirect vs device-code-style flows — per provider, and per what each vendor's registration rules actually admit. A decision story: the deliverable is a numbered design document plus the schema consequence”

## Status

`backlog` in the source. Quoted from `docs/stories/S-013-connect-session-oauth-custody-in-personal-posture.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-013-connect-session-oauth-custody-in-personal-posture.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 2 revision(s)
- Legacy id `S-013`, recorded as the reference `legacy:S-013`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 3 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `docs/design/21-personal-oauth-callback-custody.md` — inferred.
- `crates/connector-spec/src/auth.rs` — cited.
- `crates/connector-spec/src/provider/auth_validation.rs` — inferred.
- `crates/connector-spec/schema/provider-toml.schema.json` — inferred.
- `crates/connector-spec/tests/main/oauth2_acquisition.rs` — inferred.
- `crates/catalog/src/lib.rs` — cited.
- `crates/catalog/src/table.rs` — cited.
- `crates/catalog-build/src/document.rs` — cited.
- `crates/catalog/tests/main/consumer_api.rs` — inferred.
- `docs/design/07-credential-custody-topologies.md` — cited.
- `providers/gitlab.toml` — inferred.
- `providers/slack.toml` — inferred.
- `providers/jira.toml` — inferred.

Medium confidence. Existing PendingStates/PKCE/custody are reusable. Current raw-credential loopback is not an OAuth callback. A decision and schema alone cannot satisfy runtime acceptance; confirm provider callback facts from official sources, settle callback/device transport and type any new values before dispatch. Reserve design 21 for this decision.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 9 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.

## Personal OAuth implementation decision and staged ownership

Design21 and sixteen additive ESS value types record the concrete development-scoped GitLab PKCE/device custody decision. Complete ESS0.18.0 validation reports10files valid; existing Connection/ConnectSession entities and lifecycles remain unchanged. Dedicated explicitly selected DevelopmentFile uses PreparedSecretStore with matching reads/writes and truthful unsealed readiness; it does not claim production custody. Legacy keyring credentials are unchanged. Provider facts are source-grounded; Slack/Jira remain measured compatibility gaps, not invented support.

Coordinator authorizes stage1 pure connector-oauth lib/token/device helpers and connect-session-transport lib/manifest/newoauth modules only. Stage1 has no provider/spec/catalog/runtime/CLI owner and no operator/provider I/O. Per-tree metadata-only locks may refresh after a necessary dependency; generated schema/pack/ESS and all AEP are coordinator-owned. Full operational stage follows rate source handoff. OAuth schema4 needs its own Atlas migration decision extension before canonical implementation. No frozen Connection wire change, implicit Connection creation authority, or model-tool URL disclosure is authorized.

OAuth story 21 can keep ConnectorConnection v0alpha1 only where authenticated owner plus the existing integration_ref and admitted auth_profile resolve exactly one configured stable Binding. Missing profile uses a deployment-owned default only when it is unambiguous. A supplied profile selects an admitted credential purpose within that same integration/owner; it cannot name another instance or broaden scope. Labels are display text and never participate in authority or target selection.

The session must capture that exact immutable Binding/Connection reference internally at creation and verify it again at completion. Zero or multiple matching bindings refuse before a session, callback listener, or provider request is created. Reauthorization under v1 is safe only for that same uniquely selected binding; it cannot promise to repair an arbitrary caller-selected Connection or use completion-time lookup to choose a different target.

Story 21 stage2 should enforce and test this limit before advertising its setup profile. Its helper-only stage1 needs no change. The held multi-credential branch is not a prerequisite and must not be imported.

The later auth-result design proposes ConnectorConnection v0alpha2 bound-remediation commands carrying an explicit existing connection_ref alongside operation_ref. The receiver resolves and admits that pair, derives integration/profile from its owner, then creates or acknowledges a session bound to it. It never trusts label/profile as a target, never creates a missing placement, and never treats a session as an operation grant. This is a new contract migration; rate v2 and unchanged Connection v1 do not reserve or authorize those fields.

## Helper stage handoff and custody decision — 2026-09-06

Stage1 source fc26525efaf2ca86ff4e4b003ecbcfe0751bf561 adds pure device polling/token validation and isolated owner-bound local OAuth instructions. The seven assigned source files and three allowed nested locks match the frozen source manifest. Complete package counts rose29 to58, including five compile-fail checks;18 distinct actual deciding failures are retained. Full affected Clippy and formatting passed. Existing locked versions/checksums remain unchanged; the console lock additionally closes the inherited GitLab connector-resolve/jsonschema graph. This is helper evidence, not completed acquisition/custody or independent whole-unit review. Raw commands, results, source hashes and lock deltas remain in `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/stage1-*`.

The stage2 read-only plan identified that the existing SQLite NORMAL durability cannot support ordering a recoverable decision before an fsynced credential commit. Add an explicit FULL-durability constructor in state-sqlite; retain existing callers. Design21 now defines the serialized completion claim as the authorization cutoff: after prepare, capture internal trusted authorized_at before the deadline while rechecking the same session/grant/generation. Persist that exact value-free decision before committing credentials. I/O can finish later. Before-claim expiry aborts; after-claim expiry cannot publish Expired; uncertain decision writes remain RecoveryRequired. Recovery aborts without a durable decision and finishes only a valid decision whose authorized_at precedes its captured deadline. No caller supplies or backdates time.

Connection v1 cannot represent endpoint-free Pending. After instructions retire, Committing/RecoveryRequired therefore uses the existing safe Unavailable refusal while internal recovery continues; only coherent publication produces Completed. Trusted bounded status polling repeats neither provider authorization nor operation execution. Barrier and restart cases must prove these boundaries in stage2. Runtime edits still await the rate unit's shared-owner handoff.

## Durable journal port integrated — 2026-09-06

Source81fac96b869c6f76a70ba3d15ab312269de2266d is integrated atec987d82. The additive SqliteState::open_full requires file-backed WAL with synchronous=FULL before schema creation and checks the effective mode. Existing NORMAL constructors and their callers retain their behavior. The package suite measured9 to14 passing tests, with3 deciding pre-fix failures retained; strict all-target Clippy and formatting pass. Tests cover effective PRAGMAs and real committed records observed from another connection and after reopening, not a simulated power loss.

Exact source SHA256c18c8cc6cb575d5ae5ea82471c2cc88727844235939987be4de4e54b88c15af5 was checked before commit. Raw reports, source patch, manifest and command results are in `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/stage2a-*`. This port alone does not complete OAuth runtime custody or its independent whole-unit review. The completed rate schema3 source must be published before schema4 publication; shared consumer owners remain with rate until review handoff.

## Independent private custody kernel slice — 2026-09-06

Stage2B may proceed from8e3797dde4309f878b66668ef14735bc5ac3df65 while the completed rate source receives independent review in its own tree. The exact source owners are crates/service/src/connect_session.rs; new crates/integration-catalog/src/custody.rs and custody_tests.rs; a private module registration only in integration-catalog/src/lib.rs; and a state-sqlite dependency edge only in integration-catalog/Cargo.toml. Existing scoped locks may receive required metadata closure with unchanged existing versions; any further path requires coordinator assignment before writing. No rate source or test ownership transfers here.

Use the receiver-owned private Preparing guard to prevent a false terminal session outcome after uncertain prepare/abort. It does not authorize credential commit. Only the later serialized current-authority claim sampled strictly before the original deadline, followed by confirmed FULL persistence of the captured decision, permits commit. Recovery never substitutes requested or backdated time. Preserve the original public Connection v1 states and raw-session behavior. The private kernel uses one unresolved transaction per dedicated prepared store and a contiguous safe reclamation watermark.

The kernel remains normally compiled but privately uncalled until actual OAuth acquisition wiring. One explicitly temporary module-scoped non-test expect(dead_code) is permitted for that intermediate state and must be removed by the later wiring before whole-story completion/publication. It is not permission to suppress other diagnostics, change existing assertions, expose a fake backend or claim runtime OAuth behavior. No credential/address secrets enter serialized journal or errors. The prepared-store port does not expose independent digest readback; recovery claims must respect that limit.

The retained stage2b-plan.md in the assigned OAuth scratch specifies the complete transaction/recovery matrix and decisive barriers, clock and real FULL SQLite/FileStore fixtures. Preserve actual pre-fix failures and existing tests. Full affected service/catalog tests, strict Clippy/format checks and exact lock closure are required for this slice. Whole OAuth runtime wiring, its independent review and complete gate remain outstanding; complete schema3 publication still precedes schema4 publication.

## Satellite lock closure observed before stage2B edits

All twelve locked/offline metadata checks were run on the assigned pre-edit eef5ccb6 tree. Seven passed; driver-speech, driver-cdp, driver-sip, rtvbp-voice-endpoint and voice-runtime refused their inherited stale Cargo.lock before any stage2B dependency/source change. Exact errors and argv are retained in stage2b-metadata-before.json and its indexed logs in the OAuth scratch. These five exact locks are assigned for metadata-only closure, preserving existing package versions/checksums. The completed rate branch independently refreshes the same inherited graph; the coordinator will reconcile the final combined dependency graph through Cargo rather than text-splicing lockfiles. Do not attribute preexisting closure drift to the new FULL-journal dependency.

## Private completion kernel integrated — 2026-09-06

Source3b6c72d31b1a4f197314eb295ce0ed6ba1f250cd is integrated at8af3acdb. The exact five source owners and eight authorized dependency locks match the frozen manifest. Service57→63 and integration-catalog35→56 yield119 passing tests, with13 distinct deciding failures retained. Strict affected Clippy, formatting and all12 locked/offline dependency graphs passed; existing versions and checksums were preserved. This is implementor evidence for the private kernel, not whole-unit independent review or operational OAuth completion.

The coordinator verified90 frozen evidence artifacts and all13 source/lock hashes, then retained source unchanged while cargo-cleaning the completed owned root/runtime targets for the20GiBfloor. Outside evidence remains in `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/stage2b-*`. Raw full report SHA256 2feda44c7cf55af9e371d3f8c2efa924a1d313adbec5caf8a1df2d23826b8a51; the complete portable copy replaces only the home-directory prefix and has SHA256 cca8d45e974e9d7c9374090074c80fc6ab658d88744fab107ce7855d464b7057. The separate short public report is a summary, not that complete copy.

The normally compiled private kernel is still uncalled. Actual daemon acquisition must remove its explicitly temporary dead-code expectation, supply the admitted unique binding and same dedicated store instance, enforce operation-read/recovery gates, and own instruction tasks and cancellation. Full operational OAuth wiring, whole-unit independent review and combined gate remain outstanding. Schema4 publication follows complete schema3 publication. No live OAuth or power-loss test is claimed.

## Bounded refresh adapter decision — 2026-09-06

The stage2C read-only supplement identified the actual kernel seam: completion currently requires a live ConnectSession, whereas on-demand refresh must use the same custody transaction without creating a session. Design21 now records a single receiver-owned30-second window captured under the binding gate before refresh egress. The current invoke/backend context has no admitted outer deadline. Use checked Unix-millisecond timing and a matching monotonic elapsed budget across token exchange, token-info and prepare, with no reset. The final short authority claim rechecks the same binding/grant/client/origin/generation and new evidence strictly before the original deadline. A timely claim permits later FULL-decision/commit I/O; uncertainty follows the same durable recovery rules, with no fake session or caller time.

Authorize the private custody Session/Refresh claim adapter within the existing custody.rs owner, preserving its session completion API. Add exactly crates/integration-catalog/src/custody_refresh_tests.rs through a test-only registration in custody.rs; keep the1441-line existing custody_tests.rs bytes and assertions intact. Runtime/acquisition witnesses stay in their already assigned owners. These changes wait for the exact accepted rate source handoff; this record alone starts no runtime edits.

Read-only plans remain immutable in the assigned OAuth scratch. The files are stage2c-plan-supplement.md SHA256fbf5d12f88049f82d207b55e60e4443cb98fcfc84f97355bcb0faefdf7b9dd72; stage2c-refresh-window-supplement.md SHA256264313aff8886943a4728fd88cd6c7c71623cf41045661a496530aa160f3a6d2. The coordinator records the local bounded-time choice; no additional contract field, entity, public session state, live provider compatibility, or executed refresh result is claimed.

## Recording-actor correction — 2026-09-06

The coordinator agent, agent:cli-ten-slack-first, performed the three preceding updates at revisions45–47. Those CLI invocations omitted AEP_ACTOR and therefore inherited the configured human actor. They are coordinator source-evidence, scope and design updates; they are not human decisions or new approvals. The existing approved wave is their authorization. This append records the actual actor without rewriting the journal, the recorded timestamps, or source history.

Affected immutable event IDs: story:connect-session-oauth-custody-in-personal-posture@45#0~23b3e9b7463d8afa; story:connect-session-oauth-custody-in-personal-posture@46#0~888bee356beb49a9; story:connect-session-oauth-custody-in-personal-posture@47#0~21b13e63d853ea45. Exact raw lines and SHA256 hashes are retained outside the repository in `~/.cache/cw6/p/coordinator-actor-correction-inventory.json`. No status move, approval record or test_result was among these events. Subsequent coordinator CLI writes explicitly set the agent actor.

## Independent private refresh adapter slice — 2026-09-06

The full stage2C runtime handoff still waits for reviewed rate source. A bounded private refresh adapter can proceed independently now: its only source owners are crates/integration-catalog/src/custody.rs and new crates/integration-catalog/src/custody_refresh_tests.rs. Their private kernel is already frozen and integrated; neither file is in the rate unit. This supersedes the blanket wait for those two exact private paths only. No acquisition/backend registration, shared catalog lib, service API, provider/config/protocol/client/console/runtime source, manifest, lock, model or generated surface is assigned to this slice.

Implement the recorded Design21 thirty-second receiver-owned refresh window and private claim adapter using the existing session API and FULL custody sequence. Preserve the complete current custody_tests.rs bytes, every session case, durable journal compatibility and the currently recorded absence of independent prepared-store digest readback. A real refresh claim must bind the admitted operation/current authority, same connection and previous generation before any later egress; tests may supply explicit private receiver seams, never a permissive production default or synthetic ConnectSession. The later acquisition backend still owns actual provider requests and the monotonic remaining-budget wrapper around them.

Retain actual deciding failures against executable pre-fix behavior before implementation, then run full affected integration-catalog tests and strict checks with the existing per-tree target and20GiBfloor. This slice adds no dependency and needs no unrelated cold workspace builds. Freeze source, exact command/results, case preservation and complete raw/portable reports in stage2b-refresh-* under the existing OAuth scratch. It remains uncalled private support under the already authorized temporary module expectation; actual wiring must remove that expectation. Operational refresh/OAuth, shared-owner handoff, whole-unit independent review and publication remain pending.

## Private refresh adapter integrated — 2026-09-06

Source 9d1cef375ade657d1fbca24cc25097e3bdd09b9d is integrated at 30f8f522bee4d59976bfc1485ad8f96b0fb6b6a4. Only custody.rs and new custody_refresh_tests.rs changed. All 73 frozen evidence hashes and all three source/preserved-test hashes were verified, as was the complete prefix-only portable report. Its SHA256 is dce8878e31f7a1593b89d0eeafbf8af8a4cb4eaf9ce57fb54188b7e13d52cccf; source patch SHA256 fd38e515c216eed1c53d496c049a9952506b5a9dcb9e751e8bd01b81daf07e49. Exact commands, results, raw/portable reports and source snapshots remain in `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/stage2b-refresh-*`.

The full integration-catalog suite passes 56 to 66 cases, with no failures or ignored cases, and strict Clippy/formatting pass. Four actual unimplemented-entry failures and one injected monotonic-clock rollback failure were retained; compiler/supertrait and enum-size Clippy failures are disclosed separately. The original 1441-line custody_tests.rs, existing complete signature and journal/type definition block remain byte-identical. The session and refresh variants share the same FULL custody transaction, and refresh creates no ConnectSession.

Actual runtime composition must share one RefreshOwner and binding gate across acquisition, read and refresh, install the real current-authority receiver, and wrap token/token-info egress using the handle's original remaining budget. It must retain that gate through operation dispatch, remove the temporary unused-module expectation, and keep remote rotation uncertainty unavailable. These are still pending runtime obligations; private support tests do not prove operational OAuth, shared registry ownership or live provider compatibility. Whole-unit independent review and combined publication remain pending.

After the freeze, shared disk dropped below the 20 GiB reserve due to concurrent workspace consumption. The coordinator paused new builds, reverified all refresh evidence and source hashes, and cargo-cleaned only this completed runtime target. All 1157 tracked source hashes remained identical; exact cleanup evidence is retained in `~/.cache/cw6/p/completed-oauth-observer-target-clean.json`. No claim is made that shared free space remained above the floor throughout the session.

## Stage 2C source handoff after final rate review — 2026-09-06

The rate final adversary is green with no findings: reviewed test/source head 0c026610e67f3ec6da0a8014813f3d88c4e72799 is integrated at a222c138b5b61ce3251f95fe54422b48ed696ee2 alongside private OAuth kernel and refresh source 9d1cef37. The sole console lock conflict preserves all package versions, sources and checksums from both parents. All twelve locked/offline metadata graphs pass unchanged locks; ESS validates, its six outputs are regenerated, and all documentation links pass after assembly. These are integration checks, not a combined full-suite claim.

Release the already scoped stage 2C source owners to the OAuth implementor in its existing managed checkout after coordinator fast-forward. Implement the source/declaration/config/schema-4 and actual acquisition/runtime/trusted-client work recorded in stage2-plan.md and stage2c-plan-supplement.md, using the now-completed private refresh adapter instead of repeating that old missing-seam refactor. Preserve all 12 rate adversary file preimages and all original custody/session/refresh cases. Shared-file tests are additive only. Existing Connection v1 and Operation v1/v2 bundle bytes remain frozen; no Design 22 Operation v3 or Connection v2 runtime work is assigned.

The same receiver-owned binding gate must serialize acquisition, coherent reads, refresh and operation dispatch. Install real current-authority/clock receivers and one dedicated FileStore instance; use the FULL journal before preparing credentials, retire/join acquisition tasks before claiming completion, and retain guarded uncertainty and original timing through recovery. Wrap actual refresh egress with the fixed original remaining budget; do not fake a session or assert rollback restored a rotated remote token. Remove the temporary private-module dead-code expectation as actual backend calls land. Keep production/development custody refusals, exact preconfigured target selection, observed token-info scope/client evidence, private instruction delivery, one-shot limits and no operation replay.

AEP, ESS models/projections, Git, worktree lifecycle, source publication, installation and live provider/operator actions remain coordinator-owned. The implementor may change only current machine-scoped source/dependency/provider/schema/config/document paths required by this stage, preserving existing versions/checksums and acquiring any evidenced additional exact path before writing it. Schema 3 source publication remains a prerequisite for later schema 4 writer publication, not for isolated implementation. The held multi-credential unit supplies no source or authorization.

Retain actual deciding failures through runnable baseline/scaffold callers, then complete affected suites and strict checks before the whole-OAuth independent review. Missing symbols/fixture/compiler errors are separate evidence. Start with source and test preparation; acquire the coordinator's build slot before Cargo compilation because the shared disk reserve is narrow and schema 3 publication gates are also queued. Use jobs 1, one target at a time and the strict 20 GiB floor. Freeze complete raw/portable stage2c-* reports, source inventory and command/results outside targets; no operational success or completed story is claimed until composed behavior and whole-unit review/gates establish it.

## Declaration reexport closure — 2026-09-06

The stage 2C implementor identified crates/connector-spec/src/lib.rs as the explicit public auth-type reexport owner. Assign that exact path only for the new Design 21 admission-type reexports required by catalog-build and tests. The typed vocabulary already exists in ESS; this is a declaration/consumer closure, not a new model, protocol or helper abstraction. Preserve all existing exports and rate behavior. Compilation remains subject to the shared build slot.

## Typed registration scope ceiling closure — 2026-09-06

Design 21 names allowed scopes among deployment-owned registration values at its deployment registration paragraph. The proposed source PersonalOAuthRegistration correctly requires allowed_scopes, but the existing ESS struct omitted it. Add the required field as List<connectors.deployment.Scope> in the already scoped deployment domain, using the existing Scope type. This records the deployment ceiling; it does not grant scopes or replace observed token-info evidence and the existing Connection grant. ESS validates and its six deterministic CLI outputs are regenerated with report JSON retained outside source. Runtime/config admission, scope grammar and authorization enforcement remain implementation/test obligations.

## Durable pre-egress refresh marker and schema fixture closure — 2026-09-06

The stage 2C implementor identified a recovery gap before credential prepare: a provider may rotate the refresh credential before token-info succeeds, while the custody journal still contains only the old publication. Record the private OAuthRefreshAttempt value in the already scoped deployment domain and the dated Design 21 decision before implementing persistence. Its version, exact Connection reference, canonical binding SHA256 and previous generation contain no secret or provider response. The existing FULL SQLite path must confirm this marker before refresh egress; recovery keeps the old generation unavailable until a newer coherent publication for the same binding passes current authority and evidence checks. Unknown marker writes permit zero egress; unknown reads or deletion remain unavailable. This does not change the custody journal envelope, public protocols, Connection lifecycle, or the single FileStore instance.

Add crates/connector-resolve/src/document.rs to this story's cited scope only for four test-fixture schema-version literals at the current lines 754, 811, 847 and 873. They describe the currently emitted canonical document and may use schema 4 or the supported-version constant after the writer changes. Preserve all production resolver bytes and every existing assertion. Also permit the already scoped crates/catalog-build/tests/main/catalog_invariants.rs active planned-schema lookup at the current line 687 to follow schema 4/the current constant. This is an explicit narrow exception to shared tests' additive-only rule. Preserve its full original preimage and all existing assertions; the frozen schema 3 rate adversary cases at current lines 2530, 2568 and 2721 remain byte-exact, with separate schema 4 coverage added.

The marker's version/key/digest shape, bounded decoding, FULL durability, authority checks and recovery behavior remain explicit implementation and adversarial-test obligations, not claims established by ESS. The current one-workspace compilation slot and prospective 16 GiB reserve apply. Whole-unit OAuth review and publication remain pending.

## Historical evidence-reference scan closure — 2026-09-06

The pinned scanner identified an evidence basename immediately following the wording about the scratch directory as a generic API key. All 15 observed matches at 8 distinct historical fingerprints were reconstructed value-free and proved equal to an existing non-secret evidence basename, not either adjacent SHA256 citation. The complete original red and proof remain in the rate publication scratch; proof SHA256 a1aa8ab682786c0305b6d076fca776c7b0e3958ccb52da9549b1655b32320287. The current mutable prose now uses a sentence boundary to keep the same file references without the misleading key/value shape. The historical journal and commits remain unchanged.

Assign only .gitleaksignore and docs/security/secret-scan-baseline.md to record exact newly confirmed historical fingerprints under the existing baseline policy. Regenerate from the complete scan with the ignore file absent, retain and reclassify the newly observed matches, preserve existing detector rules and all prior exact fingerprints, and run the unchanged pinned gate afterward. No credential rotation, rule/path exemption, history rewrite, implementation success or completed OAuth review is implied by this reference-only classification.

## Retain frozen schema 3 during schema 4 planning — 2026-09-06

Assign crates/catalog-build/src/pipeline.rs only for the additional planned frozen schema 3 artifact. Its current plan emits frozen schema 2 plus the current schema; advancing the current writer to schema 4 would otherwise treat the required schema 3 artifact as an orphan. Use the already scoped document/workspace owners to return and place the exact frozen schema 3 bytes. Do not reconstruct that frozen schema from schema 4 or change its validators, identity or semantics. The full planned artifact inventory must retain both older schemas alongside current schema 4.

The two rate URI cases in the already scoped catalog_invariants.rs currently validate freshly rendered Slack documents against the frozen v3 validator (functions rate_adversary_canonical_source_urls_match_authoring_reader and rate_final_actual_provider_loading_preserves_uri_and_vendor_contract). The schema 4 writer changes their top-level document identity. Permit a narrow fixture-only exception to their previous byte-preservation assignment: make only those generated test documents mutable as needed and set only their top-level schema_version/$schema identities to the exact frozen v3 identities before the existing v3 assertions. Keep all operation bytes, original URI vectors, validator configuration and assertions unchanged. The definition-only middle v3 case remains byte-exact. Retain complete original preimages and add separate v4 producer/schema coverage over these same URI vectors.

This authorizes v3-compatible test fixtures, not a production downgrade or removal of schema 4 fields to fit an old reader. If any field beyond those two top-level identities is incompatible with the frozen validator, stop this adaptation and report that concrete field; do not strip it or weaken the validator. The independent schema 3 publication candidate and its reviewed tests remain unchanged. These bounded fixture edits belong to the subsequent whole-OAuth/schema 4 unit and must be disclosed in its implementation and independent review evidence.

The trusted frontend uses setup connect <provider>, with --auth-profile naming the declared purpose and optional --instruction-file naming an owner-only output file. Design 21 records these exact spellings. Existing ESS components.yaml explicitly classifies setup connect as an unspecified flow, because this frontend forwards multiple acts rather than owning a domain command. Its current Clap projection cannot declare these flow options; do not invent a service command or handler ownership to make it emit them. The actual Rust parser, guided-flow owner and their tests enforce the options and pre-session terminal/file admission. The existing ESS exception and generated group tree remain accurate.

## Existing raw configuration fixture closure — 2026-09-06

Add crates/integration-catalog/src/tests.rs only for oauth: None in the existing entry helper's CatalogIntegrationConfig literal at current line 258. The new optional typed field requires that explicit old/raw configuration value. Preserve every existing test, assertion and other byte in this frozen rate preimage, apart from formatter placement of that one field. This is a subsequent OAuth constructor closure; the schema 3 publication candidate remains unchanged. Retain the exact preimage and disclose the one-field adaptation in whole-unit OAuth review evidence.

## Measured schema4 fixture and module closure — 2026-09-06

The first affected root execution retained actual failures: canonical artifact count fixtures still expected5 where the new frozen-v3-plus-current-v4 output contains6; the resolver's shared synthetic source-fidelity fixture declared3; and the first GitLab adversary combined a frozen-v3 validator with current-reader acceptance of the generated GitHub document. Its reader deciding test separately failed with UnsupportedSchema found4 before implementation. Preserve every first execution and original source preimage.

Assign catalog-build/src/check.rs and tests/main/no_network.rs only for the exact fixture artifact-count expectations5to6, retaining snapshot/no-write and zero-network assertions. In connector-resolve/src/resolve.rs, change only the shared synthetic fixture's current schema identity3to4; preserve every body omission/null/requiredness and request assertion. Existing catalog_invariants current-output closure may admit the typed personal_flows field while continuing to reject unknown and invalid acquisition declarations.

Assign connector-resolve/tests/adversary_gitlab_pass1.rs solely for a versioned compatibility split of its schema_versions_and_profiles_fail_closed case. Preserve every previous version/profile class against the exact frozen v3 schema, using only the generated GitHub document's top-level schema_version/$schema normalization to3. Preserve all operation/vendor/request bytes. Separately test actual current-v4 document and reader acceptance, invalid profiles, old3 and future5 refusal and equivalent malformed version classes. The historical current-reader-positive-v3 assertion describes the older reader; migrate that assertion explicitly to current4 rather than retaining a false oracle or copying an obsolete reader. Preserve all other original adversary cases/assertions and the historical file in Git; disclose this narrow compatibility exception in whole-unit review. Any nonidentity field incompatibility with frozen v3 must be reported before alteration.

For the measured1500-line fence, allow only cohesive new OAuth extraction: connectors-config/src/personal_oauth.rs and personal_oauth_tests.rs own the new declarations/validation/tests. All preexisting config tests remain in personal.rs. The existing CatalogIntegrationConfig::validate method may move intact alongside its new OAuth check to keep that existing owner below the limit; preserve its old validation branches, messages and public API. connectors-client/src/personal_oauth.rs owns only the new client implementation/helpers/tests; original lib APIs/tests remain intact via registration/reexports. If the measured split still exceeds the limit, connectors-client/src/completion.rs may receive only the existing dedicated CompletionEndpoint type/implementation, unchanged, with its existing tests and API preserved. integration-catalog/src/oauth_acquisition.rs receives only the cohesive new acquisition implementation, keeping private state and authority with the existing OAuth owner. Record exact formatted sizes and moved-byte preservation. No broad refactor, line-limit waiver, assertion deletion or production policy relaxation is assigned.

Coordinator refreshed the stale service lifecycle evidence comments in ess/system/domains/connection.yaml against integrated9c6ce892 and regenerates existing ESS derivatives. No entity, lifecycle, command, auth authority or transition changes. All root/source/backend/config/client closures remain preparation for actual operational tests and independent whole-unit review; schema3 publication is a separate verified candidate.

## Lifecycle prose fence closure — 2026-09-06

Measured root fixture failure: crates/catalog-build/tests/main/ess_claim_fence.rs still requires the earlier lifecycle prose and reads lines().nth(232) as the service Failed writer after stage2B moved that code. Add only this exact test owner for a bounded fixture update to the current explicit ESS claim and named service lifecycle owner. Reuse its existing brace-counted item helper to locate fail_pending and its actual Failed assignment instead of a moving absolute line. Keep the non-vacuous requirements that the document states both halves, the hosted writer scan is empty, and the other registry really writes Failed. Preserve planted-hosted-writer, removed-claim and missing-other-writer refusal behavior and every unrelated ESS claim case. No conditional skip, weakened implication or production/model edit is delegated. The coordinator owns the refreshed comments; the implementor owns only these assertions and actual test evidence.

Formatted module previews fit the existing fence: config personal1479/new OAuth162/new tests84; client lib1500/new OAuth367; backend OAuth1402/acquisition626. No CompletionEndpoint move or size waiver is needed. The existing private config_ref helper may become pub(super) solely for the new sibling validation owner; no public API or caller authority changes. Preserve the existing validation method's old branches/messages and all old config/client tests.

## Current service planning fixture closure — 2026-09-06

The first OAuth root/client regression execution retains five service failures at the same synthetic document helper: crates/service/src/planning.rs declares schema_version 3 in the cfg(test) document function, while the implemented current reader requires schema4. Its remaining59service cases, catalog-build148, client30, protocol64 and server103 passed in that actual execution. The unmodified command, exit and failures remain in stage2c-root-client-regression-tests-1 evidence; this record does not relabel the run green.

Add crates/service/src/planning.rs only for that one current-document fixture literal3to4. Preserve every production byte, existing test, assertion and other fixture field; retain the exact original preimage and disclose the change in whole-unit OAuth review. This is the current synthetic source fixture, not a frozen-version contract case. Reexecute the affected service checks and complete the remaining owner validations. No reader widening, legacy document rewrite, independent authentication scope or test skip is assigned.

## Prospective whole-unit review test owner — 2026-09-06

The read-only whole-unit checklist is prepared at SHA256 78f3b2af0490335ded1abaa310a94f10e892f0339b4b76c3aa295e9fc9fdef8b. Its two-member manifest and observation index are retained in the assigned OAuth scratch; source observations were provisional, and the checklist is neither an executed attack nor a verdict. Both ordinary whole-unit review passes remain unstarted until the coordinator supplies a frozen complete implementation, full report and explicit review assignment.

Add the inferred test owner crates/connectors-client/tests/personal_oauth_adversary.rs for public-API Unix/HTTP fixtures proving the trusted client handoff and its closed output/error boundaries. Cargo's normal integration-test discovery and the existing dependencies suffice; no production hook, manifest change or public protocol edit is assigned. The other ten proposed test files are already machine-scoped by this story. Keep existing test assertions and original reports, with only the earlier explicitly recorded fixture-version/citation exceptions. This exact new file is for the later independent reviewer; the implementor continues its already scoped inline client/console cases.

The review must cover real controlling-terminal and private-file behavior, interrupted private delivery, exact owner/profile/Connection checks, callback/device bounds, actual FULL-store reopen recovery, refresh uncertainty and no operation resend, frozen/current schema compatibility, and mixed raw/OAuth custody. The coordinator has separately identified unexecuted client completion checks: Callable state, exact pending/status/Describe correlation, and successful daemon labels/references containing a synthetic private marker. Their actual deciding evidence and narrow implementation remain the implementor's responsibility before freeze. Preserve the meaning of the daemon ConnectionDescription and use trusted local display data plus the existing receiver-derived expected Connection reference for the console's public summary; no new wire target or CLI selection authority is introduced.

## Additional first-pass test owner for the existing size fence — 2026-09-06

The reviewer reports the completed first root full command exited 101. Alongside the measured personal_flows authoring-contract mismatch, the existing size fence rejected its appended cases because crates/integration-catalog/src/oauth_tests.rs grew from 1,470 to 1,699 lines. Root independently compared the current file to 6fae9df000986f39d009a2e8503bdff03db41762 and verified all original bytes remain an exact prefix, with 229 added lines. This size failure is the reviewer's test placement, not an introduced product defect or a resource interruption; retain the original output and the two already passing deciding runs.

Prospectively add exactly crates/integration-catalog/src/oauth_adversary_tests.rs as an inferred test owner. Move only the reviewer's two new cases, oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation and oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline, verbatim to that file under use super::*;. The already assigned oauth_tests.rs receives only an appended path/module registration, preserving all original bytes. Its parent is already cfg(test), so no production module or hook is required. Preserve all assertions, literals and function names; only the new cases' module qualification changes. Keep the 1,500-line fence unchanged.

The additional owner and exact validation assignment are in whole-unit-adversary-1/oauth-test-owner-assignment.md. Root preimage proof is oauth-pass1-test-owner-preimage.json under the coordinator scratch. After placement, execute both exact new cases and rerun the affected root full command with no-fail-fast; remaining strict/fmt checks see final source. Preserve every original deciding/compiler/full-run observation and report the scope exception. The complete first-pass review is still pending and production correction remains held until its immutable report is recorded. This does not add an attack or widen implementation scope.

## Completed first review and first correction routing — 2026-09-06

Review-result:cli-oauth-adversary-1-20260906 holds the complete returned public report unchanged, including its findings block. Report SHA256 is 9dffb196779d9b7e2d0ccd7ba4ea1284daca9f1e0294c9368f80361413cbead5; raw SHA256 is 4455c529c5d33f4b0f4d88ad157e3f98a1e2a43d52af94717803a1ae7fd1bc55. Root verified all 227 evidence members and all 1,193 source hashes, plus prefix-only public/raw equality and every old test line in order. The exact eleven test-only paths are locally committed at 22e4d11ee1aad753d379e59f407cba3fc88bf6e2, parent 6fae9df0. Both identities are the organization bot and source hashes remain exact after commit. This preserves the real red case on a private branch; no red unit was integrated or published.

The operational cohort grew 1,496 to 1,509 executed: 1,508 passed, one failed, one preexisting reader measurement ignored. Fifty existing helper/SQLite cases and one actual published-reader witness passed separately. Final strict Clippy and formatting passed in all four owning workspaces. The completed report preserves the actual fixture/compiler/size failures and usage-limit continuation; they are not additional product findings or review passes. Full command logs are held in its manifest; the returned report gives the first red verbatim and final-suite summaries with exact log references. Root preserves that reporting form unchanged rather than representing the summaries as full output quotations.

The single CONFIRMED warning is the public provider loader accepting personal_flows = [] despite the new authoring schema's minItems 1. The report's origin remains undecided because no old-loader execution occurred. Coordinator routing inference: this complete unit introduced the field, schema constraint and empty validation guard, verified byte-for-byte between published 0c69450921ab1794c81dadec915b717a61bf0983 and frozen 6fae9df000986f39d009a2e8503bdff03db41762. Current reachability is test helper personal_oauth.rs:73 through provider::load and loading.rs:135; OAuth2Spec at auth.rs:384 erases presence and auth_validation.rs:300 returns early. The schema requirement is at provider-toml.schema.json:1447. Catalog loading reaches the same loading implementation through catalog-build/src/seam.rs:188. Root's oauth-authoring-origin-root-analysis.json pins the source hashes and explicitly labels this inference. Route it as an introduced authoring-contract correction, without claiming baseline execution or credential exposure.

The same original implementor, resume_gitlab, receives only the already scoped auth.rs production correction in whole-unit-correction-1/brief.md: reject explicit empty admission while preserving omission, serialization, valid admissions, all old/new assertions and generated/frozen bytes. No schema weakening or wider implementation is assigned. The reviewer has finished every command and relinquished compilation. Transfer the sole slot to this correction in the same OAuth worktree/private target under the existing 12 GiB cap and 12/8/16 GiB reserves. Its new assigned scratch is the whole-unit-correction-1 directory and TMPDIR is ~/.cache/cw6/oc1. Execute the retained failing case, affected root full package set, strict Clippy and formatting as specified; freeze outcomes and release the slot before sealing. Outcome fixed is recorded only after the actual correction lands. Second whole-unit review, integration/publication/delivery and the supported installation remain outstanding; auth runtime and the credential decision remain held.
