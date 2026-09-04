---
format: aep.planning-md/1
id: story:preserve-claude-refresh-scope
kind: story
status: implemented
title: Preserve Claude refresh scope
summary: Accept scope-omitting Claude refresh responses only by carrying forward previously verified scopes.
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/subscription-custody/src/lib.rs
revision: 6
---
## Outcome

An OAuth-backed Claude Code subscription renews in Connector custody before an Agent attempt receives its leased access token.

## Context

The initial OAuth exchange works and reconnect temporarily restores Agent execution, but the provider refresh response can omit `scope`. The decoder currently requires that field before the existing carry-forward logic can preserve the already verified scope set.

## Acceptance

- A refresh response without `scope` is accepted only when a previous verified OAuth record supplies the required `user:inference` scope.
- A first authorization response without scopes remains refused.
- Refresh-token rotation remains in Connector custody and a new task can redeem a refreshed access token without re-authentication.

## Scope

- `crates/subscription-custody/src/lib.rs`
- Connectors release metadata and live model-backed Devcenter evidence.
