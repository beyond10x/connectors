# Wave 2026-09-30c: Google provider — Discovery ingest, OAuth, reads and guarded writes

Skill version: aep implementing 0.19.1. Operator approved the plan for `epic:google-workspace-reads`
on 2026-09-30 and then set the goal "google connectors fully implemented, ready for an agentic e2e
trial", so every story of the epic except the live deck read joined the wave as its dependencies
were committed.

## Units

Every unit: `aep:implementor`, then two `aep:adversary` passes (the second correction verified by
the coordinator), each pass recorded as a `review-result` with an outcome per finding.

| story | branch | adversary findings (pass 1, pass 2) | into the wave |
|---|---|---|---|
| story:catalog-discovery-projection | impl/catalog-discovery-projection | 8, 4 | e97f54665 |
| story:catalog-oauth2-refresh-profile | impl/catalog-oauth2-refresh-profile | 3, 5 | aa885094f |
| story:catalog-google-drive-reads, story:catalog-google-slides-reads | impl/catalog-google-reads | 3, 4 | c3f3fffb5 |
| story:catalog-google-drive-writes | impl/catalog-google-drive-writes | 5, 3 | 03dfc6feb |
| story:catalog-google-slides-writes | impl/catalog-google-slides-writes | 4, 3 | 99f0a40a5 |
| story:cli-oauth-loopback-acquisition | impl/cli-oauth-loopback-acquisition | 4, 2 | 945438266 |
| story:catalog-repeated-query-parameters, story:catalog-selection-required-parameters, story:catalog-engine-provider-refusal-shapes | impl/catalog-query-parameter-shapes (built on wave 20260930b) | 5, 2 | via 8bb5c9b36 |
| story:catalog-google-calendar-reads, story:catalog-google-gmail-reads | impl/catalog-google-calendar-gmail-reads | 8, 5 | 8bb5c9b36 |
| story:catalog-google-calendar-writes | impl/catalog-google-calendar-writes | 5, 4 | ae4dd085f |
| story:catalog-google-gmail-draft-writes | impl/catalog-google-gmail-draft-writes | 3, 4 | step 4 of the final integration |

Not in the wave: `story:catalog-google-live-deck-read`, blocked by
`credential-blocker:google-oauth-client` (the operator's Google Desktop OAuth client).

Integration: `wave/20260930c-google-oauth-discovery` (store and wave page) and
`wave/20260930c-final` (every unit, origin/main 0.19.0 and wave 20260930b merged).

## Decisions taken during the wave

- Gmail `cse.identities.patch` is excluded from the projection: its path differs from its siblings
  only by parameter name, which OpenAPI 3.0 does not allow.
- `repair_connection` is reported for an unauthorized invoke of a read; a guarded write refused for
  its credential reports `retry_status` (story:guarded-write-credential-refusal-says-repair).
- Writes use a separate instance per API with the write scope; `connections repair` cannot add a
  scope because the configuration revision and profile are part of the connection binding.
- The Google authorize URL is `https://accounts.google.com/o/oauth2/auth`, the `auth_uri` of a
  downloaded Desktop client file; the consent flow compares the two byte for byte.
- A repeated query parameter's declared type stays a union with the string form, for callers that
  send comma-joined strings (Jira, Confluence).
- A selection may close a write body (`body_keys`); `users.drafts.send` accepts only `{"id"}`.

## Filed during the wave

story:http-empty-query-no-trailing-question-mark, story:catalog-guard-optional-preflight-values,
story:guarded-write-credential-refusal-says-repair.

## Pre-flight and environment

- `/` fell to 100% (1.1G free) at dispatch; the operator freed space. `/dev/shm` builds do not work
  for this repository (`crates/connectors-host/src/local/filesystem.rs:45-55` refuses world-writable
  ancestors; keyring custody needs ext4), so builds used `<tree>/target` with a short TMPDIR.
- The repository gate must run from a short checkout: from a managed worktree the CLI owner tests'
  Unix socket paths exceed `SUN_LEN` (recorded in docs/development.md).

## Commits approval authorises

The unit commits and their correction commits, the merges into the integration branches, the
closing store commit, and the merge into `main` through a pull request once the gate is green.
