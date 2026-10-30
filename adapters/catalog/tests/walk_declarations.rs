//! story:catalog-operation-declares-its-walk: a read in a
//! `connectors-catalog-operations/1` selection declares how its pages are walked and when the
//! walk ends (`walk`, modelled as `connectors_catalog.walk.Walk` in `adapters/catalog/spec/ess`).
//! The engine refuses a malformed walk, and a walk on a write, by name when the selection loads;
//! the declaration reaches the operation's admitted description as the input schema's
//! `x-connectors-walk` annotation, which `invoke --describe` prints inside `input_schema`. The
//! evaluator decides each page of the Zendesk fixture's walks as the guide states them.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Effect, Engine, Selection,
    walk::{self, Broken, Step, Walk, Walker},
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn file(provider: &str, name: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers").join(provider).join(name)).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    serde_json::from_value(file["operations"].clone()).unwrap()
}

fn committed(provider: &str) -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), provider).unwrap()
}

fn zendesk() -> Vec<Selection> {
    file("zendesk", "operations.json")
}

/// Every shipped selection file and the reads in it that declare a walk: each a cursor read whose
/// guide (`docs/catalog-<provider>.md`) names its end condition. A walk added or dropped fails
/// here. Confluence's REST v2 `pages.changed` has none: its guide ends it at a cutoff the caller
/// applies to `version.createdAt`, which no declaration can state, so a declared walk would read
/// the whole space. Offset reads whose next start is a sum (Jira's `startAt`, Opsgenie's
/// `offset`) name no position in the answer and have none either.
const WALKS: [(&str, &str, &str, &[&str]); 10] = [
    (
        "zendesk",
        "zendesk",
        "operations.json",
        &[
            "tickets.incremental",
            "ticket.comments",
            "ticket.audits",
            "users.incremental",
            "organizations.incremental",
        ],
    ),
    (
        "google-gmail",
        "google-gmail",
        "operations.json",
        &[
            "users.messages.list",
            "users.threads.list",
            "users.history.list",
        ],
    ),
    (
        "google-calendar",
        "google-calendar",
        "operations.json",
        &["calendarList.list", "events.list"],
    ),
    (
        "google-drive",
        "google-drive",
        "operations.json",
        &["files.list", "changes.list"],
    ),
    (
        "confluence",
        "confluence",
        "operations.json",
        &["space.pages", "page.comments"],
    ),
    (
        "confluence-v1",
        "confluence-v1",
        "operations.json",
        &[
            "spaces.list",
            "space.pages",
            "pages.changed",
            "page.comments",
            "content.search",
        ],
    ),
    ("jira", "jira", "operations.json", &["issues.search"]),
    (
        "slack",
        "slack",
        "operations.json",
        &[
            "conversations.list",
            "conversations.history",
            "conversations.replies",
            "users.list",
        ],
    ),
    (
        "slack",
        "slack",
        "user-operations.json",
        &[
            "conversations.list",
            "conversations.history",
            "conversations.replies",
            "users.list",
        ],
    ),
    ("hubspot", "hubspot", "operations.json", &["objects.list"]),
];

/// Each shipped walk loads, and the operation's admitted description carries it unchanged under
/// `x-connectors-walk` in its input schema, where `--describe` shows it and an invoke reads it
/// beside the schema and revision. The annotated schema still compiles and still closes the input.
#[test]
fn every_shipped_walk_loads_and_reaches_the_admitted_description() {
    for (provider, bundle, name, expected) in WALKS {
        let selections = file(provider, name);
        let mut declared: Vec<&str> = selections
            .iter()
            .filter(|s| s.walk.is_some())
            .map(|s| s.id.as_str())
            .collect();
        declared.sort_unstable();
        let mut wanted = expected.to_vec();
        wanted.sort_unstable();
        assert_eq!(declared, wanted, "{provider}/{name}: the reads with a walk");
        let engine = Engine::new(&committed(bundle), "/", &selections)
            .unwrap_or_else(|error| panic!("{provider}/{name}: {error:?}"));
        let operations = engine.declarations(&[Effect::Read, Effect::Write]);
        for selection in &selections {
            let operation = operations.iter().find(|o| o.id == selection.id).unwrap();
            let carried = Walk::declared(&operation.input_schema)
                .unwrap_or_else(|error| panic!("{provider} `{}`: {error}", selection.id));
            assert_eq!(carried, selection.walk, "{provider} `{}`", selection.id);
            if let Some(walk) = &selection.walk {
                assert_eq!(
                    operation.input_schema[walk::KEYWORD],
                    serde_json::to_value(walk).unwrap(),
                    "{provider} `{}`",
                    selection.id
                );
                assert_eq!(
                    operation.input_schema["additionalProperties"], false,
                    "{provider} `{}`",
                    selection.id
                );
                // The annotation is no assertion: the schema compiles and an
                // unknown member is still refused.
                let error = connectors_sdk::validate(
                    &operation.input_schema,
                    &json!({"not-a-parameter": 1}),
                )
                .unwrap_err();
                assert_eq!(
                    error.code,
                    connectors_core::ErrorCode::InvalidInput,
                    "{provider} `{}`",
                    selection.id
                );
            }
        }
    }
}

