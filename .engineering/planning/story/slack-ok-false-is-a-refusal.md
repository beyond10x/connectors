---
format: aep.planning-md/3
id: story:slack-ok-false-is-a-refusal
kind: story
status: active
title: A Slack ok:false answer is a typed refusal, never an empty read
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:slack-user-token-history-reads
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T23:35:16Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T23:35:16Z", actor: "human:timo", revision: 4}
---
## Outcome

A Slack Web API answer with `ok: false` is a refusal with the provider's error name, never a
successful empty read.

## Today

Reported by a downstream consumer on connectors 0.50.0, 2026-10-14: a Slack call answering HTTP
200 with `{"ok": false, "error": "..."}` comes back as exit 0, status 200, so a failed read
(`not_in_channel`, `missing_scope`, `ratelimited`, `invalid_cursor`) looks like an empty result.
Fixing this makes 8 operations safe to automate.

## Change

Spec first in the Slack catalog selection: every Slack operation declares the `ok` envelope, and
`ok: false` maps to a typed refusal (`forbidden` for `missing_scope` and `not_in_channel`,
`capacity` with the retry hint for `ratelimited`, `invalid_input` for cursor and argument errors,
`service_failure` otherwise) carrying the provider's `error` string. If the catalog engine cannot
express a body-level failure on a 200, that is the engine change, made generic for any provider that
signals failure in its body.

## Acceptance

- Recorded fixtures for each mapped error answer their refusal and a non-zero exit.
- No Slack read answers success on a body with `ok: false`.

## Scope

Primary: adapters/catalog/src/lib.rs (read_body :2026, Selection :397, RefusalCode), feed.rs, providers/slack/{operations,user-operations}.json, catalog spec selection.yaml/guard.yaml, contracts/catalog semantics.md, tests/slack.rs, docs local-catalog-provider.md, catalog-slack.md. Not cli.yaml, not host. Collides with slack-user-token (user-operations.json, tests/slack.rs) → same unit or sequenced.
