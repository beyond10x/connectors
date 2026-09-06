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
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/personal_oauth.rs
- confidence: inferred
  path: crates/integration-catalog/Cargo.toml
- confidence: inferred
  path: crates/integration-catalog/src/custody.rs
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
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: inferred
  path: crates/service/src/connect_session.rs
- confidence: inferred
  path: crates/state-sqlite/src/lib.rs
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
revision: 39
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
