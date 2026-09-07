---
format: aep.planning-md/1
id: story:connect-session-oauth-custody-in-personal-posture
kind: story
status: implemented
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
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: catalog/airtable.catalog.json
- confidence: cited
  path: catalog/alertmanager.catalog.json
- confidence: cited
  path: catalog/algolia.catalog.json
- confidence: cited
  path: catalog/anthropic.catalog.json
- confidence: cited
  path: catalog/argocd.catalog.json
- confidence: cited
  path: catalog/asana.catalog.json
- confidence: cited
  path: catalog/asterisk.catalog.json
- confidence: cited
  path: catalog/b10x.catalog.json
- confidence: cited
  path: catalog/babelforce.catalog.json
- confidence: cited
  path: catalog/bitbucket.catalog.json
- confidence: cited
  path: catalog/box.catalog.json
- confidence: cited
  path: catalog/calendly.catalog.json
- confidence: cited
  path: catalog/claude-code.catalog.json
- confidence: cited
  path: catalog/clickup.catalog.json
- confidence: cited
  path: catalog/cloudflare.catalog.json
- confidence: cited
  path: catalog/confluence.catalog.json
- confidence: cited
  path: catalog/connector-document-v4.schema.json
- confidence: cited
  path: catalog/contentful.catalog.json
- confidence: cited
  path: catalog/datadog.catalog.json
- confidence: cited
  path: catalog/discord.catalog.json
- confidence: cited
  path: catalog/docusign.catalog.json
- confidence: cited
  path: catalog/dropbox.catalog.json
- confidence: cited
  path: catalog/figma.catalog.json
- confidence: cited
  path: catalog/fly.catalog.json
- confidence: cited
  path: catalog/freshdesk.catalog.json
- confidence: cited
  path: catalog/front.catalog.json
- confidence: cited
  path: catalog/github.catalog.json
- confidence: cited
  path: catalog/gitlab.catalog.json
- confidence: cited
  path: catalog/google.catalog.json
- confidence: cited
  path: catalog/grafana.catalog.json
- confidence: cited
  path: catalog/hubspot.catalog.json
- confidence: cited
  path: catalog/intercom.catalog.json
- confidence: cited
  path: catalog/jira.catalog.json
- confidence: cited
  path: catalog/klaviyo.catalog.json
- confidence: cited
  path: catalog/launchdarkly.catalog.json
- confidence: cited
  path: catalog/loki.catalog.json
- confidence: cited
  path: catalog/mailchimp.catalog.json
- confidence: cited
  path: catalog/microsoft_graph.catalog.json
- confidence: cited
  path: catalog/miro.catalog.json
- confidence: cited
  path: catalog/mysql.catalog.json
- confidence: cited
  path: catalog/newrelic.catalog.json
- confidence: cited
  path: catalog/notion.catalog.json
- confidence: cited
  path: catalog/okta.catalog.json
- confidence: cited
  path: catalog/openai.catalog.json
- confidence: cited
  path: catalog/openrouter.catalog.json
- confidence: cited
  path: catalog/pagerduty.catalog.json
- confidence: cited
  path: catalog/postgresql.catalog.json
- confidence: cited
  path: catalog/postmark.catalog.json
- confidence: cited
  path: catalog/prometheus.catalog.json
- confidence: cited
  path: catalog/resend.catalog.json
- confidence: cited
  path: catalog/runpod.catalog.json
- confidence: cited
  path: catalog/salesforce.catalog.json
- confidence: cited
  path: catalog/sendgrid.catalog.json
- confidence: cited
  path: catalog/sentry.catalog.json
- confidence: cited
  path: catalog/shopify.catalog.json
- confidence: cited
  path: catalog/slack.catalog.json
- confidence: cited
  path: catalog/statuspage.catalog.json
- confidence: cited
  path: catalog/stripe.catalog.json
- confidence: cited
  path: catalog/supabase.catalog.json
- confidence: cited
  path: catalog/trello.catalog.json
- confidence: cited
  path: catalog/twilio.catalog.json
- confidence: cited
  path: catalog/typeform.catalog.json
- confidence: cited
  path: catalog/vercel.catalog.json
- confidence: cited
  path: catalog/webflow.catalog.json
- confidence: cited
  path: catalog/zendesk.catalog.json
- confidence: cited
  path: catalog/zoom.catalog.json
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-build/src/check.rs
- confidence: cited
  path: crates/catalog-build/src/document.rs
- confidence: cited
  path: crates/catalog-build/src/document_schema.rs
- confidence: cited
  path: crates/catalog-build/src/document_tests.rs
