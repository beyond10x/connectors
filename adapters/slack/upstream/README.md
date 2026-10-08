# Pinned Slack Web API source

`slack_web_openapi_v2_without_examples.json` is the unmodified Slack Web API
Swagger 2.0 document from `slackapi/slack-api-specs` at commit
`bc08db49625630e3585bf2f1322128ea04f2a7f3`, retrieved on 2026-10-08:

- [Exact source](https://raw.githubusercontent.com/slackapi/slack-api-specs/bc08db49625630e3585bf2f1322128ea04f2a7f3/web-api/slack_web_openapi_v2_without_examples.json)
- SHA-256: `8b92da26a3c5b11d20042a9f36d81f1fa6fc9382c5ddc471babb68b91936bc3a`
- Size: 1,039,581 bytes; Git blob `f7b1affd1fb34f9473cd87980428883019b29c07`, the
  blob the repository's tree names at that commit.
- Swagger `2.0`; `info.title` `Slack Web API`; `info.version` `1.7.0`; host
  `slack.com`, base path `/api`, scheme `https`; 174 operations; one security
  scheme, `slackAuth` (OAuth 2.0 `accessCode`, with no `tokenUrl`).
- [License at the same revision](https://raw.githubusercontent.com/slackapi/slack-api-specs/bc08db49625630e3585bf2f1322128ea04f2a7f3/LICENSE),
  MIT, retained as `LICENSE`.

The document is pinned as published. Gitleaks 8.30.0 reports no finding in it,
and it carries no e-mail address.

The catalog does not read Swagger 2.0 directly: `crates/connectors-catalog`
projects it to OpenAPI 3.1.0 under `swagger2-openapi/1` (rule table in
`src/swagger.rs`), and `crates/connectors-catalog/tests/swagger.rs` asserts the
digest above, that the projection ingests all 174 operations with none
unsupported, and that `conversations.list`, `conversations.history` and
`conversations.replies` are present with their query parameters. No Slack
bundle, selection set or runtime exists yet. Slack does not endorse this
adapter. Refreshing the source means replacing this file and the digest the
test asserts together.
