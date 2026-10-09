---
format: aep.planning-md/3
id: review-result:adversary-catalog-credential-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the catalog credential field
relations:
- reviews: story:parity-slack-discovery-reads
revision: 1
---
## Scope

The catalog selection field `credential` (4273a3ceb) and its first users, the Slack selections of 8bc743fec, on wave/20261009a. Tests: `adapters/catalog/tests/adversary_credential_pass1.rs` (38940419f), 11 cases.

## Findings

- F1 (warning, introduced): a write naming its credential kept an open body, so a caller-supplied `body.token` was sent.
- F2 (warning, introduced): a selection whose `body_keys` admit its credential loaded.
- F3 (note, introduced): a guard could map a caller input onto the preflight read's credential parameter.

None was reachable from a shipped selection. Held without a break: spellings and wrappers of `token`, a value carrying `&token=`, query and header together, path and cookie locations, withhold and bounds beside a credential, configuration and descriptor revisions, the user-token connection reaching bot operations.

## Outcome

Fixed in f935679ef: a top-level body key naming a credential (any case) is refused before any request and by the declared body schema; `body_keys` and `preflight.values` naming a credential are refused at load; probe operations drop credential parameters. `cargo test -p connectors-catalog-provider`: 458 passed, 0 failed, 25 ignored. Left: nested body objects and percent-encoded spellings in an open body are not checked, as `docs/local-catalog-provider.md` states.
