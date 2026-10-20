---
format: aep.planning-md/3
id: decision-blocker:slack-openrouter-pinned-source
kind: decision-blocker
status: open
title: Nobody has decided how Slack and OpenRouter instances get a bundle and selection once the tree is gone
relations:
- blocks: story:catalog-provider-data-removed
revision: 1
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