- confidence: cited
  path: crates/catalog-build/src/pipeline.rs
- confidence: cited
  path: crates/catalog-build/src/workspace.rs
- confidence: cited
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: cited
  path: crates/catalog-build/tests/main/ess_claim_fence.rs
- confidence: cited
  path: crates/catalog-build/tests/main/no_network.rs
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/catalog-reader/src/lib.rs
- confidence: cited
  path: crates/catalog-reader/tests/main/pack.rs
- confidence: cited
  path: crates/catalog/src/lib.rs
- confidence: cited
  path: crates/catalog/src/table.rs
- confidence: cited
  path: crates/catalog/tests/main/consumer_api.rs
- confidence: cited
  path: crates/connect-session-transport/Cargo.toml
- confidence: cited
  path: crates/connect-session-transport/src/lib.rs
- confidence: cited
  path: crates/connect-session-transport/src/oauth.rs
- confidence: cited
  path: crates/connect-session-transport/src/oauth_tests.rs
- confidence: cited
  path: crates/connector-oauth/src/device.rs
- confidence: cited
  path: crates/connector-oauth/src/lib.rs
- confidence: cited
  path: crates/connector-oauth/src/token.rs
- confidence: cited
  path: crates/connector-resolve/src/document.rs
- confidence: cited
  path: crates/connector-resolve/src/resolve.rs
- confidence: cited
  path: crates/connector-resolve/tests/adversary_gitlab_pass1.rs
- confidence: cited
  path: crates/connector-spec/schema/provider-toml.schema.json
- confidence: cited
  path: crates/connector-spec/src/auth.rs
- confidence: cited
  path: crates/connector-spec/src/lib.rs
- confidence: cited
  path: crates/connector-spec/src/provider/auth_validation.rs
- confidence: cited
  path: crates/connector-spec/src/provider/schema_sync.rs
- confidence: cited
  path: crates/connector-spec/tests/main.rs
- confidence: cited
  path: crates/connector-spec/tests/main/ir_roundtrip.rs
- confidence: cited
  path: crates/connector-spec/tests/main/oauth2_acquisition.rs
- confidence: cited
  path: crates/connector-spec/tests/main/oauth_token_endpoint.rs
- confidence: cited
  path: crates/connector-spec/tests/main/personal_oauth.rs
- confidence: cited
  path: crates/connector-spec/tests/main/provider_schema.rs
- confidence: cited
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-cli/tests/one_shot_operations.rs
- confidence: cited
  path: crates/connectors-client/src/completion.rs
- confidence: cited
  path: crates/connectors-client/src/lib.rs
- confidence: cited
  path: crates/connectors-client/src/model.rs
- confidence: cited
  path: crates/connectors-client/src/personal_oauth.rs
- confidence: cited
  path: crates/connectors-client/tests/personal_oauth_adversary.rs
- confidence: cited
  path: crates/connectors-config/examples/gitlab-personal-oauth.example.toml
- confidence: cited
  path: crates/connectors-config/src/lib.rs
- confidence: cited
  path: crates/connectors-config/src/personal.rs
- confidence: cited
  path: crates/connectors-config/src/personal_oauth.rs
- confidence: cited
  path: crates/connectors-config/src/personal_oauth_tests.rs
- confidence: cited
  path: crates/connectors-console/Cargo.lock
- confidence: cited
  path: crates/connectors-console/src/connect.rs
- confidence: cited
  path: crates/connectors-console/src/doctor.rs
- confidence: cited
  path: crates/connectors-console/src/enrol.rs
- confidence: cited
  path: crates/connectors-console/src/envelope.rs
- confidence: cited
  path: crates/connectors-console/tests/personal_oauth.rs
- confidence: cited
  path: crates/connectors-runtime/Cargo.lock
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry_claims_tests.rs
- confidence: cited
  path: crates/connectors-runtime/tests/local_catalog_writes.rs
- confidence: cited
  path: crates/connectors-runtime/tests/local_gitlab_schedules.rs
- confidence: cited
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: cited
  path: crates/connectors-runtime/tests/personal_oauth.rs
- confidence: cited
  path: crates/driver-cdp/Cargo.lock
- confidence: cited
  path: crates/driver-sip/Cargo.lock
- confidence: cited
  path: crates/driver-speech/Cargo.lock
- confidence: cited
  path: crates/integration-catalog/Cargo.toml
- confidence: cited
  path: crates/integration-catalog/src/custody.rs
- confidence: cited
  path: crates/integration-catalog/src/custody_refresh_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/custody_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_acquisition.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_adversary_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/tests.rs
- confidence: cited
  path: crates/protocol/src/connection.rs
