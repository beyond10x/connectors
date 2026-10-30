//! story:invoke-walks-declared-pages: `connectors invoke … --walk --out <file|->`
//! follows the walk a read declares, one admitted read per page, writes each
//! record as one NDJSON line and prints a summary
//! (`contracts/cli/v1alpha1/semantics.md`, "Walking pages").
//!
//! The jailed cases run the production CLI, owner and an adapter child against
//! the read-only `mounted_secret` binding (`support/mounted.rs`); Linux only, as
//! that binding is. The adapter child is this test binary serving a Zendesk
//! Support instance whose `ticket.audits` and `ticket.comments` carry the walks
//! the catalog declares for them (`adapters/catalog/providers/zendesk/operations.json`)
//! under `x-connectors-walk`, as the catalog engine publishes them. Each answers
//! two pages: the first without `page[after]`, the second for the cursor the
//! first named. Every operation it is asked for is appended to
//! `/work/invocations` and its input to `/work/inputs`, so a refusal before any
//! provider call is observed as an empty log.
use serde_json::{Value, json};

/// story:invoke-walks-declared-pages: the reusable loop reads pages until an
/// end holds, hands each page's shaped records to the sink, and stops with
/// `resume` at `max_pages`, on a refusal and when the sink refuses a page.
#[test]
fn the_loop_walks_pages_and_resumes_where_it_stopped() {
    use connectors::walk::{self, Page, Stopped, Walk};
    let walk: Walk = serde_json::from_value(json!({"parameter":"page[after]",
        "next":"/meta/after_cursor","records":"/items","query":{"fixed":true},
        "continues":{"pointer":"/meta/has_more","equals":true},
        "ends":[{"name":"has_more_false","when":{"pointer":"/meta/has_more","equals":false}}]}))
    .unwrap();
    let page = |input: &Value| {
        let (items, more) = match input.get("page[after]").and_then(Value::as_str) {
            None => (
                json!([{"id":1,"x":"a"},{"id":2}]),
                json!({"has_more":true,"after_cursor":"c2"}),
            ),
            Some("c2") => (json!([{"id":3}]), json!({"has_more":false})),
            Some(other) => panic!("an unknown cursor {other}"),
        };
        Page::Answered(json!({"status":200,"body":{"items":items,"meta":more},"provenance":{}}))
    };
    let caller = json!({"ticket_id":7});
    let fields = connectors::shape::Fields::parse("id,x").unwrap();
    let mut taken = Vec::new();
    let walked = walk::run(&walk, &caller, 20, Some(&fields), page, |records| {
        taken.extend(records);
        Ok(())
    });
    assert_eq!(
        taken,
        [json!({"id":1,"x":"a"}), json!({"id":2}), json!({"id":3})]
    );
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":2,"records":3,"complete":true,"end":"has_more_false"}),
        "story:invoke-walks-declared-pages: a field some record has is not missing"
    );

    let walked = walk::run(&walk, &caller, 1, None, page, |_| Ok(()));
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":1,"records":2,"complete":false,"stopped":"max_pages",
            "resume":{"ticket_id":7,"fixed":true,"page[after]":"c2"}}),
        "story:invoke-walks-declared-pages"
    );

    let error = json!({"code":"service_failure","data":{"service_code":"rate_limited"}});
    let walked = walk::run(
        &walk,
        &caller,
        20,
        None,
        |input: &Value| match input.get("page[after]") {
            None => page(input),
            Some(_) => Page::Refused(error.clone()),
        },
        |_| Ok(()),
    );
    assert_eq!(walked.stopped, Some(Stopped::Refused(error.clone())));
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":1,"records":2,"complete":false,"stopped":"refused","error":error,
            "resume":{"ticket_id":7,"fixed":true,"page[after]":"c2"}}),
        "story:invoke-walks-declared-pages"
    );

    // A sink that takes no more than one page: the second page is the resume.
    let mut pages = 0;
    let walked = walk::run(&walk, &caller, 20, None, page, |_| {
        pages += 1;
        if pages > 1 { Err("bound") } else { Ok(()) }
    });
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":1,"records":2,"complete":false,"stopped":"bound",
            "resume":{"ticket_id":7,"fixed":true,"page[after]":"c2"}}),
        "story:invoke-walks-declared-pages"
    );

    // A catalog page whose status is not 2xx stops the walk before its records.
    let walked = walk::run(
        &walk,
        &caller,
        20,
        None,
        |_: &Value| Page::Answered(json!({"status":404,"body":{"items":[{"id":1}]}})),
        |_| panic!("no record of a 404 is handed over"),
    );
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":0,"records":0,"complete":false,"stopped":"status","status":404,
            "resume":{"ticket_id":7,"fixed":true}}),
        "story:invoke-walks-declared-pages"
    );
}