/// An operation without a walk carries no annotation, so `Walk::declared` answers `None`.
#[test]
fn a_read_without_a_walk_declares_none() {
    let engine = Engine::new(&committed("zendesk"), "/", &zendesk()).unwrap();
    let show = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == "ticket.show")
        .unwrap();
    assert!(show.input_schema.get(walk::KEYWORD).is_none());
    assert_eq!(Walk::declared(&show.input_schema).unwrap(), None);
}

/// The Zendesk walks, as `docs/catalog-zendesk.md` states them: the comments and audits send
/// `meta.after_cursor` as `page[after]`, `meta.has_more` decides when present, and without it no
/// `meta.after_cursor` or no records ends the walk; the audits send
/// `include_boundary_indicators: true` on every page. The cursor exports send `start_time` on the
/// first page only and `after_cursor` as `cursor`; the organization export sends `end_time` as
/// `start_time`; all three end on `end_of_stream: true`.
#[test]
fn zendesk_walks_are_declared_as_the_guide_states() {
    let selections = zendesk();
    let walk = |id: &str| {
        serde_json::to_value(
            selections
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .walk
                .as_ref()
                .unwrap(),
        )
        .unwrap()
    };
    let paged = |records: &str| {
        json!({
            "parameter": "page[after]",
            "next": "/meta/after_cursor",
            "records": records,
            "continues": {"pointer": "/meta/has_more", "equals": true},
            "ends": [
                {"name": "has_more_false", "when": {"pointer": "/meta/has_more", "equals": false}},
                {"name": "no_cursor", "absent": "/meta/after_cursor"},
                {"name": "no_records", "empty": records}
            ]
        })
    };
    assert_eq!(walk("ticket.comments"), paged("/comments"));
    let mut audits = paged("/audits");
    audits["query"] = json!({"include_boundary_indicators": true});
    assert_eq!(walk("ticket.audits"), audits);
    let end_of_stream =
        json!([{"name": "end_of_stream", "when": {"pointer": "/end_of_stream", "equals": true}}]);
    for (id, records) in [
        ("tickets.incremental", "/tickets"),
        ("users.incremental", "/users"),
    ] {
        assert_eq!(
            walk(id),
            json!({"parameter": "cursor", "next": "/after_cursor", "records": records,
                   "first_only": ["start_time"], "ends": end_of_stream}),
            "`{id}`"
        );
    }
    assert_eq!(
        walk("organizations.incremental"),
        json!({"parameter": "start_time", "next": "/end_time", "records": "/organizations",
               "ends": end_of_stream})
    );
}

fn audits() -> Value {
    serde_json::to_value(
        zendesk()
            .into_iter()
            .find(|s| s.id == "ticket.audits")
            .unwrap(),
    )
    .unwrap()
}

/// The engine's answer to the Zendesk selection file with `ticket.audits` replaced by `changed`.
fn load_audits(changed: Value) -> Result<Engine, String> {
    let mut selections = zendesk();
    let at = selections
        .iter()
        .position(|s| s.id == "ticket.audits")
        .unwrap();
    selections[at] = serde_json::from_value(changed).map_err(|error| error.to_string())?;
    Engine::new(&committed("zendesk"), "/", &selections).map_err(|error| error.message)
}

