---
format: aep.planning-md/3
id: story:definition-reader-refuses-what-the-feed-engine-refuses
kind: story
status: draft
title: The definition reader refuses every feed the catalog engine refuses at load
revision: 1
---
## Outcome

A user's connector definition whose feed the catalog engine would refuse at load is refused when the definition is lowered, under a named rule, instead of lowering and failing when the instance starts.

## Gaps measured

Measured by the wave 20261022 U1 adversary, pass 2 (review-result:w1022-u1-adversary-r2). The reader lowers each of these, and the engine refuses it at load:

| # | case | engine refusal |
|---|---|---|
| 1 | a parameter bound twice, for example `items.query` adds the key `query.parameter` binds | `feed binds … twice` |
| 2 | a required `page.parameter` (or `time`) sent only on resume (`adapters/catalog/src/feed.rs:700`) | `… only on resume, but it is required` |
| 3 | `containers.query` or `items.query` keys the operation does not take | `feed binds …, which … does not take there` |
| 4 | template slots the engine's parser refuses: a slot beside a quote or backslash, an unknown slot, a stray brace, a slot inside a literal (`feed.rs:420-531`); rules 4 and 5 count substrings | `… puts a slot beside a quote or backslash; the engine quotes each value` |

Also: the definitions schema summary (`spec/domains/definition.yaml:2` in connectors-definitions) still says `first` holds `{container}` once, without "and `{since}` never". The reader's rule 4 and the engine both say so. A one-line change in that repository, made through its own route.

## Acceptance

- One test case per row above in `crates/connectors-catalog/tests/definition_feed.rs`, each failing at the base and passing at the head, each asserting a refusal name.
- Rows 4: the reader reuses the engine's template parser, or one shared parser, rather than a second substring count.
- The shipped Jira and Confluence v1 feeds and every 536945d definition still lower (`definition_import`).