/// story:invoke-walks-declared-pages: `--max-pages` is a whole number from 1
/// to 10,000, 20 when it is not given.
#[test]
fn max_pages_is_one_to_ten_thousand() {
    use connectors::walk::max_pages;
    assert_eq!(max_pages(None), Ok(20));
    assert_eq!(max_pages(Some("1")), Ok(1));
    assert_eq!(max_pages(Some("10000")), Ok(10_000));
    for bad in ["0", "10001", "-1", "+3", "1.5", "", "x", "99999999999"] {
        assert!(max_pages(Some(bad)).is_err(), "{bad:?}");
    }
}

#[cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;

#[cfg(target_os = "linux")]
mod jailed {
    use super::{Value, json};
    use crate::mounted::*;
    use connectors_host::local::{registry, runtime};
    use std::{collections::BTreeSet, fs, io::Write, os::unix::fs::PermissionsExt, path::Path};

    /// The adapter child the owner starts filters to this test's exact name.
    const ADAPTER_FIXTURE: &str = "jailed::zendesk_adapter_fixture";
    const SHOW: &str = "ticket.show";
    const AUDITS: &str = "ticket.audits";
    const COMMENTS: &str = "ticket.comments";
    const UPDATE: &str = "ticket.update";
    /// A ticket whose first page the fixture refuses as the provider's own not-found.
    const REFUSED: u64 = 999;
    /// A ticket whose second page the fixture refuses as rate limited.
    const LIMITED: u64 = 429;
    const AUDIT_CURSOR: &str = "fixture-audit-cursor-2";
    const COMMENT_CURSOR: &str = "fixture-comment-cursor-2";

    /// The adapter child the owner starts: this binary, filtered to this test.
    /// The ordinary run does nothing.
    #[test]
    fn zendesk_adapter_fixture() {
        if std::env::args().any(|arg| arg == "--connectors-private-fd") {
            let _ = runtime::serve(3, Zendesk);
        }
    }

    /// The three synthetic audits of a ticket (`adapters/catalog/tests/zendesk.rs`).
    fn audits(ticket: u64) -> Vec<Value> {
        vec![
            json!({"id": 9101, "ticket_id": ticket, "author_id": 201,
                   "created_at": "2025-09-15T08:00:00Z",
                   "events": [{"id": 9201, "type": "Comment", "author_id": 201, "public": true,
                               "body": "fixture comment 1"},
                              {"id": 9202, "type": "Create", "field_name": "status", "value": "new"}]}),
            json!({"id": 9102, "ticket_id": ticket, "author_id": 200,
                   "created_at": "2025-09-15T09:00:00Z",
                   "events": [{"id": 9205, "type": "Change", "field_name": "status",
                               "value": "open", "previous_value": "new"}]}),
            json!({"id": 9103, "ticket_id": ticket, "author_id": 200,
                   "created_at": "2025-09-16T10:00:00Z",
                   "events": [{"id": 9207, "type": "Change", "field_name": "priority",
                               "value": "high", "previous_value": "normal"}]}),
        ]
    }