/// A change to the walk of `ticket.audits`.
type Change = Box<dyn Fn(&mut Value)>;

/// Each malformed walk is refused when the selection loads, the message naming the selection and
/// the member at fault.
#[test]
fn a_malformed_walk_is_refused_by_name_when_the_selection_loads() {
    let cases: Vec<(&str, Change, &str)> = vec![
        (
            "a next that is not a pointer",
            Box::new(|w| w["next"] = json!("meta/after_cursor")),
            "walk `next` is not a JSON pointer",
        ),
        (
            "an empty next",
            Box::new(|w| w["next"] = json!("")),
            "walk `next` is not a JSON pointer",
        ),
        (
            "a records pointer with a bad escape",
            Box::new(|w| w["records"] = json!("/audits~2")),
            "walk `records` is not a JSON pointer",
        ),
        (
            "an end pointer that does not parse",
            Box::new(|w| w["ends"][1]["absent"] = json!("meta")),
            "walk end `no_cursor` `absent` is not a JSON pointer",
        ),
        (
            "a continues pointer that does not parse",
            Box::new(|w| w["continues"]["pointer"] = json!("has_more")),
            "walk `continues` is not a JSON pointer",
        ),
        (
            "a parameter the operation does not declare",
            Box::new(|w| w["parameter"] = json!("page[later]")),
            "walk `parameter` `page[later]` is not a query parameter of its operation",
        ),
        (
            "a path parameter",
            Box::new(|w| w["parameter"] = json!("ticket_id")),
            "walk `parameter` `ticket_id` is not a query parameter of its operation",
        ),
        (
            "a withheld parameter",
            Box::new(|w| w["parameter"] = json!("sort_order")),
            "walk `parameter` `sort_order` is not a query parameter of its operation",
        ),
        (
            "a fixed query value the operation does not declare",
            Box::new(|w| w["query"] = json!({"sort_order": "asc"})),
            "walk `query` `sort_order` is not a query parameter of its operation",
        ),
        (
            "a fixed query value that is not a scalar",
            Box::new(|w| w["query"] = json!({"include_boundary_indicators": [true]})),
            "walk `query` `include_boundary_indicators` is not a string, an integer or a boolean",
        ),
        (
            "a fixed query value on the parameter",
            Box::new(|w| w["query"] = json!({"page[after]": "x"})),
            "walk `query` `page[after]` is the walk's own `parameter`",
        ),
        (
            "a first-only parameter the operation does not declare",
            Box::new(|w| w["first_only"] = json!(["start_time"])),
            "walk `first_only` `start_time` is not a query parameter of its operation",
        ),
        (
            "a first-only parameter that is the walk's own",
            Box::new(|w| w["first_only"] = json!(["page[after]"])),
            "walk `first_only` `page[after]` is the walk's own `parameter` or fixed in its `query`",
        ),
        (
            "an empty ends",
            Box::new(|w| w["ends"] = json!([])),
            "walk `ends` holds no end",
        ),
        (
            "more than eight ends",
            Box::new(|w| {
                w["ends"] = Value::Array(
                    (0..9)
                        .map(|n| json!({"name": format!("end_{n}"), "absent": "/meta"}))
                        .collect(),
                )
            }),
            "walk `ends` holds more than 8 ends",
        ),
        (
            "an end that is not a name",
            Box::new(|w| w["ends"][0]["name"] = json!("Has More")),
            "walk end `Has More` is not a name",
        ),
        (
            "an end named twice",
            Box::new(|w| w["ends"][1]["name"] = json!("has_more_false")),
            "walk names end `has_more_false` twice",
        ),
        (
            "an end with two tests",
            Box::new(|w| w["ends"][1]["empty"] = json!("/audits")),
            "walk end `no_cursor` takes exactly one of `when`, `absent`, `present` and `empty`",
        ),
        (
            "an end with no test",
            Box::new(|w| w["ends"][2] = json!({"name": "no_records"})),
            "walk end `no_records` takes exactly one of `when`, `absent`, `present` and `empty`",
        ),
        (
            "an empty url parameter",
            Box::new(|w| w["url_parameter"] = json!("")),
            "walk `url_parameter` is not 1 to 128 visible ASCII characters",
        ),
    ];
    for (case, change, expected) in cases {
        let mut selection = audits();
        change(&mut selection["walk"]);
        let refusal = load_audits(selection)
            .err()
            .unwrap_or_else(|| panic!("{case}: the selection loads"));
        assert_eq!(
            refusal,
            format!("selection `ticket.audits` {expected}"),
            "{case}"
        );
    }
    // An unknown member, at the walk or in an end, is no walk at all.
    for (case, change) in [
        (
            "a walk member",
            Box::new(|w: &mut Value| w["stop"] = json!(true)) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "an end member",
            Box::new(|w: &mut Value| w["ends"][0]["or"] = json!("/x")),
        ),
    ] {
        let mut selection = audits();
        change(&mut selection["walk"]);
        let refusal = load_audits(selection)
            .err()
            .unwrap_or_else(|| panic!("{case}: the selection loads"));
        assert!(refusal.contains("unknown field"), "{case}: {refusal}");
    }
}

