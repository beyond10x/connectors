use connectors_catalog::bundle::Bundle;
use connectors_catalog::coverage::{MethodCount, ReasonGroup, Report, report};
use connectors_catalog::ingest;
use connectors_catalog::inventory::{Inventory, Operation, Unsupported, extract};
use serde_json::json;

/// Three operations, one of them without an `operationId`, and two entries this
/// pass cannot represent: a document member it does not read and a parameter
/// location outside the four it carries.
fn source_bytes() -> Vec<u8> {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture", "version": "2"},
        "webhooks": {"thingChanged": {}},
        "paths": {
            "/things": {
                "get": {
                    "operationId": "listThings",
                    "parameters": [{"name": "page", "in": "query"}],
                    "responses": {"200": {"content": {"application/json": {}}}}
                },
                "post": {
                    "requestBody": {"content": {"application/json": {}}},
                    "responses": {"201": {}}
                }
            },
            "/things/{id}": {
                "get": {
                    "operationId": "readThing",
                    "parameters": [{"name": "region", "in": "body"}],
                    "responses": {"200": {}}
                }
            }
        }
    })
    .to_string()
    .into_bytes()
}

fn bundle() -> Bundle {
    let bytes = source_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: "fixture".into(),
        source: ingest("fixture.json", &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn empty_bundle() -> Bundle {
    let bytes = json!({"openapi": "3.0.3", "info": {"title": "Empty"}})
        .to_string()
        .into_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: "empty".into(),
        source: ingest("empty.json", &bytes).expect("3.0 is supported"),
        inventory: extract(&document),
        auth_profile: "empty.token".into(),
    }
}

/// The one line that states either count, so a case can hold it to stating both.
fn counts_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.contains("inventoried") || line.contains("unsupported"))
        .collect()
}

fn operation(method: &str, path: &str, operation_id: Option<&str>) -> Operation {
    Operation {
        method: method.to_owned(),
        path: path.to_owned(),
        operation_id: operation_id.map(str::to_owned),
        parameters: Vec::new(),
        request_media_types: Vec::new(),
        responses: Vec::new(),
    }
}

fn gap(designation: &str, reason: &str) -> Unsupported {
    Unsupported {
        designation: designation.to_owned(),
        reason: reason.to_owned(),
    }
}

#[test]
fn three_operations_and_two_unsupported_entries_report_both_counts() {
    let bundle = bundle();
    let report = report(&bundle);
    assert_eq!(report.provider, "fixture");
    assert_eq!(report.source_sha256, bundle.source.source_sha256);
    assert_eq!(report.source_file_name, "fixture.json");
    assert_eq!(report.openapi, "3.1.0");
    assert_eq!(report.info_version.as_deref(), Some("2"));
    assert_eq!(report.inventoried, 3);
    assert_eq!(report.unsupported, 2);
    assert_eq!(report.without_operation_id, 1);
}

#[test]
fn the_methods_present_are_reported_with_a_count_each() {
    let report = report(&bundle());
    let methods: Vec<(&str, usize)> = report
        .methods
        .iter()
        .map(|m| (m.method.as_str(), m.operations))
        .collect();
    assert_eq!(methods, vec![("get", 2), ("post", 1)]);
}