    fn comments() -> Vec<Value> {
        (1..=3)
            .map(|n| {
                json!({"id": 9300 + n, "author_id": 201, "public": true,
                       "body": format!("fixture comment {n}"),
                       "created_at": "2025-09-15T08:00:00Z"})
            })
            .collect()
    }

    /// A catalog result with a fixed observation time.
    fn catalog(resource: &str, body: Value) -> Value {
        json!({"status": 200, "body": body,
               "provenance": {"instance": "forge-local", "resource": resource,
                              "observed_at_unix_ms": 1_758_000_000_000_u64,
                              "source_revision": "fixture-source-1"}})
    }

    /// The walks the catalog declares for Zendesk's `ticket.comments` and
    /// `ticket.audits`, as the engine carries them in the input schema.
    fn walk(records: &str, query: Value) -> Value {
        json!({"parameter": "page[after]", "next": "/meta/after_cursor", "records": records,
               "query": query,
               "continues": {"pointer": "/meta/has_more", "equals": true},
               "ends": [{"name": "has_more_false", "when": {"pointer": "/meta/has_more", "equals": false}},
                        {"name": "no_cursor", "absent": "/meta/after_cursor"},
                        {"name": "no_records", "empty": records}]})
    }

    fn operation(id: &str, walk: Option<Value>) -> connectors_core::Operation {
        let mut input_schema = json!({"type":"object","additionalProperties":false,
            "properties":{"ticket_id":{"type":"integer"},"ticket":{"type":"object"},
                "page[size]":{"type":"integer"},"page[after]":{"type":"string"},
                "include_boundary_indicators":{"type":"boolean"}},
            "required":["ticket_id"]});
        if let Some(walk) = walk {
            input_schema["x-connectors-walk"] = walk;
        }
        connectors_core::Operation {
            id: id.into(),
            description: format!("{id} of one ticket"),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema,
            output_schema: json!({"type":"object","required":["status","body","provenance"]}),
        }
    }

    fn bootstrap() -> runtime::Bootstrap {
        let descriptor = connectors_core::Descriptor {
            version: "v1alpha1".into(),
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            revision: REVISION.into(),
            operations: vec![
                operation(SHOW, None),
                operation(
                    AUDITS,
                    Some(walk(
                        "/audits",
                        json!({"include_boundary_indicators": true}),
                    )),
                ),
                operation(COMMENTS, Some(walk("/comments", json!({})))),
                operation(UPDATE, None),
            ],
            configuration_schema: json!({"type":"object"}),
        };
        let requirement = |id: &str, effect| runtime::Requirement {
            operation: id.into(),
            custody: false,
            profile: PROFILE.into(),
            scopes: BTreeSet::new(),
            effect,
        };
        runtime::Bootstrap {
            bindings: None,
            credential_revision: None,
            executable: None,
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            protocol: "v1alpha1".into(),
            configuration_revision: "cfg-1".into(),
            provider_authority: "https://fixture.zendesk.invalid".into(),
            descriptor: serde_json::to_string(&descriptor).unwrap(),
            profiles: vec![runtime::Profile {
                id: PROFILE.into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                scheme: "http_bearer".into(),
                capability: "http-bearer".into(),
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                fields: vec![runtime::EntryField {
                    name: "token".into(),
                    label: "Token".into(),
                    max_bytes: 1024,
                }],
                acquisition: None,
            }],
            requirements: vec![
                requirement(SHOW, runtime::Effect::Read),
                requirement(AUDITS, runtime::Effect::Read),
                requirement(COMMENTS, runtime::Effect::Read),
                requirement(UPDATE, runtime::Effect::Write),
            ],
        }
    }

    /// The Zendesk Support stand-in: each invocation appends its operation to
    /// `/work/invocations` and its input to `/work/inputs`.
    struct Zendesk;