/// A walk is a read's: on a write, and on a read that does not answer JSON, it is refused when
/// the selection loads.
#[test]
fn a_walk_on_a_write_or_on_a_read_that_does_not_answer_json_is_refused() {
    let walk = audits()["walk"].clone();
    let mut selections = zendesk();
    let at = selections
        .iter()
        .position(|s| s.id == "ticket.update")
        .unwrap();
    let mut update = serde_json::to_value(&selections[at]).unwrap();
    update["walk"] = walk.clone();
    selections[at] = serde_json::from_value(update).unwrap();
    let refusal = Engine::new(&committed("zendesk"), "/", &selections)
        .err()
        .expect("a write with a walk loads");
    assert_eq!(
        refusal.message,
        "selection `ticket.update` declares a walk on a write"
    );

    let mut selections = file("google-drive", "operations.json");
    let at = selections
        .iter()
        .position(|s| s.id == "files.export")
        .unwrap();
    let mut export = serde_json::to_value(&selections[at]).unwrap();
    export["walk"] = json!({"parameter": "mimeType", "next": "/next", "records": "/items",
                            "ends": [{"name": "no_next", "absent": "/next"}]});
    selections[at] = serde_json::from_value(export).unwrap();
    let refusal = Engine::new(&committed("google-drive"), "/", &selections)
        .err()
        .expect("a text read with a walk loads");
    assert_eq!(
        refusal.message,
        "selection `files.export` declares a walk on a read that does not answer JSON"
    );
}

/// A description whose annotation is not a walk is refused by name, so an invoke never walks by
/// a declaration it cannot read.
#[test]
fn a_described_walk_that_is_not_one_is_refused_by_name() {
    let error =
        Walk::declared(&json!({"type": "object", walk::KEYWORD: {"parameter": "p"}})).unwrap_err();
    assert!(error.contains("missing field"), "{error}");
    let mut walk = audits()["walk"].clone();
    walk["ends"] = json!([]);
    let error = Walk::declared(&json!({"type": "object", walk::KEYWORD: walk})).unwrap_err();
    assert_eq!(error, "walk `ends` holds no end");
}

fn zendesk_walk(id: &str) -> Walk {
    zendesk()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap()
        .walk
        .unwrap()
}

