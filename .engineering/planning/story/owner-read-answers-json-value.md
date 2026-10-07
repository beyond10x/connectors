---
format: aep.planning-md/3
id: story:owner-read-answers-json-value
kind: story
status: active
title: A read through the owner answers its result as a JSON value
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:cli-json-answers-as-json
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T01:58:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T01:58:11Z", actor: "human:timo", revision: 3}
---
## Defect

Found 2026-10-07 by the GitLab feed unit of wave 20261007a. `story:cli-json-answers-as-json`
(#105, merged in #127, unreleased) made `operations invoke` answer the provider result as a
JSON value, but only on the path `apps/connectors/src/local/operations.rs` `project()` serves.
A read that the owner runs answers it as JSON text: the supervisor wraps the provider body as a
string (`crates/connectors-host/src/local/owner/supervisor.rs:901-903`,
`"result": body` with `body` a `String`). The generated CLI contract types the field `Json` and
accepts any JSON value there, a string included, so no test saw it.
`cli_journey::gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart`
(`adapters/catalog/tests/local_runtime/cli_journey.rs:394`, ignored) fails on it: `Null` where
`7` is expected.

## Acceptance

- Every `operations invoke` answer carries the provider result as a JSON value, whichever path
  serves it (owner read, owner write, direct): a test per path pins it, and the ignored journey
  above passes again when run with `--ignored`.
- The CLI contract's result check refuses a JSON string where the provider answered an object,
  or the specification states why it cannot (a `Json` field admits any value).
- `CHANGELOG.md` Unreleased: the #105 entry stays true as written.
