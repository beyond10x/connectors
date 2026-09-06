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
- confidence: inferred
  path: crates/catalog-build/src/document.rs
- confidence: inferred
  path: crates/catalog-build/src/document_schema.rs
- confidence: inferred
  path: crates/catalog-build/src/document_tests.rs
- confidence: inferred
  path: crates/catalog-build/src/workspace.rs
- confidence: inferred
  path: crates/catalog-build/tests/main/catalog_invariants.rs
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
- confidence: inferred
  path: crates/connector-spec/schema/provider-toml.schema.json
- confidence: inferred
  path: crates/connector-spec/src/auth.rs
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
  path: crates/connectors-client/src/lib.rs
- confidence: inferred
  path: crates/connectors-client/src/model.rs
- confidence: inferred
  path: crates/connectors-config/examples/gitlab-personal-oauth.example.toml
- confidence: inferred
  path: crates/connectors-config/src/lib.rs
- confidence: inferred
  path: crates/connectors-config/src/personal.rs
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
  path: crates/integration-catalog/src/oauth_tests.rs
- confidence: inferred
  path: crates/protocol/src/connection.rs
- confidence: inferred
  path: crates/rtvbp-voice-endpoint/Cargo.lock
- confidence: inferred
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: inferred
  path: crates/service/src/connect_session.rs
- confidence: inferred
  path: crates/state-sqlite/src/lib.rs
- confidence: inferred
  path: crates/voice-runtime/Cargo.lock
- confidence: cited
  path: docs/design/07-credential-custody-topologies.md
- confidence: inferred
  path: docs/design/21-personal-oauth-callback-custody.md
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
revision: 48
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

Read-only plans remain immutable in the assigned OAuth scratch: stage2c-plan-supplement.md SHA256fbf5d12f88049f82d207b55e60e4443cb98fcfc84f97355bcb0faefdf7b9dd72; stage2c-refresh-window-supplement.md SHA256264313aff8886943a4728fd88cd6c7c71623cf41045661a496530aa160f3a6d2. The coordinator records the local bounded-time choice; no additional contract field, entity, public session state, live provider compatibility, or executed refresh result is claimed.

## Recording-actor correction — 2026-09-06

The coordinator agent, agent:cli-ten-slack-first, performed the three preceding updates at revisions45–47. Those CLI invocations omitted AEP_ACTOR and therefore inherited the configured human actor. They are coordinator source-evidence, scope and design updates; they are not human decisions or new approvals. The existing approved wave is their authorization. This append records the actual actor without rewriting the journal, the recorded timestamps, or source history.

Affected immutable event IDs: story:connect-session-oauth-custody-in-personal-posture@45#0~23b3e9b7463d8afa; story:connect-session-oauth-custody-in-personal-posture@46#0~888bee356beb49a9; story:connect-session-oauth-custody-in-personal-posture@47#0~21b13e63d853ea45. Exact raw lines and SHA256 hashes are retained outside the repository in `~/.cache/cw6/p/coordinator-actor-correction-inventory.json`. No status move, approval record or test_result was among these events. Subsequent coordinator CLI writes explicitly set the agent actor.