/// The Zendesk fixture's pages (`tests/zendesk.rs`, `page`), decided page by page: the walk goes
/// on to the next position, ends by the name of the first end that holds, or is broken by name.
#[test]
fn the_evaluator_decides_the_zendesk_fixture_pages() {
    // Comments: two pages, `has_more` false on the second.
    let comments = zendesk_walk("ticket.comments");
    let mut walker = Walker::new(&comments);
    let first = json!({"comments": [{"id": 1}, {"id": 2}],
        "meta": {"has_more": true, "after_cursor": "fixture-comment-cursor-2",
                 "before_cursor": "fixture-comment-cursor-1"}});
    assert_eq!(comments.records(&first).unwrap().len(), 2);
    assert_eq!(
        walker.step(&first),
        Ok(Step::Next(json!("fixture-comment-cursor-2")))
    );
    let last = json!({"comments": [{"id": 3}],
        "meta": {"has_more": false, "after_cursor": "fixture-comment-cursor-3",
                 "before_cursor": "fixture-comment-cursor-2"}});
    assert_eq!(walker.step(&last), Ok(Step::End("has_more_false".into())));

    let audits = zendesk_walk("ticket.audits");
    // With the indicator: `has_more` false ends the walk though a cursor is named.
    let mut walker = Walker::new(&audits);
    assert_eq!(
        walker.step(&json!({"audits": [{}, {}],
            "meta": {"has_more": true, "after_cursor": "fixture-audit-cursor-2"}})),
        Ok(Step::Next(json!("fixture-audit-cursor-2")))
    );
    assert_eq!(
        walker.step(&json!({"audits": [{}],
            "meta": {"has_more": false, "after_cursor": "fixture-audit-cursor-3"}})),
        Ok(Step::End("has_more_false".into()))
    );
    // Without it: the last page's null cursor ends the walk.
    let mut walker = Walker::new(&audits);
    assert_eq!(
        walker
            .step(&json!({"audits": [{}, {}], "meta": {"after_cursor": "fixture-audit-cursor-2"}})),
        Ok(Step::Next(json!("fixture-audit-cursor-2")))
    );
    assert_eq!(
        walker.step(&json!({"audits": [{}], "meta": {"after_cursor": null}})),
        Ok(Step::End("no_cursor".into()))
    );
    // Ticket 104: `has_more` true continues past an empty page.
    let mut walker = Walker::new(&audits);
    assert_eq!(
        walker.step(&json!({"audits": [{}], "meta": {"has_more": true, "after_cursor": "fixture-audit-cursor-a"}})),
        Ok(Step::Next(json!("fixture-audit-cursor-a")))
    );
    assert_eq!(
        walker.step(&json!({"audits": [], "meta": {"has_more": true, "after_cursor": "fixture-audit-cursor-b"}})),
        Ok(Step::Next(json!("fixture-audit-cursor-b")))
    );
    assert_eq!(
        walker.step(&json!({"audits": [{}], "meta": {"has_more": false, "after_cursor": "fixture-audit-cursor-c"}})),
        Ok(Step::End("has_more_false".into()))
    );
    // Ticket 105: `has_more` true and no cursor.
    assert_eq!(
        Walker::new(&audits)
            .step(&json!({"audits": [{}], "meta": {"has_more": true, "before_cursor": null}})),
        Err(Broken::ContinuesWithoutPosition)
    );
    // Ticket 106: the page names the cursor that fetched it again.
    let mut walker = Walker::new(&audits);
    let looping = json!({"audits": [{}], "meta": {"has_more": true, "after_cursor": "fixture-audit-cursor-r"}});
    assert_eq!(
        walker.step(&looping),
        Ok(Step::Next(json!("fixture-audit-cursor-r")))
    );
    assert_eq!(
        walker.step(&looping),
        Err(Broken::RepeatedPosition(json!("fixture-audit-cursor-r")))
    );
    // Ticket 103: an empty page without `has_more` ends the walk though it names a cursor.
    let mut walker = Walker::new(&audits);
    assert_eq!(
        walker.step(&json!({"audits": [{}], "meta": {"after_cursor": "fixture-audit-cursor-e"}})),
        Ok(Step::Next(json!("fixture-audit-cursor-e")))
    );
    assert_eq!(
        walker.step(&json!({"audits": [], "meta": {"after_cursor": "fixture-audit-cursor-f"}})),
        Ok(Step::End("no_records".into()))
    );
    // Records that are not a list break the walk.
    assert_eq!(
        audits.records(&json!({"audits": {"id": 1}})),
        Err(Broken::RecordsNotAList)
    );

    // The ticket export: `after_cursor` until `end_of_stream`.
    let tickets = zendesk_walk("tickets.incremental");
    let mut walker = Walker::new(&tickets);
    assert_eq!(
        walker.step(
            &json!({"tickets": [{}, {}], "after_cursor": "fixture-ticket-cursor-2",
            "before_cursor": null, "end_of_stream": false})
        ),
        Ok(Step::Next(json!("fixture-ticket-cursor-2")))
    );
    assert_eq!(
        walker.step(
            &json!({"tickets": [{}], "after_cursor": "fixture-ticket-cursor-3",
            "before_cursor": "fixture-ticket-cursor-2", "end_of_stream": true})
        ),
        Ok(Step::End("end_of_stream".into()))
    );
    // A page that neither ends the walk nor names a next position is broken.
    assert_eq!(
        Walker::new(&tickets).step(&json!({"tickets": [], "end_of_stream": false})),
        Err(Broken::NoPosition)
    );
    // The organization export: `end_time`, an integer, is the next `start_time`.
    let organizations = zendesk_walk("organizations.incremental");
    let mut walker = Walker::new(&organizations);
    assert_eq!(
        walker.step(
            &json!({"organizations": [{}, {}], "count": 2, "end_of_stream": false,
            "end_time": 1_758_003_600u64})
        ),
        Ok(Step::Next(json!(1_758_003_600u64)))
    );
    assert_eq!(
        walker.step(
            &json!({"organizations": [{}], "count": 1, "end_of_stream": true,
            "end_time": 1_758_007_200u64})
        ),
        Ok(Step::End("end_of_stream".into()))
    );
}

