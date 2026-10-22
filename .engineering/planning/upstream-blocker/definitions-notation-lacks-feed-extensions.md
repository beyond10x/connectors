---
format: aep.planning-md/3
id: upstream-blocker:definitions-notation-lacks-feed-extensions
kind: upstream-blocker
status: open
title: The definitions notation cannot express the Jira and Confluence feeds
relations:
- blocks: story:jira-feed-binding
- blocks: story:confluence-feed-binding
revision: 2
---
# The definitions notation cannot express the Jira and Confluence feeds

`crates/connectors-catalog/src/definition.rs` mirrors the definition schema of the
`beyond10x/connectors-definitions` repository (`spec/domains/definition.yaml` there) field for
field, and refuses any field that schema does not name. `definition_import` holds every shipped
catalog selection against what that notation lowers to.

The feed engine extensions of wave 20261021 (`story:catalog-feed-engine-extensions`, and the
`utc_offsets` time-query widening added by the Jira unit) have no counterpart in the notation:

- offset container paging (`cursor.offset`); the notation's cursor takes `parameter`, `next`, `last`;
- the query-language item template (`query`, JQL or CQL);
- `utc_offsets {least, greatest}` on the time query.

Observed 2026-10-22 in the Jira unit's tree: with the feed block in
`adapters/catalog/providers/jira/operations.json`, `cargo test -p connectors-catalog --test
definition_import` went from 40 passed to 33 passed, 7 failed:
"unknown field `offset`, expected one of `parameter`, `next`, `last`".

Wave 20261021 therefore ships the engine extensions and the Jira and Confluence bindings as
fixture-backed bindings: the feed declarations live under `adapters/catalog/tests/feed/`, and the
shipped Jira and Confluence selections carry no `feed`.

Clears when the definitions schema names these fields, this repository's reader and lowering
(`definition.rs`, `import.rs`) carry them, and the two feed blocks move into the shipped
selections with `definition_import` green.
