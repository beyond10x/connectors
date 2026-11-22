---
format: aep.planning-md/3
id: review-result:wave-b-enum-date-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on catalog enum and date-time checks
relations:
- reviews: story:catalog-validates-enum-and-date-time
revision: 1
---
## Adversary pass 1 on story:catalog-validates-enum-and-date-time

Reviewed commit 4110a06f40 on impl/catalog-validates-enum-and-date-time (base 7988b0427). Model: opus.

```
verdict: red
cases: executed 41→45, red 3
origin: introduced 3, pre-existing 0, undecided 1
```

New uncommitted test file `adapters/catalog/tests/enum_date_time_adversary.rs`:

- `adversary_confluence_enum_behind_a_schema_ref_is_refused` (red): on the shipped Confluence
  bundle `pages.changed {"sort":"newest"}`, `pages.changed {"body-format":"html"}` and
  `page.get {"id":"1001","body-format":"html"}` are still sent. Their enums come through `$ref`
  schemas (`PageSortOrder`, `PrimaryBodyRepresentation`, `CommentSortOrder`); `constraints()`
  never follows a `$ref`. 78 Confluence parameters are like this, 7 in selected operations.
- `adversary_comma_joined_repeated_enum_of_allowed_elements_is_sent` (red): `{"modes":"x,y"}` is
  refused although the guide accepts one comma-joined value for a repeated parameter and both
  elements are in the enum; the check compares the whole joined text.
- `adversary_declared_input_schema_lists_the_pinned_enum` (red): the input schema from
  `operations describe` publishes neither the pinned enum nor the date-time format.
- `adversary_probes_that_hold` (green): 26 refused, 5 sent as expected.

Held: the RFC 3339 parser (offsets `+23:60`/`+24:00`, empty fraction, trailing bytes, `+0200`,
non-leap Feb 29, April 31, month/day 00, minute 60, second 61, non-ASCII digits, leading `+`,
`ZZ`); integer enum given as `1.0` or `"01"` refused; explicit `null` refused as before;
deepObject `not[milestone_id]` reaches the check; writes share `parameter_values`; the chatgpt,
claude-subscription, hubspot and slack bundles are byte-identical.

Also found: a stale guide row (`deployments.list` still says ISO 8601) and a second hand-written
RFC 3339 reader beside the existing `feed::instant`.

```findings
[{"file":"crates/connectors-catalog/src/inventory.rs","line":177,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"undecided","message":"constraints() never follows a schema $ref, so Confluence sort/body-format enums on four shipped selections are not recorded and pages.changed sort=newest and page.get body-format=html are still sent (enum_date_time_adversary.rs adversary_confluence_enum_behind_a_schema_ref_is_refused)."},
{"file":"adapters/catalog/src/lib.rs","line":1063,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"check_declared compares a comma-joined value for a repeated parameter as one string, so modes=\"x,y\" with both elements in the enum is refused, against the guide's accepted comma-joined form and the unit's own per-element claim."},
{"file":"adapters/catalog/src/lib.rs","line":2828,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"declare() publishes neither the pinned enum nor the date-time format, so the declared input schema callers get from describe accepts values the engine now refuses."},
{"file":"docs/local-catalog-provider.md","line":269,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The deployments.list row still says its date filters are strings GitLab reads as ISO 8601, but the engine now refuses anything that is not RFC 3339."},
{"file":"adapters/catalog/src/lib.rs","line":1098,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"is_rfc3339_date_time repeats the rules of the existing feed::instant (feed.rs:1978); two hand-written RFC 3339 readers in one crate can drift apart."}]
```