- confidence: cited
  path: crates/rtvbp-voice-endpoint/Cargo.lock
- confidence: cited
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: cited
  path: crates/service/src/connect_session.rs
- confidence: cited
  path: crates/service/src/planning.rs
- confidence: cited
  path: crates/state-sqlite/src/lib.rs
- confidence: cited
  path: crates/voice-runtime/Cargo.lock
- confidence: cited
  path: docs/design/07-credential-custody-topologies.md
- confidence: cited
  path: docs/design/21-personal-oauth-callback-custody.md
- confidence: cited
  path: docs/security/secret-scan-baseline.md
- confidence: cited
  path: ess/generated/clap
- confidence: cited
  path: ess/system/domains/catalog.yaml
- confidence: cited
  path: ess/system/domains/connection.yaml
- confidence: cited
  path: ess/system/domains/deployment.yaml
- confidence: cited
  path: json-schemas.toml
- confidence: cited
  path: providers/gitlab.toml
- confidence: cited
  path: providers/jira.toml
- confidence: cited
  path: providers/slack.toml
revision: 38
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

## Current bounded OAuth acceptance — 2026-09-07

Design21 records the supported provider and custody boundary. Implement configured GitLab public PKCE and device authorization, exact callback/state/PKCE ownership, one existing ConnectSession owner, verified subject continuity, FULL credential publication/recovery and refresh without invocation replay. GitLab support is DevelopmentOnly using the explicitly selected unsealed DevelopmentFile store; existing raw/keyring behavior remains unchanged. Slack/Jira compatibility gaps are explicit. Schema4 and matching readers declare the supported capabilities instead of runtime trial-and-fallback. Preserve the original acceptance as history; no production-store or universal-provider claim follows from this development-scoped implementation.

## Publication continuation — 2026-09-07

The same approved CLI wave continues from public main 0c69450921ab1794c81dadec915b717a61bf0983. The coordinator carries the tested nonplanning source into this isolated publication checkout and records its exact content equivalence before CI. Raw execution records and the original implementation branches remain preserved separately. Published branch history is not rewritten. This public store keeps its existing journal prefix and records current scope and evidence through AEP.

Seven selected stories already reached published main. Personal OAuth passed its two ordinary reviews; the first whole-authentication review is running against the frozen identical source. The several-credentials story remains held under its existing decision blocker. No version bump, release tag, live provider action or additional credential-store support is authorized here. Source publication, normal documentation delivery, supported source installation and the two already authorized daemon replacements remain in the approved delivery sequence.

Current source includes Operation v3 default with explicit operation-only v2, Connection v2 bound remediation, schema4 with matching readers, and development-only GitLab public PKCE/device authorization through a dedicated unsealed store. Hosted remediation remains Unsupported after admission where no acquisition owner exists. Protected instructions stay outside model output; completion ends with fresh validation and an explicit new invocation. Full affected local gates pass: server115, runtime455 per default/no-default configuration with two existing PostgreSQL ignores, console107, CLI140, strict affected checks and the final catalog/Markdown/story/ESS gate. Initial root1301pass/2fail/4oldignores was closed by full affected server115 and catalog-cli14 rechecks. Complete sharded CI and auth review remain pending.

Exact runtime final seal: eba21e5f93152eb0aacd01dadf31a19569401421c8a8397f3b09216d11ed7b04 (179 members). Exact client final seal:334ab4c9adf141862dcb9a701e930f80dcdf909dfb3a1c1142dc75af3c8fcae7 (95 members). All first failures remain in those records. Counts describe executed cases per command, not a unique aggregate. Future review results are recorded in full before any findings are routed.

## Full implementation gate completed — 2026-09-07

Manual release rehearsal 34069722221 completed successfully on exact published candidate 0837f79beb4202405942e669c417057f69aac58a. All twelve workspace gates, shared checks and four native Linux/macOS builds passed. Each native binary passed its version/help smoke checks and produced an unexpired development artifact. The publish job was skipped, as required for workflow_dispatch. Exact run, job and artifact responses are retained in coordinator evidence; the public run is https://github.com/beyond10x/connectors/actions/runs/34069722221 .

Both remaining implementations completed their two ordinary review passes. Authentication's final one-file document correction is verified separately with every test byte preserved. The resulting source includes explicit development-only personal GitLab OAuth, bound authentication recovery, matching schema 4 readers and the stated predecessor protocol support. Implementation and executable verification are complete. The coordinator continues source main publication, normal Website/Atlas documentation delivery and the already authorized local installation; this record does not claim those delivery steps have finished. No version bump or release tag has been made.
