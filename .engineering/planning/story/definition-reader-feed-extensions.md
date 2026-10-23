---
format: aep.planning-md/3
id: story:definition-reader-feed-extensions
kind: story
status: active
title: The definition reader lowers the Jira and Confluence feeds of connectors-definitions 536945d
summary: definition.rs and import.rs accept offset, url_parameter, a query position, utc_offsets, url and url_base; both feeds move into the shipped selections
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-feed-engine-extensions
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md
- confidence: cited
  path: adapters/catalog/contracts/feed/v1alpha1/jira.md
- confidence: cited
  path: adapters/catalog/providers/confluence-v1/operations.json
- confidence: cited
  path: adapters/catalog/providers/jira/operations.json
- confidence: cited
  path: adapters/catalog/tests/confluence_binding.rs
- confidence: cited
  path: adapters/catalog/tests/confluence_v1_feed.rs
- confidence: cited
  path: adapters/catalog/tests/connector_declarations.rs
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: cited
  path: adapters/catalog/tests/jira.rs
- confidence: cited
  path: adapters/catalog/tests/jira_feed.rs
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/feed/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/catalog_feed_conformance.rs
- confidence: cited
  path: crates/connectors-build/src/confluence_feed_conformance.rs
- confidence: cited
  path: crates/connectors-build/src/jira_feed_conformance.rs
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
- confidence: cited
  path: crates/connectors-catalog/tests/definition_feed.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/confluence-v1.yaml
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: docs/catalog-confluence.md
- confidence: cited
  path: docs/catalog-jira.md
- confidence: cited
  path: website/data/status.json
- confidence: cited
  path: website/docs/connectors/confluence.md
- confidence: cited
  path: website/docs/connectors/index.md
- confidence: cited
  path: website/docs/connectors/jira.md
- confidence: cited
  path: website/docs/status.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T09:48:11Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":6}}, executor: "agent:connectors-session", correlation: "wave-20261022"}
- {from: "proposed", to: "active", at: "2026-10-22T09:48:11Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":6}}, executor: "agent:connectors-session", correlation: "wave-20261022"}
---
## Outcome

The definition reader and its lowering accept every feed field the definitions notation gained.
With that, two feeds from `beyond10x/connectors-definitions` main `536945d` (pull request 7) lower
to the catalog engine's feed declarations:
- the Jira issue feed (`connectors/jira.yaml`, `datasources.issues`, profile `jira-issues/1`);
- the Confluence page feed (`connectors/confluence-v1.yaml`, `datasources.pages`, profile
  `confluence-pages/1`).

This story then moves both feeds from the test fixtures into the shipped selections and resolves
upstream-blocker:definitions-notation-lacks-feed-extensions (`open` to `cleared`).

## Fields

The reader today:
- `FeedCursor` takes only `parameter`, `next` and `last` (`crates/connectors-catalog/src/definition.rs:517-526`).
- `FeedPosition` requires `time` (`:547-553`).
- `FeedItems` has no `url`, `url_base` or `parent` (`:572-590`).
- Every struct is `deny_unknown_fields`.

The two feeds at `536945d` use the fields below. The schema is definitions
`spec/domains/definition.yaml` (`FeedOffset` :222, `url_parameter` :231/:269, `FeedUtcOffsets`
:248-252, `url_base` :301).

| Struct | Fields to accept | Used by |
|---|---|---|
| `FeedCursor` | `offset {complete {pointer, equals}}`, `url_parameter` | Jira containers (`offset`), Confluence containers (`url_parameter`) |
| `FeedPosition` | `time` optional; `query {parameter, language, first, resume, utc_offsets {least, greatest}}`; `precision`; `page {parameter, next, url_parameter}` | both feeds' items |
| `FeedItems` | `url`, `url_base` | Confluence |
| `FeedItems` | `parent` | neither feed; accepted as a JSON pointer because the schema names it |

## Rules

