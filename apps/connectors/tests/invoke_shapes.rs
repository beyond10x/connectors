//! story:invoke-shapes-its-result: `connectors invoke … --fields <paths>` prints only
//! the named members of the provider result, and `--each <path>` one NDJSON line
//! per element of the array at `path`, then the envelope
//! (`contracts/cli/v1alpha1/semantics.md`, "Shaping a result").
//!
//! The jailed cases run the production CLI, owner and an adapter child against
//! the read-only `mounted_secret` binding (`support/mounted.rs`); Linux only, as
//! that binding is. The adapter child is this test binary serving a Zendesk
//! Support instance: catalog results (`{status, body, provenance}`) whose bodies
//! are the synthetic tickets and audits of the catalog's Zendesk fixture
//! (`adapters/catalog/tests/zendesk.rs`), with a fixed `observed_at_unix_ms` so
//! two answers compare byte for byte. Every operation it is asked for is
//! appended to `/work/invocations`, so a refusal before any provider call is
//! observed as an empty log.
//!
//! A write's answer is shaped by `connectors::shape::answer`, the function the
//! CLI prints through; no write reaches a provider in these cases, because the
//! mounted binding holds no approval key.
#[cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use serde_json::{Value, json};

/// story:invoke-shapes-its-result: on a write, `--fields` narrows only the
/// provider result; `request_id`, `mutation` and `source_audit` are printed
/// whatever the fields name, so `outcome_unknown` cannot be projected away.
#[test]
fn a_write_keeps_request_id_mutation_and_source_audit() {
    let answer = json!({"connector":"zendesk","instance":"support","operation":"ticket.update",
        "revision":"desc-1",
        "result":{"status":200,"body":{"ticket":{"id":101,"status":"solved","subject":"s"}},
            "provenance":{"instance":"forge-local","resource":"/api/v2/tickets/{ticket_id}",
                "observed_at_unix_ms":1,"source_revision":null}},
        "request_id":"req-0001",
        "mutation":{"classification":"outcome_unknown"},
        "source_audit":{"id":"audit-0001"}});
    let fields = connectors::shape::Fields::parse("ticket.status").unwrap();
    let shaped = connectors::shape::answer(&answer, Some(&fields), None)
        .expect("story:invoke-shapes-its-result: a 2xx write is shaped");
    assert!(shaped.records.is_empty());
    assert_eq!(
        shaped.result,
        json!({"ticket":{"status":"solved"},"request_id":"req-0001",
            "mutation":{"classification":"outcome_unknown"},"source_audit":{"id":"audit-0001"}}),
        "story:invoke-shapes-its-result"
    );
    // A field may not take one of their names, so none can be replaced.
    for reserved in ["mutation", "x:a,request_id", "source_audit:a", "missing"] {
        assert!(
            connectors::shape::Fields::parse(reserved).is_err(),
            "story:invoke-shapes-its-result: {reserved:?}"
        );
    }
    let _: Value = shaped.result;
}

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
    const UPDATE: &str = "ticket.update";
    /// A ticket the fixture answers 404 for, as Zendesk answers an unknown id.
    const UNKNOWN: u64 = 404;
    /// A ticket the fixture refuses as the provider's own not-found failure.
    const REFUSED: u64 = 999;

    /// The adapter child the owner starts: this binary, filtered to this test.
    /// The ordinary run does nothing.
    #[test]
    fn zendesk_adapter_fixture() {
        if std::env::args().any(|arg| arg == "--connectors-private-fd") {
            let _ = runtime::serve(3, Zendesk);
        }
    }

    fn ticket(id: u64) -> Value {
        json!({"id": id, "url": format!("https://fixture.example.com/api/v2/tickets/{id}.json"),
               "subject": format!("fixture ticket {id}"), "status": "open",
               "requester_id": 201, "submitter_id": 201, "assignee_id": 200,
               "organization_id": 301, "created_at": "2025-09-15T08:00:00Z",
               "updated_at": "2025-09-16T10:00:00Z"})
    }

    /// The three synthetic audits of ticket 101 (`adapters/catalog/tests/zendesk.rs`).
    fn audits() -> Value {
        json!([
            {"id": 9101, "ticket_id": 101, "author_id": 201, "created_at": "2025-09-15T08:00:00Z",
             "via": {"channel": "web", "source": {"from": {}, "to": {}, "rel": null}},
             "events": [
                {"id": 9201, "type": "Comment", "author_id": 201, "public": true,
                 "body": "fixture comment 1", "attachments": []},
                {"id": 9202, "type": "Create", "field_name": "status", "value": "new"},
                {"id": 9203, "type": "Create", "field_name": "priority", "value": "normal"}]},
            {"id": 9102, "ticket_id": 101, "author_id": 200, "created_at": "2025-09-15T09:00:00Z",
             "via": {"channel": "rule", "source": {"from": {"id": 5001, "title": "fixture trigger"},
                                                   "to": {}, "rel": "trigger"}},
             "events": [
                {"id": 9204, "type": "Comment", "author_id": 200, "public": false,
                 "body": "fixture comment 2", "attachments": []},
                {"id": 9205, "type": "Change", "field_name": "status",
                 "value": "open", "previous_value": "new"},
                {"id": 9206, "type": "Change", "field_name": "tags",
                 "value": ["fixture", "triaged"], "previous_value": ["fixture"]}]},
            {"id": 9103, "ticket_id": 101, "author_id": 200, "created_at": "2025-09-16T10:00:00Z",
             "via": {"channel": "api", "source": {"from": {}, "to": {}, "rel": null}},
             "events": [
                {"id": 9207, "type": "Change", "field_name": "priority",
                 "value": "high", "previous_value": "normal"}]}
        ])
    }

    /// A catalog result with a fixed observation time.
    fn catalog(status: u16, resource: &str, body: Value) -> Value {
        json!({"status": status, "body": body,
               "provenance": {"instance": "forge-local", "resource": resource,
                              "observed_at_unix_ms": 1_758_000_000_000_u64,
                              "source_revision": "fixture-source-1"}})
    }

    fn operation(id: &str, description: &str) -> connectors_core::Operation {
        connectors_core::Operation {
            id: id.into(),
            description: description.into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"ticket_id":{"type":"integer"},"ticket":{"type":"object"}},
                "required":["ticket_id"]}),
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
                operation(SHOW, "Show one ticket"),
                operation(AUDITS, "List one ticket's audits"),
                operation(UPDATE, "Update one ticket"),
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
                requirement(UPDATE, runtime::Effect::Write),
            ],
        }
    }

    /// The Zendesk Support stand-in: each invocation appends its operation to
    /// `/work/invocations`.
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
            let mut log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(Path::new("/work/invocations"))
                .unwrap();
            writeln!(log, "{operation}").unwrap();
            let id = input["ticket_id"].as_u64().unwrap_or(0);
            match (operation, id) {
                (_, REFUSED) => Err(runtime::Failure::ProviderNotFound),
                (SHOW, UNKNOWN) => Ok(catalog(
                    404,
                    "/api/v2/tickets/{ticket_id}",
                    json!({"error":"RecordNotFound","description":"Not found"}),
                )),
                (SHOW, id) => Ok(catalog(
                    200,
                    "/api/v2/tickets/{ticket_id}",
                    json!({"ticket": ticket(id)}),
                )),
                // `page[after]` is a synthetic member whose name holds `[`,
                // as the cursor parameter's name does.
                (AUDITS, _) => Ok(catalog(
                    200,
                    "/api/v2/tickets/{ticket_id}/audits",
                    json!({"audits": audits(),
                           "meta": {"has_more": false, "after_cursor": "fixture-audit-cursor-2",
                                    "before_cursor": null},
                           "page[after]": "fixture-audit-cursor-2"}),
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
            "format='connectors-local/5'\nowner_uid={uid}\n{}\n[connectors.zendesk]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.permissions]\nprofiles=['{PROFILE}']\noperations=['{SHOW}','{AUDITS}','{UPDATE}']\n[{table}.executable]\npath='/bin-test/adapter'\nsha256='{adapter}'\nargs={args}\n",
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

    /// `invoke zendesk/support <operation>` on ticket `id` with `extra` arguments.
    fn invoke(
        container: &Container,
        operation: &str,
        id: u64,
        extra: &[&str],
    ) -> std::process::Output {
        let input = json!({"ticket_id": id}).to_string();
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

    /// The one success envelope standard output holds.
    #[track_caller]
    fn only_envelope(output: &std::process::Output) -> Value {
        let text = String::from_utf8(output.stdout.clone()).unwrap();
        assert_eq!(text.lines().count(), 1, "one envelope: {text}");
        success(output)
    }

    /// story:invoke-shapes-its-result: `--fields ticket.id,state:ticket.status`
    /// prints `{"ok":true,"result":{"ticket":{"id":101},"state":"open"}}`: the
    /// leading `body.` left out, the alias renaming, and nothing else of the
    /// answer printed. The provider is called once.
    #[test]
    fn fields_and_an_alias_print_only_the_named_members_of_a_read() {
        let container = connected();
        let output = invoke(
            &container,
            SHOW,
            101,
            &["--fields", "ticket.id,state:ticket.status"],
        );
        assert_eq!(
            only_envelope(&output),
            json!({"ticket":{"id":101},"state":"open"}),
            "story:invoke-shapes-its-result"
        );
        assert_eq!(container.seen("invocations"), [SHOW]);
        let full = only_envelope(&invoke(&container, SHOW, 101, &["--fields", "body.ticket"]));
        assert_eq!(
            full,
            json!({"body":{"ticket":ticket(101)}}),
            "`body.` may be written"
        );
    }

    /// story:invoke-shapes-its-result: `[]` maps over an array; paths sharing
    /// a prefix print one member, and an event without `field_name` is left out.
    #[test]
    fn each_step_maps_over_an_array() {
        let container = connected();
        let output = invoke(
            &container,
            AUDITS,
            101,
            &["--fields", "audits[].id,audits[].events[].field_name"],
        );
        assert_eq!(
            only_envelope(&output),
            json!({"audits":[
                {"id":9101,"events":[{"field_name":"status"},{"field_name":"priority"}]},
                {"id":9102,"events":[{"field_name":"status"},{"field_name":"tags"}]},
                {"id":9103,"events":[{"field_name":"priority"}]}]}),
            "story:invoke-shapes-its-result"
        );
    }

    /// story:invoke-shapes-its-result: a key holding `[` is written as an
    /// RFC 6901 pointer.
    #[test]
    fn a_pointer_names_a_key_holding_a_bracket() {
        let container = connected();
        let output = invoke(
            &container,
            AUDITS,
            101,
            &[
                "--fields",
                "/page[after],more:meta.has_more,cursor:/meta/after_cursor",
            ],
        );
        assert_eq!(
            only_envelope(&output),
            json!({"page[after]":"fixture-audit-cursor-2","more":false,
                "cursor":"fixture-audit-cursor-2"}),
            "story:invoke-shapes-its-result"
        );
    }

    /// story:invoke-shapes-its-result: a path that matches nothing is listed in
    /// `result.missing`, as written and in the order given.
    #[test]
    fn a_path_that_matches_nothing_is_listed_in_missing() {
        let container = connected();
        let output = invoke(
            &container,
            SHOW,
            101,
            &[
                "--fields",
                "ticket.id,ticket.due_at,channel:ticket.via.channel",
            ],
        );
        assert_eq!(
            only_envelope(&output),
            json!({"ticket":{"id":101},"missing":["ticket.due_at","channel:ticket.via.channel"]}),
            "story:invoke-shapes-its-result"
        );
    }

    /// story:invoke-shapes-its-result: `--each audits` prints one NDJSON line
    /// per audit, each shaped by `--fields`, then the envelope as the last line.
    #[test]
    fn each_prints_one_ndjson_line_per_element_then_the_envelope() {
        let container = connected();
        let output = invoke(
            &container,
            AUDITS,
            101,
            &[
                "--each",
                "audits",
                "--fields",
                "id,author_id,events[].field_name",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        let lines: Vec<Value> = String::from_utf8(output.stdout.clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            lines,
            [
                json!({"id":9101,"author_id":201,
                    "events":[{"field_name":"status"},{"field_name":"priority"}]}),
                json!({"id":9102,"author_id":200,
                    "events":[{"field_name":"status"},{"field_name":"tags"}]}),
                json!({"id":9103,"author_id":200,"events":[{"field_name":"priority"}]}),
                json!({"ok":true,"result":{"records":3}}),
            ],
            "story:invoke-shapes-its-result"
        );
        assert!(output.stderr.is_empty(), "{output:?}");
        let whole = invoke(&container, AUDITS, 101, &["--each", "audits"]);
        let text = String::from_utf8(whole.stdout).unwrap();
        let first: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(
            first,
            audits()[0],
            "without --fields each element is printed whole"
        );
        assert_eq!(text.lines().count(), 4);
    }

    /// story:invoke-shapes-its-result: `--fields status` is the status-only
    /// probe.
    #[test]
    fn fields_status_is_the_status_only_probe() {
        let container = connected();
        let output = invoke(&container, SHOW, 101, &["--fields", "status"]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "{\"ok\":true,\"result\":{\"status\":200}}\n",
            "story:invoke-shapes-its-result"
        );
    }

    /// story:invoke-shapes-its-result: a catalog result whose status is not
    /// 2xx is printed whole, byte for byte the answer without `--fields`, and
    /// so is `--each`.
    #[test]
    fn a_non_2xx_catalog_result_is_printed_whole() {
        let container = connected();
        let whole = invoke(&container, SHOW, UNKNOWN, &[]);
        assert_eq!(success(&whole)["result"]["status"], 404, "{whole:?}");
        for extra in [&["--fields", "ticket.id"][..], &["--each", "ticket"][..]] {
            let shaped = invoke(&container, SHOW, UNKNOWN, extra);
            assert!(shaped.status.success(), "{shaped:?}");
            assert_eq!(
                String::from_utf8_lossy(&shaped.stdout),
                String::from_utf8_lossy(&whole.stdout),
                "story:invoke-shapes-its-result: {extra:?}"
            );
        }
    }

    /// story:invoke-shapes-its-result: a refusal is never shaped: the failure
    /// envelope on standard error is the one the same call without `--fields`
    /// answers, and standard output stays empty.
    #[test]
    fn a_refusal_is_not_shaped() {
        let container = connected();
        let whole = invoke(&container, SHOW, REFUSED, &[]);
        let (code, data) = refusal(&whole);
        assert_eq!(code, Some(1), "{data}");
        for extra in [&["--fields", "status"][..], &["--each", "audits"][..]] {
            let shaped = invoke(&container, SHOW, REFUSED, extra);
            assert_eq!(shaped.status.code(), whole.status.code(), "{shaped:?}");
            assert!(shaped.stdout.is_empty(), "{shaped:?}");
            assert_eq!(
                String::from_utf8_lossy(&shaped.stderr),
                String::from_utf8_lossy(&whole.stderr),
                "story:invoke-shapes-its-result: {extra:?}"
            );
        }
        assert_eq!(container.seen("invocations"), [SHOW, SHOW, SHOW]);
    }

    /// story:invoke-shapes-its-result: `--each` is refused on a write as
    /// `invalid_input` naming `each`, before the approval check and any
    /// provider call; `--fields` on a write is not refused (the write without
    /// its proof is `approval_required`).
    #[test]
    fn each_is_refused_on_a_write() {
        let container = connected();
        let (code, data) = refusal(&invoke(&container, UPDATE, 101, &["--each", "ticket"]));
        assert_eq!(
            (code, &data["code"], &data["stage"], &data["field"]),
            (
                Some(2),
                &json!("invalid_input"),
                &json!("arguments"),
                &json!("each")
            ),
            "story:invoke-shapes-its-result: {data}"
        );
        let (_, data) = refusal(&invoke(&container, UPDATE, 101, &["--fields", "ticket.id"]));
        assert_eq!(data["code"], "approval_required", "{data}");
        assert!(container.seen("invocations").is_empty());
    }

    /// story:invoke-shapes-its-result: a malformed path is refused before any
    /// provider call as `invalid_input` naming `fields` or `each`; so is either
    /// beside `--describe`, which reads no result.
    #[test]
    fn a_malformed_path_is_refused_before_any_call() {
        let container = connected();
        for (flag, paths, field) in [
            ("--fields", "ticket..id", "fields"),
            ("--fields", "ticket.id,", "fields"),
            ("--fields", "x:a,x:b", "fields"),
            ("--fields", "missing", "fields"),
            ("--fields", "/a~2", "fields"),
            ("--each", "audits[]", "each"),
            ("--each", "a:audits", "each"),
            ("--each", "audits,meta", "each"),
        ] {
            let (code, data) = refusal(&invoke(&container, AUDITS, 101, &[flag, paths]));
            assert_eq!(
                (code, &data["code"], &data["stage"], &data["field"]),
                (
                    Some(2),
                    &json!("invalid_input"),
                    &json!("arguments"),
                    &json!(field)
                ),
                "story:invoke-shapes-its-result: {paths:?}: {data}"
            );
        }
        for (flag, field) in [("--fields", "fields"), ("--each", "each")] {
            let (code, data) = refusal(&container.run(&[
                "invoke",
                "zendesk",
                AUDITS,
                "--describe",
                flag,
                "audits",
            ]));
            assert_eq!(
                (code, &data["code"], &data["field"]),
                (Some(2), &json!("invalid_input"), &json!(field)),
                "story:invoke-shapes-its-result: {data}"
            );
        }
        assert!(container.seen("invocations").is_empty());
    }
}
