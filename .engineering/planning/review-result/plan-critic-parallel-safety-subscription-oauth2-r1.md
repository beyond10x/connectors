---
format: aep.planning-md/3
id: review-result:plan-critic-parallel-safety-subscription-oauth2-r1
kind: review-result
status: active
title: 'Plan critic parallel safety, round 1: subscription OAuth2 drafts'
relations:
- reviews: story:owner-rotating-refresh
- reviews: story:google-consent-on-the-oauth2-core
- reviews: story:connect-runs-an-oauth2-grant
- reviews: story:claude-subscription-connector
- reviews: story:oauth2-grant-core
- reviews: story:oauth2-refresh-metadata-authority
revision: 1
---
verdict: needs-revision
reviewer: aep:plan-critic-parallel-safety (opus), round 1, 2026-10-25, over epic:subscription-oauth2-access and 9 drafted stories. Its note on wave 1 (verbatim): "The proposed wave 1 (`story:oauth2-grant-core` + `story:oauth2-refresh-metadata-authority`) is safe to run in parallel. The two share no file, declared or inferred, and neither one's acceptance reads state the other produces. The verdict is set by the later waves"

```findings
- file: .engineering/planning/story/owner-rotating-refresh.md
  line: 56
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the oauth2_grant connection, entry shape and AuthorizationServer record that story:connect-runs-an-oauth2-grant creates, and no depends_on edge orders them (waves puts owner-rotating in wave 3, before connect-runs in wave 7); they also share contracts/auth/acquisition/v1alpha1/semantics.md (cited both) and owner/transport.rs inside owner/ (inferred)"
- file: .engineering/planning/story/google-consent-on-the-oauth2-core.md
  line: 11
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it and story:owner-rotating-refresh share CLI wave 3 with no edge, and both must add the connectors-oauth2 dependency to crates/connectors-host/Cargo.toml and Cargo.lock, which neither scope names (inferred)"
- file: .engineering/planning/story/connect-runs-an-oauth2-grant.md
  line: 43
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope omits ess/domains/cli.yaml and ess/domains/auth_access.yaml, which its Model edits, so its collision with story:configuration-change-error and story:connection-next-actions-no-cycle on ess/domains/cli.yaml is invisible to waves"
- file: .engineering/planning/story/claude-subscription-connector.md
  line: 60
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope omits ess/domains/oauth2.yaml and crates/connectors-host/src/local/metadata/entity-runtime-definitions.json, which its state_in_exchange field change rewrites, and story:spec-models-operation-admission claims that definitions file"
- file: crates/connectors-build/src/gate.rs
  line: 163
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:oauth2-grant-core's scope omits website/docs/reference/crates.md, which the new package changes and the gate's docs check requires regenerated, and story:claude-getting-started-tutorial rewrites the same line without either body naming the other (inferred for grant-core)"
- file: .engineering/planning/story/oauth2-refresh-metadata-authority.md
  line: 1
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it and story:spec-models-operation-admission both regenerate entity-runtime-definitions.json and edit metadata_entities.rs, and neither body names the other"
```
