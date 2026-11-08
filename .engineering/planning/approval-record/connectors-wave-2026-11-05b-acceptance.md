---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-05b-acceptance
kind: approval-record
status: approved
title: The next connectors wave's stories are accepted by the orchestrator
relations:
- decides: story:owner-bounded-read-rechecks-semantic-effects
- decides: story:claude-mcp-add-registration-is-recorded
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T17:29:36Z", actor: "human:operator", revision: 2}
---

## Acceptance (orchestrator, 2026-11-05)

The directing session `b10x-chef-orchestrator` accepted two stories for the next connectors wave, in a message to the connectors wave lead on 2026-11-05 after v0.67.0 was verified:

> Next wave: I accept story:owner-bounded-read-rechecks-semantic-effects and story:claude-mcp-add-registration-is-recorded (one recorded headless run on the existing login, sanitized as in v0.67.0).

Terms:
- The `claude mcp add` story is accepted with exactly one recorded run, on the existing login. It is sanitized and scanned as the v0.67.0 recording was (approval-record:connectors-claude-recording-on-the-existing-login): `b10x-gates scan-text` and the grep for tokens, emails, hostnames and home paths show 0 hits before any commit, and no raw capture is committed.
- story:mcp-mutating-tool-call-revision-rule was accepted earlier for this wave; its decision is recorded in its body.

The wave is proposed to the orchestrator before any build, and no cargo runs before its go.
