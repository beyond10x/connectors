---
format: aep.planning-md/3
id: decision-blocker:slack-openrouter-pinned-source
kind: decision-blocker
status: cleared
title: Nobody has decided how Slack and OpenRouter instances get a bundle and selection once the tree is gone
relations:
- blocks: story:catalog-provider-data-removed
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T09:14:28Z", actor: "human:timo", revision: 3}
---
## Question

How do Slack and OpenRouter instances get a bundle and selection once `adapters/catalog/providers`
and `adapters/catalog/generated/bundles` are gone?

## Why it blocks

Their guides configure the instance with paths into the tree (`docs/catalog-slack.md:403,412`,
`docs/catalog-openrouter.md:68,77`). Today Slack and OpenRouter ship no connector declaration
(`adapters/catalog/tests/connector_declarations.rs:186-190`), so they are not served by name from the
embedded release, and the install cache holds definitions, not lowered bundles. The shape that would
serve them by name also settles whether Slack is one connector or two (`slack`, `slack-user`), which
`story:catalog-connector-declarations` leaves to a later story.

Raised by the re-plan of epic:catalog-connectors-only-in-definitions for option C on 2026-10-20.

## Decided: option A

2026-10-20, by the b10x-chef-orchestrator session on the operator's delegation, reversible (option B,
merging the two Slack definitions, can still follow later). Serve `slack`, `slack-user` and
`openrouter` by name from the embedded release: Slack stays two connectors, as the definitions are.
The Slack and OpenRouter guides move to `--connector`; an instance configured by path moves through
`setup migrate`, with a test that a path-configured instance migrates.

The options put to it: A serve all three by name (three new connector names, guides rewritten,
`setup migrate` for path-configured instances); B one Slack connector with two auth profiles, merging
the two definitions in connectors-definitions; C keep path configuration at a stable cache path.
