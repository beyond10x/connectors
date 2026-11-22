---
format: aep.planning-md/3
id: review-result:wave-b-def-reader-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the definition reader's feed refusals
relations:
- reviews: story:definition-reader-refuses-what-the-feed-engine-refuses
revision: 1
---
## Adversary pass 1 on story:definition-reader-refuses-what-the-feed-engine-refuses

Reviewed commit 36ca2641e1 on impl/definition-reader-refuses-what-the-feed-engine-refuses (base
9b97114df). Model: opus.

```
verdict: red
cases: executed 96→119, red 9
origin: introduced 0, pre-existing 9, undecided 0
```

The four Acceptance rows hold. Nine older gaps break the Outcome sentence ("every feed the engine
refuses at load"). The new uncommitted file `adapters/catalog/tests/definition_feed_engine_adversary.rs`
lowers an edited GitLab definition and loads it with `Engine::with_feed`. Each case below is red
because the reader lowers a feed that the engine then refuses:

- a lookup with a required query parameter left unbound;
- an `items.list` with a required parameter left unbound;
- a lookup that is not a GET;
- containers without a `kind_word`;
- a capability claim the declaration does not support;
- a profile with a space;
- a page-size cap above 100;
- an item id that is not a JSON pointer;
- a body without a representation.

What held:

- The parser move: `query_template.rs` is the old engine code word for word, and `feed_engine` 13
  passed.
- Templates: one parser serves both sides.
- No feed the engine accepts is refused by the reader on the twice, required-on-resume or
  query-key rules.
- The shipped GitLab, Jira and Confluence v1 feeds and every 536945d definition still lower.

```findings
[
  {"file": "crates/connectors-catalog/src/definition.rs", "line": 2746, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "pre-existing", "message": "The reader lowers feeds whose list, lookup or items operation leaves a required parameter unbound or is not a GET, both of which the engine's bind refuses at load (feed.rs:605, feed.rs:566); shown by two red cases and the not-a-GET case in definition_feed_engine_adversary.rs."},
  {"file": "adapters/catalog/src/feed.rs", "line": 673, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "Feed::new refuses profile shape, max_limit outside 1..100, a missing kind_word, unsupported capability claims, non-pointer ids and an empty body representation, none of which the reader checks, so the story Outcome's 'every feed' does not hold (six red cases)."}
]
```