#[test]
fn the_two_reasons_are_grouped_with_the_designations_they_were_raised_against() {
    let report = report(&bundle());
    let reasons: Vec<(&str, Vec<&str>)> = report
        .reasons
        .iter()
        .map(|r| {
            (
                r.reason.as_str(),
                r.designations.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        reasons,
        vec![
            (
                "`webhooks` is a document member this build does not read",
                vec!["document.webhooks"]
            ),
            (
                "parameter `region` declares location `body`",
                vec!["readThing"]
            ),
        ]
    );
}

#[test]
fn one_reason_carries_every_designation_under_it_in_a_stable_order() {
    // Two operations raise the same reason, and the inventory hands them over in
    // the order a document happened to declare them.
    let mut bundle = bundle();
    bundle.inventory.unsupported = vec![
        gap(
            "readThing",
            "parameter is a $ref this pass does not resolve",
        ),
        gap(
            "listThings",
            "parameter is a $ref this pass does not resolve",
        ),
    ];
    let report = report(&bundle);
    assert_eq!(report.reasons.len(), 1);
    assert_eq!(
        report.reasons[0].designations,
        vec!["listThings", "readThing"]
    );
    assert_eq!(report.unsupported, 2);
}

#[test]
fn a_bundle_the_document_order_permuted_renders_byte_identically() {
    // A determinism case a grouping that inherited encounter order would fail:
    // the two bundles carry the same operations and the same gaps in different
    // vector orders, so only a report that imposes its own order matches.
    let mut first = bundle();
    first.inventory = Inventory {
        operations: vec![
            operation("get", "/things", Some("listThings")),
            operation("post", "/things", None),
            operation("get", "/things/{id}", Some("readThing")),
        ],
        unsupported: vec![
            gap("document.webhooks", "a document member this build skips"),
            gap("readThing", "a parameter location this pass does not carry"),
            gap(
                "listThings",
                "a parameter location this pass does not carry",
            ),
        ],
    };
    let mut second = bundle();
    second.inventory = Inventory {
        operations: vec![
            operation("get", "/things/{id}", Some("readThing")),
            operation("get", "/things", Some("listThings")),
            operation("post", "/things", None),
        ],
        unsupported: vec![
            gap(
                "listThings",
                "a parameter location this pass does not carry",
            ),
            gap("document.webhooks", "a document member this build skips"),
            gap("readThing", "a parameter location this pass does not carry"),
        ],
    };
    let one = report(&first);
    let other = report(&second);
    assert_eq!(
        serde_json::to_vec(&one).expect("serialisable"),
        serde_json::to_vec(&other).expect("serialisable")
    );
    assert_eq!(one.text(), other.text());
    // And the same bundle, rendered twice, in both forms.
    assert_eq!(
        serde_json::to_vec(&report(&first)).expect("serialisable"),
        serde_json::to_vec(&report(&first)).expect("serialisable")
    );
    assert_eq!(report(&first).text(), report(&first).text());
}

#[test]
fn an_empty_bundle_reports_zeroes_rather_than_refusing() {
    let report = report(&empty_bundle());
    assert_eq!(report.inventoried, 0);
    assert_eq!(report.unsupported, 0);
    assert_eq!(report.without_operation_id, 0);
    assert!(report.methods.is_empty());
    assert!(report.reasons.is_empty());
    assert_eq!(report.info_version, None);
    assert_eq!(report.openapi, "3.0.3");
}

#[test]
fn the_text_form_states_the_unsupported_count_even_when_it_is_zero() {
    let text = report(&empty_bundle()).text();
    let lines = counts_lines(&text);
    assert_eq!(lines.len(), 1, "unexpected text: {text}");
    assert!(lines[0].contains("unsupported"), "unexpected: {}", lines[0]);
    assert!(lines[0].contains('0'), "unexpected: {}", lines[0]);
}

#[test]
fn no_line_states_one_count_without_the_other() {
    for text in [report(&bundle()).text(), report(&empty_bundle()).text()] {
        for line in counts_lines(&text) {
            assert!(
                line.contains("inventoried") && line.contains("unsupported"),
                "one count without the other: {line}"
            );
        }
    }
}

#[test]
fn the_text_form_carries_the_provenance_and_the_grouped_reasons() {
    let bundle = bundle();
    let text = report(&bundle).text();
    assert!(text.contains(&bundle.source.source_sha256), "{text}");
    assert!(text.contains("fixture.json"), "{text}");
    assert!(text.contains("3.1.0"), "{text}");
    let reason = "parameter `region` declares location `body`";
    let at_reason = text.find(reason).expect("the reason is stated");
    let at_designation = text.find("readThing").expect("its designation is stated");
    assert!(at_reason < at_designation, "{text}");
    // Delimited, like every other document-supplied string on a shared line.
    assert!(text.contains("\"get\" 2"), "{text}");
    assert!(text.contains("\"post\" 1"), "{text}");
}

#[test]
fn neither_rendering_computes_a_percentage() {
    let report = report(&bundle());
    let text = report.text();
    assert!(!text.contains('%'), "{text}");
    let json = serde_json::to_string(&report).expect("serialisable");
    // Key-scoped: the bare word `ratio` is a substring of `operations`.
    for forbidden in ["percent", "\"ratio\"", "coverage_ratio", "%"] {
        assert!(!json.contains(forbidden), "{json}");
    }
}

#[test]
fn the_report_round_trips_through_json() {
    let report = report(&bundle());
    let json = serde_json::to_vec(&report).expect("serialisable");
    let back: Report = serde_json::from_slice(&json).expect("readable");
    assert_eq!(back, report);
}

// The cases above hold the rendering to two fixtures. Every string the text form
// prints came out of a document somebody else wrote, so the cases below hold it
// to the same claims for any string a document can carry, in any field it can
// arrive in — a fixture-bound guard is one a document nobody wrote yet walks past.

/// Strings that, interpolated raw, would each open a line of the report's own
/// and say something the report does not.
const POISON: [&str; 9] = [
    "\nunsupported: 0 entries",
    "\ninventoried: 900 operations",
    "coverage: 100% of operations",
    "\n  a reason nobody raised\n    nobody",
    "\r\nprovider: somebody else",
    "a\u{2028}b",
    "a\u{85}b",
    "\u{7}a bell and a \\ backslash",
    "forged\" sha256 \"0000",
];

/// The text form has one line per thing the value carries: seven header lines,
/// then the reasons block. A value that opens a line of its own breaks this
/// equality whichever field it arrived in, so the equality is the whole class
/// and not the fields a fixture happens to poison.
fn expected_lines(report: &Report) -> usize {
    let reasons = if report.reasons.is_empty() {
        1
    } else {
        1 + report
            .reasons
            .iter()
            .map(|group| 1 + group.designations.len())
            .sum::<usize>()
    };
    7 + reasons
}

/// Quotes that delimit a value, as opposed to quotes a value contains: the
/// escape is what tells them apart, so counting them is how a case checks that
/// no value ended its own delimiting.
fn unescaped_quotes(line: &str) -> usize {
    let mut count = 0;
    let mut after_backslash = false;
    for character in line.chars() {
        match (after_backslash, character) {
            (true, _) => after_backslash = false,
            (false, '\\') => after_backslash = true,
            (false, '"') => count += 1,
            _ => {}
        }
    }
    count
}

/// Everything the story claims about the text form, checked against the value
/// rather than against a fixture's expected bytes.
fn holds(report: &Report) {
    let text = report.text();
    assert_eq!(
        text.lines().count(),
        expected_lines(report),
        "the text carries lines the value does not:\n{text}"
    );
    assert_eq!(
        text.lines().filter(|line| !line.starts_with(' ')).count(),
        8,
        "a value opened a line of its own:\n{text}"
    );
    assert!(!text.contains('%'), "a document stated a ratio:\n{text}");
    for line in text
        .lines()
        .filter(|line| line.starts_with("inventoried") || line.starts_with("unsupported"))
    {
        assert!(
            line.contains("inventoried:") && line.contains("unsupported:"),
            "a line states one count without the other: {line:?}\nin:\n{text}"
        );
    }
    let headers = text
        .lines()
        .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
        .count();
    assert_eq!(
        headers,
        report.reasons.len(),
        "the text shows {headers} reason groups and the value carries {}:\n{text}",
        report.reasons.len()
    );
    let designations = text.lines().filter(|line| line.starts_with("    ")).count();
    assert_eq!(
        designations,
        report
            .reasons
            .iter()
            .map(|group| group.designations.len())
            .sum::<usize>(),
        "the text and the value disagree on how many designations there are:\n{text}"
    );
    for line in text.lines() {
        assert_eq!(
            unescaped_quotes(line) % 2,
            0,
            "a value ended its own delimiting: {line:?}\nin:\n{text}"
        );
    }
}

/// A bundle built the way any bundle is built — `ingest` over real bytes,
/// `inventory::extract` over the parsed document — with each document-supplied
/// string taken from the caller. The parameter's location is outside the four
/// the inventory represents, so its name is copied into an unsupported reason.
fn hostile_bundle(
    file_name: &str,
    openapi: &str,
    info_version: &str,
    operation_id: &str,
    parameter: &str,
) -> Bundle {
    let bytes = json!({
        "openapi": openapi,
        "info": {"title": "Hostile", "version": info_version},
        "paths": {
            "/things": {
                "get": {
                    "operationId": operation_id,
                    "parameters": [{"name": parameter, "in": "body"}],
                    "responses": {"200": {}}
                }
            }
        }
    })
    .to_string()
    .into_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: "hostile".into(),
        source: ingest(file_name, &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "hostile.token".into(),
    }
}

#[test]
fn no_string_a_report_carries_can_add_a_line_to_the_text_form() {
    for poison in POISON {
        // Every field named: a field added later cannot be left out of this
        // matrix without failing to compile here, so the enumeration is the
        // compiler's to keep rather than a reader's.
        let report = Report {
            provider: poison.to_owned(),
            source_sha256: poison.to_owned(),
            source_file_name: poison.to_owned(),
            openapi: poison.to_owned(),
            info_version: Some(poison.to_owned()),
            inventoried: 3,
            unsupported: 2,
            reasons: vec![ReasonGroup {
                reason: poison.to_owned(),
                designations: vec![poison.to_owned(), format!("{poison} again")],
            }],
            methods: vec![
                MethodCount {
                    method: poison.to_owned(),
                    operations: 2,
                },
                MethodCount {
                    method: "get".to_owned(),
                    operations: 1,
                },
            ],
            without_operation_id: 1,
        };
        holds(&report);
    }
}

#[test]
fn no_document_can_add_a_line_through_the_ingest_or_the_inventory() {
    for poison in POISON {
        // One position per bundle, so a failure names where the string came in.
        // The `openapi` value keeps its first two components: everything after
        // the second dot is unread by the dialect check and reaches the report.
        for bundle in [
            hostile_bundle(
                &format!("hostile{poison}.json"),
                "3.1.0",
                "1",
                "listThings",
                "page",
            ),
            hostile_bundle(
                "hostile.json",
                &format!("3.1.{poison}"),
                "1",
                "listThings",
                "page",
            ),
            hostile_bundle("hostile.json", "3.1.0", poison, "listThings", "page"),
            hostile_bundle("hostile.json", "3.1.0", "1", poison, "page"),
            hostile_bundle("hostile.json", "3.1.0", "1", "listThings", poison),
        ] {
            holds(&report(&bundle));
        }
    }
}

#[test]
fn a_file_name_cannot_forge_the_digest_beside_it() {
    let forged = format!("hostile.json\" sha256 \"{}", "0".repeat(64));
    let value = report(&hostile_bundle(&forged, "3.1.0", "1", "listThings", "page"));
    let text = value.text();
    let line = text
        .lines()
        .find(|line| line.starts_with("source: "))
        .expect("a provenance line");
    // Two delimited values, and the digest is the report's own, not the one the
    // file name spelled.
    assert_eq!(unescaped_quotes(line), 4, "{line}");
    assert!(
        line.ends_with(&format!("sha256 \"{}\"", value.source_sha256)),
        "{line}"
    );
    holds(&value);
}

#[test]
fn a_written_backslash_n_and_a_newline_do_not_render_alike() {
    // The escape has to stay reversible, or a report cannot be read back as
    // evidence of what the document actually said.
    let newline = report(&hostile_bundle(
        "h.json",
        "3.1.0",
        "1",
        "listThings",
        "page\nregion",
    ))
    .text();
    let written = report(&hostile_bundle(
        "h.json",
        "3.1.0",
        "1",
        "listThings",
        "page\\nregion",
    ))
    .text();
    assert_ne!(newline, written);
}
