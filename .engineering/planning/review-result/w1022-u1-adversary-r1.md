---
format: aep.planning-md/3
id: review-result:w1022-u1-adversary-r1
kind: review-result
status: active
title: 'Adversary pass 1, wave 20261022 U1: definition reader feed extensions'
summary: 'red: 2 warnings, 2 notes; fixed in b132826b8 or dispositioned'
relations:
- reviews: story:definition-reader-feed-extensions
revision: 1
---
Adversary pass 1 on the U1 working tree (base `6f44d34b1`). **Verdict: red.** 2 warnings, 2 notes. 5 cases were added; 3 fail.

| severity | path:line | finding | disposition |
|---|---|---|---|
| warning | crates/connectors-catalog/src/definition.rs:2385 | Rule 4 accepts a `query.first` that names `{since}`. The engine refuses it at load (`adapters/catalog/src/feed.rs:524-526`). | Refused at lowering under rule 4. The story's rule 4 text now says "`{container}` once and `{since}` never". |
| note | crates/connectors-catalog/src/definition.rs:2325 | `query.parameter` and `page.parameter` are not checked against the items operation. | Checked at lowering and refused by name. |
| warning | README.md:291 | The Jira row omits `projects.list` and `project.get`. | U2 turns the row into a count. The coordinator sets the Jira count at integration, and the adversary's row test is dropped. |
| note | apps/connectors/src/local/definition.rs:601 | The CLI maps every lowering refusal to `usage()`. | Pre-existing; out of scope for this story. |

These held: `FEED_MAX_LIMIT` 100 against Jira's page cap; the claim that lowered operations are unchanged (`definition_import`, 40 cases); the hand re-pinned revisions; the provider-name test, red on a mutant; and GET-only feed operations (`feed.rs:670`).