The schema states these rules only in its domain summary (`definition.yaml:2`), so the reader
enforces each one:
1. A cursor names exactly one of `next`, `last` and `offset`.
2. `url_parameter` appears only beside `next`.
3. A position names exactly one of `time` and `query`.
4. `first` holds `{container}` exactly once and `{since}` never (the engine's load rule, `adapters/catalog/src/feed.rs:524-526`).
5. `resume` holds `{container}` and `{since}` exactly once each.
6. A query position is at `minute` precision.
7. Items with a query position name no container parameter.
8. `utc_offsets` lies within -1439..1439 and `least` is not above `greatest`.
9. `utc_offsets` needs `page`.
10. `url`, `url_base` and `parent` are JSON pointers.
11. A position's `time`, `query.parameter` and `page.parameter` are query parameters of the items operation (`feed_position_parameter`; the engine binds them at load).

## Ownership

This story owns moving the feeds into the shipped selections:
`adapters/catalog/providers/{jira,confluence-v1}/operations.json` gain their `feed` block.
story:jira-feed-binding and story:confluence-feed-binding depend on this story, and their Scope
names this story for the shipped `feed` block instead of claiming it.

story:definitions-fixture-copy-removed and story:catalog-provider-data-removed depend on this
story. They delete the vendored definitions and `adapters/catalog/providers/` after the feed
blocks are in them; the pinned connectors-definitions release already carries both feeds
(`536945d`).

## Scope

- `crates/connectors-catalog/src/definition.rs`, `import.rs`: the fields and rules above.
- `crates/connectors-catalog/tests/fixtures/definitions/connectors/{jira,confluence-v1}.yaml`:
  refreshed from `536945d`.
- `adapters/catalog/providers/{jira,confluence-v1}/operations.json`: the `feed` block.
- `crates/connectors-build/src/{jira,confluence}_feed_conformance.rs` and
  `adapters/catalog/tests/{jira_feed,confluence_v1_feed}.rs`: read the declaration from the
  shipped selection, not from `tests/feed/*/feed.json` (`jira_feed_conformance.rs:80-86`).
- Claims the outcome makes false:
  - `website/docs/connectors/jira.md:163-164`
  - `website/docs/connectors/confluence.md:131-132`
  - `docs/catalog-jira.md:47-51`
  - `docs/catalog-confluence.md:175-179`
  - `crates/connectors-docs/src/status.rs:309-317`, "a test fixture only and not in the shipped
    selection", regenerated with `connectors-docs generate`
- `CHANGELOG.md`.

## Acceptance (named conformance scenarios)

`crates/connectors-catalog/tests/definition_feed.rs`:
- `the_jira_issue_feed_lowers_to_the_shipped_declaration`: `jira.yaml` at `536945d` lowers to the
  `feed` in `adapters/catalog/providers/jira/operations.json`, equal to the 0.56.0 fixture
  `adapters/catalog/tests/feed/jira/feed.json`.
- `the_confluence_page_feed_lowers_to_the_shipped_declaration`: `confluence-v1.yaml` at `536945d`
  lowers to the `feed` in `adapters/catalog/providers/confluence-v1/operations.json`, equal to the
  0.56.0 fixture `adapters/catalog/tests/feed/confluence/feed.json`.
- One refusal per rule, in order:
  1. `a_cursor_naming_none_of_next_last_offset_is_refused`
  2. `a_cursor_naming_two_of_next_last_offset_is_refused`
  3. `url_parameter_without_next_is_refused`
  4. `a_position_naming_neither_time_nor_query_is_refused`
  5. `a_position_naming_both_time_and_query_is_refused`
  6. `a_first_template_without_one_container_is_refused`
  7. `a_resume_template_without_one_container_and_one_since_is_refused`
  8. `a_query_position_not_at_minute_precision_is_refused`
  9. `items_with_a_query_position_naming_a_container_parameter_is_refused`
  10. `utc_offsets_outside_minus_1439_to_1439_is_refused`
  11. `utc_offsets_least_above_greatest_is_refused`
  12. `utc_offsets_without_page_is_refused`
  13. `a_url_url_base_or_parent_that_is_not_a_pointer_is_refused`

`crates/connectors-catalog/tests/definition_import.rs`:
- `jira_round_trips`, `jira_is_committed`, `confluence_v1_round_trips`,
  `confluence_v1_is_committed` pass at the base, where the shipped selections carry no `feed`.
- With the `feed` blocks in the shipped selections and the reader unchanged, they fail. Observed
  2026-10-22 in the Jira unit's tree: 33 passed, 7 failed, "unknown field `offset`, expected one
  of `parameter`, `next`, `last`"
  (upstream-blocker:definitions-notation-lacks-feed-extensions).
- After this story they pass with the `feed` blocks in place.

Shipped-selection reads:
- `adapters/catalog/tests/jira_feed.rs::the_shipped_jira_selection_declares_no_feed` (:484) and
  `confluence_v1_feed.rs::the_shipped_confluence_v1_selection_declares_no_feed` (:525) pass at
  the base. This story replaces them with
  `the_shipped_jira_selection_carries_the_issue_feed` and
  `the_shipped_confluence_v1_selection_carries_the_page_feed`, which fail at the base.
- `crates/connectors-build/src/jira_feed_conformance.rs::the_jira_feed_binding_passes_the_scenarios_its_profile_declares_in_every_zone`
  (:600) and
  `confluence_feed_conformance.rs::the_confluence_v1_feed_binding_passes_the_scenarios_its_profile_declares_in_every_zone`
  (:679) pass with `declaration()` (`:84`, `:83`) reading the shipped selection.
- Red check: `jira_feed_conformance.rs::the_jira_declaration_comes_from_the_shipped_selection` and
  `confluence_feed_conformance.rs::the_confluence_v1_declaration_comes_from_the_shipped_selection`
  copy the shipped selection without its `feed` block into a temporary provider directory, and
  assert `declaration()` over that directory fails, naming the missing `feed`.

Store: `aep plan artifact show upstream-blocker:definitions-notation-lacks-feed-extensions`
reports the blocker moved from `open` to `cleared`, with this story's merge commit recorded as evidence.