    #[async_trait::async_trait]
    impl runtime::Adapter for Zendesk {
        fn bootstrap(&self) -> runtime::Bootstrap {
            bootstrap()
        }
        fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
            Ok(bootstrap())
        }
        async fn validate(
            &self,
            _: &str,
            _: connectors_sdk::Secret,
        ) -> runtime::Result<runtime::Baseline> {
            let now = connectors_sdk::now_ms();
            Ok(runtime::Baseline {
                binding: None,
                identity: registry::ExternalIdentity {
                    kind: "zendesk.user".into(),
                    subject: "200".into(),
                },
                granted_scopes: None,
                credential_expires_at_ms: None,
                collected_at_ms: now,
                valid_until_ms: now + 50_000,
            })
        }
        async fn invoke(
            &self,
            operation: &str,
            _: &str,
            _: connectors_sdk::Secret,
            input: Value,
        ) -> runtime::Result<Value> {
            for (log, line) in [
                ("/work/invocations", operation.to_owned()),
                ("/work/inputs", input.to_string()),
            ] {
                let mut log = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(Path::new(log))
                    .unwrap();
                writeln!(log, "{line}").unwrap();
            }
            let id = input["ticket_id"].as_u64().unwrap_or(0);
            let after = input.get("page[after]").and_then(Value::as_str);
            match (operation, id, after) {
                (_, REFUSED, _) => Err(runtime::Failure::ProviderNotFound),
                (AUDITS, LIMITED, Some(_)) => Err(runtime::Failure::ProviderRateLimited),
                (AUDITS, id, None) => Ok(catalog(
                    "/api/v2/tickets/{ticket_id}/audits",
                    json!({"audits": audits(id)[..2],
                           "meta": {"has_more": true, "after_cursor": AUDIT_CURSOR,
                                    "before_cursor": null}}),
                )),
                (AUDITS, id, Some(AUDIT_CURSOR)) => Ok(catalog(
                    "/api/v2/tickets/{ticket_id}/audits",
                    json!({"audits": audits(id)[2..],
                           "meta": {"has_more": false, "after_cursor": "fixture-audit-cursor-3",
                                    "before_cursor": AUDIT_CURSOR}}),
                )),
                // Without `meta.has_more` the comments walk ends on no cursor.
                (COMMENTS, _, None) => Ok(catalog(
                    "/api/v2/tickets/{ticket_id}/comments",
                    json!({"comments": comments()[..2],
                           "meta": {"after_cursor": COMMENT_CURSOR}}),
                )),
                (COMMENTS, _, Some(COMMENT_CURSOR)) => Ok(catalog(
                    "/api/v2/tickets/{ticket_id}/comments",
                    json!({"comments": comments()[2..], "meta": {"after_cursor": null}}),
                )),
                (SHOW, id, _) => Ok(catalog(
                    "/api/v2/tickets/{ticket_id}",
                    json!({"ticket": {"id": id, "status": "open"}}),
                )),
                _ => Err(runtime::Failure::Unsupported),
            }
        }
    }

    /// The fixture configured as instance `zendesk/support` (alias `forge`)
    /// of a `connectors-local/5` file, custody the mounted directory.
    fn configure(container: &Container) {
        let uid = connectors_host::local::filesystem::uid();
        let adapter = digest(&fs::read(container.path("/bin-test/adapter")).unwrap());
        let args = serde_json::to_string(&["--exact", ADAPTER_FIXTURE, "--"]).unwrap();
        let table = "connectors.zendesk.instances.support";
        let text = format!(
            "format='connectors-local/5'\nowner_uid={uid}\n{}\n[connectors.zendesk]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.permissions]\nprofiles=['{PROFILE}']\noperations=['{SHOW}','{AUDITS}','{COMMENTS}','{UPDATE}']\n[{table}.executable]\npath='/bin-test/adapter'\nsha256='{adapter}'\nargs={args}\n",
            Container::mounted()
        );
        let path = container.path(CONFIG);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// A jail whose instance holds one connection, made by a connect.
    fn connected() -> Container {
        bootstrap()
            .validate_for(runtime::PrivateProtocol::V2)
            .expect("the fixture's description passes the child's own readiness check");
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        configure(&container);
        success(&container.run(&["setup", "init"]));
        success(&container.connect(PROFILE));
        container
    }

    /// `invoke zendesk/support <operation>` with `input` and `extra` arguments.
    fn invoke(
        container: &Container,
        operation: &str,
        input: &Value,
        extra: &[&str],
    ) -> std::process::Output {
        let input = input.to_string();
        let mut args = vec![
            "invoke",
            "zendesk/support",
            operation,
            "--input-json",
            &input,
        ];
        args.extend(extra);
        container.run(&args)
    }

    /// The JSON lines of a file in the jail.
    fn lines_of(container: &Container, inside: &str) -> Vec<Value> {
        fs::read_to_string(container.path(inside))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// The inputs the provider was invoked with, in order.
    fn inputs(container: &Container) -> Vec<Value> {
        container
            .seen("inputs")
            .iter()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[track_caller]
    fn refused_naming(output: &std::process::Output, code: &str, field: &str) {
        let (exit, data) = refusal(output);
        assert_eq!(
            (&data["code"], &data["field"]),
            (&json!(code), &json!(field)),
            "story:invoke-walks-declared-pages: {data}"
        );
        if code == "invalid_input" {
            assert_eq!(
                (exit, &data["stage"]),
                (Some(2), &json!("arguments")),
                "{data}"
            );
        }
        assert!(output.stdout.is_empty(), "{output:?}");
    }

    /// story:invoke-walks-declared-pages: a two-page audits walk to a file
    /// writes each audit as one line of a new owner-only file, sends the
    /// declared `query` on every page and the cursor from the second, and
    /// prints the summary naming the end that held.
    #[test]
    fn a_two_page_walk_writes_ndjson_to_a_new_owner_only_file() {
        let container = connected();
        let input = json!({"ticket_id": 101, "page[size]": 2});
        let output = invoke(
            &container,
            AUDITS,
            &input,
            &["--walk", "--out", "/work/audits.ndjson"],
        );
        let text = String::from_utf8(output.stdout.clone()).unwrap();
        assert_eq!(text.lines().count(), 1, "the summary alone: {text}");
        assert_eq!(
            success(&output),
            json!({"pages":2,"records":3,"end":"has_more_false","complete":true,
                "out":"/work/audits.ndjson"}),
            "story:invoke-walks-declared-pages"
        );
        assert_eq!(lines_of(&container, "/work/audits.ndjson"), audits(101));
        let mode = fs::metadata(container.path("/work/audits.ndjson"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "story:invoke-walks-declared-pages: owner-only"
        );
        assert_eq!(container.seen("invocations"), [AUDITS, AUDITS]);
        assert_eq!(
            inputs(&container),
            [
                json!({"ticket_id":101,"page[size]":2,"include_boundary_indicators":true}),
                json!({"ticket_id":101,"page[size]":2,"include_boundary_indicators":true,
                    "page[after]":AUDIT_CURSOR}),
            ],
            "story:invoke-walks-declared-pages: one admitted read per page"
        );
    }

    /// story:invoke-walks-declared-pages: `--out -` streams the records on
    /// standard output, the summary envelope last; the comments walk ends on
    /// the page without a cursor.
    #[test]
    fn out_dash_streams_the_records_then_the_summary() {
        let container = connected();
        let output = invoke(
            &container,
            COMMENTS,
            &json!({"ticket_id": 101}),
            &["--walk", "--out", "-"],
        );
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let lines: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let mut expected = comments();
        expected.push(
            json!({"ok":true,"result":{"pages":2,"records":3,"end":"no_cursor",
            "complete":true,"out":"-"}}),
        );
        assert_eq!(lines, expected, "story:invoke-walks-declared-pages");
    }

    /// story:invoke-walks-declared-pages: `--max-pages 1` stops after one page
    /// with `complete: false` and `resume`, the input of the next page; a walk
    /// given `resume` as its input reads on from there to the end.
    #[test]
    fn max_pages_stops_the_walk_with_resume() {
        let container = connected();
        let output = invoke(
            &container,
            AUDITS,
            &json!({"ticket_id": 101, "page[size]": 2}),
            &["--walk", "--max-pages", "1", "--out", "/work/first.ndjson"],
        );
        let resume = json!({"ticket_id":101,"page[size]":2,"include_boundary_indicators":true,
            "page[after]":AUDIT_CURSOR});
        assert_eq!(
            success(&output),
            json!({"pages":1,"records":2,"complete":false,"stopped":"max_pages",
                "resume":resume,"out":"/work/first.ndjson"}),
            "story:invoke-walks-declared-pages"
        );
        assert_eq!(lines_of(&container, "/work/first.ndjson"), audits(101)[..2]);
        assert_eq!(container.seen("invocations"), [AUDITS]);
        let rest = invoke(
            &container,
            AUDITS,
            &resume,
            &["--walk", "--out", "/work/rest.ndjson"],
        );
        assert_eq!(
            success(&rest),
            json!({"pages":1,"records":1,"end":"has_more_false","complete":true,
                "out":"/work/rest.ndjson"}),
            "story:invoke-walks-declared-pages"
        );
        assert_eq!(lines_of(&container, "/work/rest.ndjson"), audits(101)[2..]);
    }

    /// story:invoke-walks-declared-pages: an existing `--out` file is refused
    /// as `invalid_input` naming `out` before any call, and left as it was;
    /// so is `--walk` without `--out`.
    #[test]
    fn an_existing_out_file_and_a_missing_out_are_refused() {
        let container = connected();
        let existing = container.path("/work/existing.ndjson");
        fs::write(&existing, "kept\n").unwrap();
        let input = json!({"ticket_id": 101});
        refused_naming(
            &invoke(
                &container,
                AUDITS,
                &input,
                &["--walk", "--out", "/work/existing.ndjson"],
            ),
            "invalid_input",
            "out",
        );
        assert_eq!(fs::read_to_string(&existing).unwrap(), "kept\n");
        refused_naming(
            &invoke(&container, AUDITS, &input, &["--walk"]),
            "invalid_input",
            "out",
        );
        assert!(container.seen("invocations").is_empty());
    }

    /// story:invoke-walks-declared-pages: an operation that declares no walk
    /// is refused as `unsupported` naming `walk`, and a write as
    /// `invalid_input` naming `walk` ahead of `approval_required`, both before
    /// any call and without creating the `--out` file.
    #[test]
    fn an_undeclared_operation_and_a_write_are_refused() {
        let container = connected();
        let input = json!({"ticket_id": 101});
        let out = ["--walk", "--out", "/work/never.ndjson"];
        refused_naming(
            &invoke(&container, SHOW, &input, &out),
            "unsupported",
            "walk",
        );
        refused_naming(
            &invoke(&container, UPDATE, &input, &out),
            "invalid_input",
            "walk",
        );
        assert!(container.seen("invocations").is_empty());
        assert!(!container.path("/work/never.ndjson").exists());
    }

    /// story:invoke-walks-declared-pages: the walk's own arguments are checked
    /// before anything is read: `--max-pages` outside 1 to 10,000, `--each`
    /// beside `--walk`, `--walk` beside `--describe`, and `--max-pages` or
    /// `--out` without `--walk`, each `invalid_input` naming its argument.
    #[test]
    fn the_walk_arguments_are_checked_before_any_call() {
        let container = connected();
        let input = json!({"ticket_id": 101});
        for (extra, field) in [
            (
                &["--walk", "--out", "-", "--max-pages", "0"][..],
                "max_pages",
            ),
            (
                &["--walk", "--out", "-", "--max-pages", "10001"][..],
                "max_pages",
            ),
            (
                &["--walk", "--out", "-", "--max-pages", "x"][..],
                "max_pages",
            ),
            (&["--walk", "--out", "-", "--each", "audits"][..], "each"),
            (&["--walk", "--out", "-", "--describe"][..], "walk"),
            (&["--max-pages", "2"][..], "max_pages"),
            (&["--out", "-"][..], "out"),
        ] {
            refused_naming(
                &invoke(&container, AUDITS, &input, extra),
                "invalid_input",
                field,
            );
        }
        assert!(container.seen("invocations").is_empty());
    }

    /// story:invoke-walks-declared-pages: a refusal after the first page stops
    /// the walk: the records written stay, the summary holds the refusal and
    /// `resume`, the input of the refused page, and the exit status is 0.
    #[test]
    fn a_mid_walk_refusal_keeps_the_records_written() {
        let container = connected();
        let output = invoke(
            &container,
            AUDITS,
            &json!({"ticket_id": LIMITED}),
            &["--walk", "--out", "/work/limited.ndjson"],
        );
        let summary = success(&output);
        assert_eq!(
            (
                &summary["pages"],
                &summary["records"],
                &summary["complete"],
                &summary["stopped"]
            ),
            (&json!(1), &json!(2), &json!(false), &json!("refused")),
            "story:invoke-walks-declared-pages: {summary}"
        );
        assert_eq!(
            summary["error"]["data"]["service_code"], "rate_limited",
            "{summary}"
        );
        assert_eq!(
            summary["resume"],
            json!({"ticket_id":LIMITED,"include_boundary_indicators":true,
                "page[after]":AUDIT_CURSOR}),
            "{summary}"
        );
        assert!(summary.get("end").is_none(), "{summary}");
        assert_eq!(
            lines_of(&container, "/work/limited.ndjson"),
            audits(LIMITED)[..2]
        );
        assert_eq!(container.seen("invocations"), [AUDITS, AUDITS]);
    }

    /// story:invoke-walks-declared-pages: a refused first page is the failure
    /// envelope a plain invoke answers, and the walk leaves no `--out` file.
    #[test]
    fn a_refused_first_page_is_the_plain_failure() {
        let container = connected();
        let input = json!({"ticket_id": REFUSED});
        let plain = invoke(&container, AUDITS, &input, &[]);
        let walked = invoke(
            &container,
            AUDITS,
            &input,
            &["--walk", "--out", "/work/refused.ndjson"],
        );
        assert_eq!(walked.status.code(), plain.status.code(), "{walked:?}");
        assert!(walked.stdout.is_empty(), "{walked:?}");
        assert_eq!(
            String::from_utf8_lossy(&walked.stderr),
            String::from_utf8_lossy(&plain.stderr),
            "story:invoke-walks-declared-pages"
        );
        assert!(!container.path("/work/refused.ndjson").exists());
    }

    /// story:invoke-walks-declared-pages: `--fields` shapes each record, and a
    /// path no record has is listed in the summary's `missing`.
    #[test]
    fn fields_shape_each_record() {
        let container = connected();
        let output = invoke(
            &container,
            AUDITS,
            &json!({"ticket_id": 101}),
            &[
                "--walk",
                "--out",
                "-",
                "--fields",
                "id,author:author_id,events[].field_name,due_at",
            ],
        );
        let lines: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            lines,
            [
                json!({"id":9101,"author":201,"events":[{"field_name":"status"}]}),
                json!({"id":9102,"author":200,"events":[{"field_name":"status"}]}),
                json!({"id":9103,"author":200,"events":[{"field_name":"priority"}]}),
                json!({"ok":true,"result":{"pages":2,"records":3,"end":"has_more_false",
                    "complete":true,"out":"-","missing":["due_at"]}}),
            ],
            "story:invoke-walks-declared-pages"
        );
    }
}
