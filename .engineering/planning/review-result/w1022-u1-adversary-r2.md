---
format: aep.planning-md/3
id: review-result:w1022-u1-adversary-r2
kind: review-result
status: active
title: 'Adversary pass 2, wave 20261022 U1: definition reader feed extensions'
summary: green; four engine-side refusals the reader does not mirror, recorded as a follow-up story
relations:
- reviews: story:definition-reader-feed-extensions
revision: 1
---
Adversary pass 2 on the U1 rework. **Verdict: green.** Both round-1 fixes hold. Neither the shipped feeds nor the 536945d definitions are falsely refused.

The engine refuses these four cases at load, and the reader still lowers them. Each was measured in a private mount namespace. They are recorded as follow-up work, not fixed in this unit:

| # | case | engine refusal |
|---|---|---|
| 1 | a parameter bound twice (`items.query` adds `jql`) | `feed binds jql … twice` |
| 2 | a required page parameter sent only on resume | `… only on resume, but it is required` |
| 3 | `containers.query` or `items.query` keys the operation does not take | `feed binds bogus, which … does not take there` |
| 4 | a slot beside a quote in `first` (rules 4 and 5 count substrings; the engine parses the template, `adapters/catalog/src/feed.rs:420-531`) | `… puts a slot beside a quote or backslash` |