/// A position read from a next link is the one parameter the declaration names, decoded; a link
/// without it breaks the walk.
#[test]
fn a_walk_with_a_url_parameter_reads_the_position_from_the_link() {
    let walk: Walk = serde_json::from_value(json!({
        "parameter": "cursor", "next": "/_links/next", "url_parameter": "cursor",
        "records": "/results", "ends": [{"name": "no_next", "absent": "/_links/next"}]}))
    .unwrap();
    walk.check().unwrap();
    let mut walker = Walker::new(&walk);
    assert_eq!(
        walker.step(&json!({"results": [{}],
            "_links": {"next": "/wiki/api/v2/pages?cursor=abc%3D%3D&limit=2"}})),
        Ok(Step::Next(json!("abc==")))
    );
    assert_eq!(
        walker.step(&json!({"results": [{}], "_links": {"next": "/wiki/api/v2/pages?limit=2"}})),
        Err(Broken::NoPosition)
    );
    assert_eq!(
        Walker::new(&walk).step(&json!({"results": [], "_links": {}})),
        Ok(Step::End("no_next".into()))
    );
}

/// Each page's input: the caller's, the walk's fixed `query` on every page, and from the second
/// page on the position as `parameter`, without the `first_only` parameters.
#[test]
fn each_page_input_carries_the_query_and_the_position() {
    let audits = zendesk_walk("ticket.audits");
    let caller = json!({"ticket_id": 101, "page[size]": 2});
    assert_eq!(
        audits.page_input(&caller, None),
        json!({"ticket_id": 101, "page[size]": 2, "include_boundary_indicators": true})
    );
    assert_eq!(
        audits.page_input(&caller, Some(&json!("fixture-audit-cursor-2"))),
        json!({"ticket_id": 101, "page[size]": 2, "include_boundary_indicators": true,
               "page[after]": "fixture-audit-cursor-2"})
    );
    let tickets = zendesk_walk("tickets.incremental");
    let caller = json!({"start_time": 1_758_000_000u64, "per_page": 2});
    assert_eq!(tickets.page_input(&caller, None), caller);
    assert_eq!(
        tickets.page_input(&caller, Some(&json!("fixture-ticket-cursor-2"))),
        json!({"per_page": 2, "cursor": "fixture-ticket-cursor-2"})
    );
    let organizations = zendesk_walk("organizations.incremental");
    assert_eq!(
        organizations.page_input(
            &json!({"start_time": 1_758_000_000u64, "per_page": 2}),
            Some(&json!(1_758_003_600u64))
        ),
        json!({"start_time": 1_758_003_600u64, "per_page": 2})
    );
}

/// Each broken walk has a stable name, for the summary that reports it.
#[test]
fn a_broken_walk_is_named() {
    assert_eq!(
        Broken::ContinuesWithoutPosition.name(),
        "continues_without_position"
    );
    assert_eq!(Broken::NoPosition.name(), "no_position");
    assert_eq!(
        Broken::RepeatedPosition(json!("x")).name(),
        "repeated_position"
    );
    assert_eq!(Broken::RecordsNotAList.name(), "records_not_a_list");
    assert_eq!(Broken::InvalidPosition.name(), "invalid_position");
}
